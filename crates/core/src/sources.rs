use crate::{AppPaths, Db, Error, Profile, Prospect, Result, Settings, invalid, jobs::JobControl};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
#[derive(Clone)]
pub struct GovernmentClient {
    client: Client,
}
impl GovernmentClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(45))
                .user_agent("ProspectIntelligence/0.1 personal use")
                .build()
                .map_err(|_| invalid("No se pudo preparar el cliente de datos"))?,
        })
    }
    async fn json(
        &self,
        url: String,
        banxico: Option<&str>,
        control: &JobControl,
    ) -> Result<Value> {
        for attempt in 0..3 {
            control.check()?;
            control.update(|s| s.requests += 1);
            let mut request = self.client.get(&url);
            if let Some(token) = banxico {
                let mut header = reqwest::header::HeaderValue::from_str(token)
                    .map_err(|_| invalid("Token Banxico inválido"))?;
                header.set_sensitive(true);
                request = request.header("Bmx-Token", header);
            }
            let response = tokio::select! {_=control.token.cancelled()=>return Err(Error::Cancelled),r=request.send()=>match r{Ok(r)=>r,Err(_)=>{if attempt==2{return Err(invalid("No se pudo conectar con la fuente oficial"));}control.wait(Duration::from_secs(2*(attempt+1))).await?;continue;}}};
            let status = response.status();
            if status.as_u16() == 404 || status.as_u16() == 204 {
                return Ok(Value::Array(vec![]));
            }
            if status.as_u16() == 429 || status.is_server_error() {
                if attempt == 2 {
                    return Err(invalid(format!(
                        "Fuente temporalmente no disponible (HTTP {})",
                        status.as_u16()
                    )));
                }
                control.wait(Duration::from_secs(2 * (attempt + 1))).await?;
                continue;
            }
            if !status.is_success() {
                return Err(invalid(format!(
                    "La fuente rechazó la solicitud (HTTP {}); verifica token y permisos",
                    status.as_u16()
                )));
            }
            if response.content_length().is_some_and(|n| n > 20_000_000) {
                return Err(invalid("Respuesta oficial demasiado grande"));
            }
            let mut bytes = Vec::new();
            let mut stream = response.bytes_stream();
            use futures_util::StreamExt;
            while let Some(chunk) = tokio::select! {_=control.token.cancelled()=>return Err(Error::Cancelled),r=stream.next()=>r}
            {
                let chunk = chunk.map_err(|_| invalid("La fuente interrumpió su respuesta"))?;
                if bytes.len() + chunk.len() > 20_000_000 {
                    return Err(invalid("Respuesta oficial demasiado grande"));
                }
                bytes.extend_from_slice(&chunk);
            }
            return serde_json::from_slice(&bytes)
                .map_err(|_| invalid("La fuente no devolvió JSON válido"));
        }
        Err(invalid("La fuente agotó sus reintentos"))
    }
    // Mirrors the official DENUE geographic/sector pagination parameters.
    #[allow(clippy::too_many_arguments)]
    pub async fn denue_page(
        &self,
        token: &str,
        entity: &str,
        sector: &str,
        subsector: &str,
        stratum: &str,
        start: usize,
        control: &JobControl,
    ) -> Result<Vec<Value>> {
        if token.is_empty() {
            return Err(invalid("Falta DENUE_TOKEN en los secretos locales"));
        }
        if !entity.chars().all(|c| c.is_ascii_digit())
            || !sector.chars().all(|c| c.is_ascii_digit())
            || !subsector.chars().all(|c| c.is_ascii_digit())
            || !["1", "2", "3", "4", "5", "6", "7"].contains(&stratum)
        {
            return Err(invalid("Consulta DENUE inválida"));
        }
        let end = start + 199;
        let url = format!(
            "https://www.inegi.org.mx/app/api/denue/v1/consulta/BuscarAreaActEstr/{entity}/0/0/0/0/{sector}/{subsector}/0/0/0/{start}/{end}/0/{stratum}/{token}"
        );
        let value = self.json(url, None, control).await?;
        value
            .as_array()
            .cloned()
            .ok_or_else(|| invalid("DENUE devolvió un esquema inesperado"))
    }
    pub async fn quantify(
        &self,
        token: &str,
        entity: &str,
        sector: &str,
        stratum: &str,
        control: &JobControl,
    ) -> Result<u64> {
        if token.is_empty() {
            return Err(invalid("Falta DENUE_TOKEN"));
        }
        if [entity, sector, stratum]
            .iter()
            .any(|s| s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()))
        {
            return Err(invalid("Consulta inválida"));
        }
        let value=self.json(format!("https://www.inegi.org.mx/app/api/denue/v1/consulta/Cuantificar/{sector}/{entity}/{stratum}/{token}"),None,control).await?;
        let arr = value
            .as_array()
            .ok_or_else(|| invalid("Respuesta de cuantificación inesperada"))?;
        Ok(arr
            .iter()
            .filter_map(|v| {
                v["Total"]
                    .as_u64()
                    .or_else(|| v["Total"].as_str().and_then(|s| s.parse().ok()))
            })
            .sum())
    }
    pub async fn banxico(&self, token: &str, control: &JobControl) -> Result<Vec<EconomicPoint>> {
        if token.is_empty() {
            return Err(invalid("Falta BANXICO_TOKEN en los secretos locales"));
        }
        let value=self.json("https://www.banxico.org.mx/SieAPIRest/service/v1/series/SF43718,SF61745/datos/oportuno".into(),Some(token),control).await?;
        let series = value["bmx"]["series"]
            .as_array()
            .ok_or_else(|| invalid("Banxico devolvió un esquema inesperado"))?;
        let mut out = Vec::new();
        for s in series {
            let id = s["idSerie"]
                .as_str()
                .ok_or_else(|| invalid("Serie Banxico sin ID"))?;
            let desc = s["titulo"].as_str().unwrap_or("");
            let data = s["datos"]
                .as_array()
                .ok_or_else(|| invalid("Serie Banxico sin observaciones"))?;
            for p in data {
                let period = p["fecha"]
                    .as_str()
                    .ok_or_else(|| invalid("Periodo Banxico inválido"))?;
                let value = p["dato"].as_str().unwrap_or("N/E");
                out.push(EconomicPoint {
                    source: "Banxico SIE".into(),
                    series_id: id.into(),
                    period: period.into(),
                    value: value.into(),
                    unit: if id == "SF43718" {
                        "MXN por USD"
                    } else {
                        "% anual"
                    }
                    .into(),
                    description: desc.into(),
                    updated_at: crate::config::now(),
                });
            }
        }
        Ok(out)
    }
    pub async fn inegi(
        &self,
        token: &str,
        indicator: &str,
        control: &JobControl,
    ) -> Result<Vec<EconomicPoint>> {
        if token.is_empty() {
            return Err(invalid("Falta INEGI_TOKEN (o DENUE_TOKEN compartido)"));
        }
        let (bank, indicator) = indicator.split_once(':').unwrap_or(("BISE", indicator));
        if !["BIE", "BISE"].contains(&bank) {
            return Err(invalid("Banco INEGI inválido"));
        }
        if indicator.is_empty() || !indicator.chars().all(|c| c.is_ascii_digit()) {
            return Err(invalid("ID de indicador inválido"));
        }
        let url = format!(
            "https://www.inegi.org.mx/app/api/indicadores/desarrolladores/jsonxml/INDICATOR/{indicator}/es/00/true/{bank}/2.0/{token}?type=json"
        );
        let value = self.json(url, None, control).await?;
        let series = value["Series"].as_array().ok_or_else(|| {
            invalid("INEGI rechazó el indicador o el token compartido; no hay Series")
        })?;
        let mut out = Vec::new();
        for s in series {
            let data = s["OBSERVATIONS"]
                .as_array()
                .ok_or_else(|| invalid("Indicador sin observaciones"))?;
            for p in data {
                out.push(EconomicPoint {
                    source: format!("INEGI {bank}"),
                    series_id: indicator.into(),
                    period: p["TIME_PERIOD"].as_str().unwrap_or("").into(),
                    value: p["OBS_VALUE"].as_str().unwrap_or("").into(),
                    unit: s["UNIT"].as_str().unwrap_or("").into(),
                    description: format!(
                        "Indicador {indicator} · área {} · unidad {}",
                        s["OBSERVATIONS"][0]["COBER_GEO"].as_str().unwrap_or("0700"),
                        s["UNIT"].as_str().unwrap_or("")
                    ),
                    updated_at: crate::config::now(),
                });
            }
        }
        Ok(out)
    }
}
pub fn denue_record(raw: Value, entity: &str) -> Result<Prospect> {
    let obj = raw
        .as_object()
        .ok_or_else(|| invalid("Registro DENUE no es un objeto"))?;
    let required = |key: &str| -> Result<String> {
        let s = obj
            .get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| invalid(format!("Registro DENUE sin {key}")))?;
        if s.trim().is_empty() {
            return Err(invalid(format!("Campo {key} vacío")));
        }
        Ok(s.trim().into())
    };
    let mut p = Prospect {
        id: required("Id")?,
        nombre: required("Nombre")?,
        ..Default::default()
    };
    let mut fields = serde_json::to_value(&p)?;
    let aliases = [
        ("razon_social", "Razon_social"),
        ("clase_actividad", "Clase_actividad"),
        ("estrato", "Estrato"),
        ("tipo_establecimiento", "Tipo"),
        ("calle", "Calle"),
        ("num_exterior", "Num_Exterior"),
        ("num_interior", "Num_Interior"),
        ("edificio", "EDIFICIO"),
        ("numero_local", "numero_local"),
        ("cp", "CP"),
        ("telefono", "Telefono"),
        ("correo", "Correo_e"),
        ("sitio", "Sitio_internet"),
        ("corredor_nombre", "nom_corredor_industrial"),
        ("corredor_tipo", "tipo_corredor_industrial"),
        ("clase_id", "CLASE_ACTIVIDAD_ID"),
        ("rama_id", "RAMA_ACTIVIDAD_ID"),
        ("subrama_id", "SUBRAMA_ACTIVIDAD_ID"),
        ("fecha_alta", "Fecha_Alta"),
    ];
    for (field, key) in aliases {
        let s = match obj.get(key) {
            Some(Value::String(s)) => s.trim(),
            Some(Value::Null) | None => "",
            _ => return Err(invalid(format!("DENUE: tipo inválido en {key}"))),
        };
        fields[field] = s.into();
    }
    p = serde_json::from_value(fields)?;
    let text = |key: &str| obj.get(key).and_then(Value::as_str).unwrap_or("").trim();
    let parts = text("Ubicacion")
        .split(',')
        .map(str::trim)
        .collect::<Vec<_>>();
    p.entidad = crate::config::entity_name(entity).into();
    p.municipio = if parts.len() >= 2 {
        crate::rules::title(parts[parts.len() - 2])
    } else {
        String::new()
    };
    p.localidad = parts
        .first()
        .map(|s| crate::rules::title(s))
        .unwrap_or_default();
    p.direccion = [
        text("Tipo_vialidad"),
        &p.calle,
        &p.num_exterior,
        text("Colonia"),
    ]
    .join(" ")
    .trim()
    .into();
    let phone = crate::contacts::phone(&p.telefono);
    if !phone.is_empty() {
        p.telefono = phone;
    }
    p.correo = p.correo.to_lowercase();
    p.dominio = crate::contacts::domain(&p.sitio);
    p.giro = crate::rules::classify(&p.clase_id, &p.clase_actividad).giro;
    let coord = |key: &str, min: f64, max: f64| {
        text(key)
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite() && *n >= min && *n <= max)
    };
    p.latitud = coord("Latitud", 14.0, 33.5);
    p.longitud = coord("Longitud", -119.0, -86.0);
    p.estado = "nuevo".into();
    Ok(p)
}
/// Compatibility entry point: an explicit call is now bounded and skips completed scopes.
pub async fn pull(
    db: &Db,
    _paths: &AppPaths,
    settings: &Settings,
    profile: Profile,
    limit: Option<usize>,
    control: &JobControl,
) -> Result<usize> {
    let mut settings = settings.clone();
    settings.denue_pages = crate::updates::record_limit_pages(limit, settings.denue_pages)?;
    crate::updates::pull_batch(
        db,
        &settings,
        profile,
        crate::updates::ImportMode::New,
        control,
    )
    .await
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EconomicPoint {
    pub source: String,
    pub series_id: String,
    pub period: String,
    pub value: String,
    pub unit: String,
    pub description: String,
    pub updated_at: String,
}
impl Db {
    pub async fn store_economics(&self, rows: Vec<EconomicPoint>) -> Result<usize> {
        self.call(move|c|{let tx=c.transaction()?;for p in &rows{tx.execute("INSERT INTO economic_series(source,series_id,period,value,unit,description,updated_at) VALUES (?,?,?,?,?,?,?) ON CONFLICT(source,series_id,period) DO UPDATE SET value=excluded.value,unit=excluded.unit,description=excluded.description,updated_at=excluded.updated_at",rusqlite::params![p.source,p.series_id,p.period,p.value,p.unit,p.description,p.updated_at])?;}tx.commit()?;Ok(rows.len())}).await
    }
    pub async fn economics(&self) -> Result<Vec<EconomicPoint>> {
        self.call(|c|{let mut s=c.prepare("SELECT source,series_id,period,value,unit,description,updated_at FROM economic_series ORDER BY source,series_id,updated_at DESC LIMIT 100")?;Ok(s.query_map([],|r|Ok(EconomicPoint{source:r.get(0)?,series_id:r.get(1)?,period:r.get(2)?,value:r.get(3)?,unit:r.get(4)?,description:r.get(5)?,updated_at:r.get(6)?}))?.collect::<std::result::Result<Vec<_>,_>>()?)}).await
    }
}
pub async fn sync_economics(
    db: &Db,
    paths: &AppPaths,
    source: &str,
    indicator: &str,
    control: &JobControl,
) -> Result<usize> {
    if !["banxico", "inegi"].contains(&source) {
        return Err(invalid("Fuente de indicadores inválida"));
    }
    let scope = if source == "banxico" {
        "banxico".to_owned()
    } else {
        format!("inegi:{indicator}")
    };
    db.source_attempt(scope.clone(), source.into()).await?;
    let result = async {
        let secrets = paths.secrets()?;
        let client = GovernmentClient::new()?;
        let values = if source == "banxico" {
            client.banxico(&secrets.banxico, control).await?
        } else {
            client.inegi(&secrets.inegi, indicator, control).await?
        };
        control.check()?;
        if values.is_empty() {
            return Err(invalid(
                "La fuente no devolvió observaciones. Se conservan los datos anteriores.",
            ));
        }
        let n = db.store_economics(values).await?;
        control.update(|s| s.processed = n);
        Ok(n)
    }
    .await;
    db.source_result(
        scope,
        result.is_ok(),
        result
            .as_ref()
            .err()
            .map(ToString::to_string)
            .unwrap_or_default(),
    )
    .await?;
    result
}
