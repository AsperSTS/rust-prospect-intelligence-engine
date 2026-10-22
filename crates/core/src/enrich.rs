use crate::{Db, Error, Result, Signal, invalid, jobs::JobControl};
use futures_util::stream::{self, StreamExt};
use scraper::{Html, Selector};
use std::{
    collections::HashSet,
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use url::Url;
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Enrichment {
    pub status: String,
    pub pages: usize,
    pub email: String,
    pub whatsapp: String,
    pub signals: Vec<Signal>,
}
pub fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            let a = v.octets();
            !v.is_private()
                && !v.is_loopback()
                && !v.is_link_local()
                && !v.is_multicast()
                && !v.is_unspecified()
                && !v.is_broadcast()
                && !v.is_documentation()
                && a[0] != 0
                && a[0] < 224
                && !(a[0] == 100 && (64..=127).contains(&a[1]))
                && !(a[0] == 192 && a[1] == 0 && a[2] == 0)
                && !(a[0] == 198 && (18..=19).contains(&a[1]))
                && !(a[0] == 192 && a[1] == 88 && a[2] == 99)
        }
        IpAddr::V6(v) => {
            if let Some(mapped) = v.to_ipv4_mapped() {
                return public_ip(IpAddr::V4(mapped));
            }
            let s = v.segments();
            (s[0] & 0xe000) == 0x2000
                && !(s[0] == 0x2001 && s[1] == 0x0db8)
                && !(s[0] == 0x2001 && s[1] <= 0x01ff)
                && s[0] != 0x2002
        }
    }
}
pub async fn safe_addresses(url: &Url) -> Result<Vec<SocketAddr>> {
    if !crate::contacts::safe_url(url) {
        return Err(invalid("URL no permitida"));
    }
    let host = url.host_str().ok_or_else(|| invalid("Destino sin host"))?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| invalid("Puerto inválido"))?;
    let addresses = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::net::lookup_host((host, port)),
    )
    .await
    .map_err(|_| invalid("DNS agotó su tiempo"))?
    .map_err(|_| invalid("DNS no disponible"))?
    .collect::<Vec<_>>();
    if addresses.is_empty() || addresses.iter().any(|a| !public_ip(a.ip())) {
        return Err(invalid("Destino privado o reservado bloqueado"));
    }
    Ok(addresses)
}
pub async fn download(start: &str, control: &JobControl) -> Result<Option<(String, String)>> {
    let Some(mut url) = crate::contacts::normalize_url(start) else {
        return Ok(None);
    };
    for hop in 0..=3 {
        control.check()?;
        let addresses = tokio::select! {_=control.token.cancelled()=>return Err(Error::Cancelled),r=safe_addresses(&url)=>r?};
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .user_agent("ProspectIntelligence/0.1 (personal research)")
            .resolve_to_addrs(url.host_str().unwrap(), &addresses)
            .build()
            .map_err(|_| invalid("No se pudo preparar la conexión"))?;
        control.update(|s| s.requests += 1);
        let response = tokio::select! {_=control.token.cancelled()=>return Err(Error::Cancelled),r=client.get(url.clone()).send()=>match r{Ok(r)=>r,Err(_)=>return Ok(None)}};
        if response.status().is_redirection() {
            if hop == 3 {
                return Ok(None);
            }
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|h| h.to_str().ok());
            let Some(location) = location else {
                return Ok(None);
            };
            url = match url.join(location) {
                Ok(u) => u,
                Err(_) => return Ok(None),
            };
            continue;
        }
        if response.status() != reqwest::StatusCode::OK {
            return Ok(None);
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_lowercase();
        if !["text/html", "application/xhtml+xml", "text/plain"].contains(&content_type.as_str())
            || response.content_length().is_some_and(|n| n > 1_000_000)
        {
            return Ok(None);
        }
        // No automatic decompression is enabled. A compressed or binary payload is rejected.
        if response
            .headers()
            .get(reqwest::header::CONTENT_ENCODING)
            .is_some_and(|h| h.as_bytes() != b"identity")
        {
            return Ok(None);
        }
        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        loop {
            let part = tokio::select! {_=control.token.cancelled()=>return Err(Error::Cancelled),r=tokio::time::timeout(Duration::from_secs(5),stream.next())=>match r{Ok(r)=>r,Err(_)=>return Ok(None)}};
            let Some(part) = part else {
                break;
            };
            let part = match part {
                Ok(p) => p,
                Err(_) => return Ok(None),
            };
            if bytes.len() + part.len() > 1_000_000 {
                return Ok(None);
            }
            bytes.extend_from_slice(&part);
        }
        return Ok(Some((
            url.into(),
            String::from_utf8_lossy(&bytes).into_owned(),
        )));
    }
    Ok(None)
}
pub fn html_text(html: &str) -> String {
    let doc = Html::parse_document(html);
    let excluded = [
        "script", "style", "noscript", "svg", "nav", "header", "footer", "aside", "form", "button",
        "select", "iframe",
    ];
    let mut parts = Vec::new();
    for node in doc.tree.nodes() {
        if let Some(text) = node.value().as_text() {
            let blocked = node
                .ancestors()
                .filter_map(|n| n.value().as_element())
                .any(|e| excluded.contains(&e.name()));
            if !blocked {
                parts.push(text.text.as_ref());
            }
        }
    }
    html_escape::decode_html_entities(&parts.join(" "))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn internal_links(base: &str, html: &str) -> Vec<String> {
    let Ok(base) = Url::parse(base) else {
        return vec![];
    };
    let doc = Html::parse_document(html);
    let selector = Selector::parse("a[href]").unwrap();
    let groups = [
        vec!["nosotros", "quienes", "empresa", "about"],
        vec!["servicios", "services"],
        vec!["productos", "products", "catalogo"],
        vec!["contacto", "contact"],
    ];
    let mut chosen = vec![None; groups.len()];
    for a in doc.select(&selector) {
        let Some(href) = a.value().attr("href") else {
            continue;
        };
        let Ok(mut u) = base.join(href) else {
            continue;
        };
        if !crate::contacts::safe_url(&u)
            || u.host_str().unwrap_or("").trim_start_matches("www.")
                != base.host_str().unwrap_or("").trim_start_matches("www.")
        {
            continue;
        }
        let path = u.path().to_lowercase();
        if [
            ".pdf", ".zip", ".rar", ".exe", ".dmg", ".jpg", ".jpeg", ".png", ".gif", ".webp",
            ".svg", ".mp4", ".mp3", ".doc", ".docx", ".xls", ".xlsx", ".ppt", ".pptx",
        ]
        .iter()
        .any(|x| path.ends_with(x))
        {
            continue;
        }
        u.set_fragment(None);
        u.set_query(None);
        for (i, keywords) in groups.iter().enumerate() {
            if chosen[i].is_none() && keywords.iter().any(|s| path.contains(s)) {
                chosen[i] = Some(u.to_string());
            }
        }
    }
    chosen.into_iter().flatten().take(4).collect()
}
pub fn extract_contacts(html: &str, text: &str, domain: &str) -> (String, String) {
    use regex::Regex;
    use std::sync::LazyLock;
    static EMAIL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+").unwrap()
    });
    static WA: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:wa\.me/|api\.whatsapp\.com/send\?phone=|wa\.me/send\?phone=)\+?(\d{10,13})")
            .unwrap()
    });
    let combined = format!("{html} {text}");
    let mut seen = HashSet::new();
    let mut emails = EMAIL
        .find_iter(&combined)
        .map(|m| m.as_str().to_lowercase())
        .filter(|e| crate::contacts::email_valid(e) && seen.insert(e.clone()))
        .collect::<Vec<_>>();
    emails.sort_by_key(|e| !e.split('@').nth(1).unwrap_or("").contains(domain));
    let wa = WA
        .captures(html)
        .map(|c| c[1][c[1].len() - 10..].to_owned())
        .unwrap_or_default();
    (emails.first().cloned().unwrap_or_default(), wa)
}
pub async fn domain(domain: &str, control: &JobControl) -> Result<Enrichment> {
    if crate::contacts::domain(domain) != domain {
        return Ok(Enrichment {
            status: "bloqueado".into(),
            ..Default::default()
        });
    }
    let first = match download(&format!("https://{domain}/"), control).await {
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(_) => {
            return Ok(Enrichment {
                status: "bloqueado".into(),
                ..Default::default()
            });
        }
        Ok(Some(r)) => Some(r),
        Ok(None) => download(&format!("http://{domain}/"), control).await?,
    };
    let Some((base, html)) = first else {
        return Ok(Enrichment {
            status: "error".into(),
            ..Default::default()
        });
    };
    let mut pages = vec![(base.clone(), html.clone())];
    for link in internal_links(&base, &html) {
        control.wait(Duration::from_secs(1)).await?;
        match download(&link, control).await {
            Ok(Some(r)) => pages.push(r),
            Err(Error::Cancelled) => return Err(Error::Cancelled),
            _ => {}
        }
    }
    let mut res = Enrichment {
        status: "ok".into(),
        pages: pages.len(),
        ..Default::default()
    };
    let mut seen = HashSet::new();
    for (url, html) in pages {
        let text = html_text(&html);
        let (email, wa) = extract_contacts(&html, &text, domain);
        if res.email.is_empty() {
            res.email = email;
        }
        if res.whatsapp.is_empty() {
            res.whatsapp = wa;
        }
        res.signals
            .extend(crate::rules::detect(&text, &url, &mut seen));
    }
    Ok(res)
}
pub async fn pending(db: &Db, limit: usize, force: bool, control: &JobControl) -> Result<usize> {
    if limit == 0 || limit > 500 {
        return Err(invalid("Selecciona entre 1 y 500 dominios"));
    }
    let age = crate::Settings::load(&db.paths)?.web_max_age_days;
    let domains = db.pending_domains_with_age(limit, force, age).await?;
    let total = domains.len();
    control.update(|s| s.message = format!("Preparando {total} dominios"));
    let mut stream = stream::iter(domains.into_iter().map(|d| {
        let c = control.clone();
        async move {
            let r = domain(&d, &c).await;
            (d, r)
        }
    }))
    .buffer_unordered(8);
    let mut visited = 0;
    while let Some((domain, result)) = stream.next().await {
        control.check()?;
        let res = match result {
            Ok(r) => r,
            Err(Error::Cancelled) => return Err(Error::Cancelled),
            Err(_) => Enrichment {
                status: "error".into(),
                ..Default::default()
            },
        };
        db.apply_enrichment(domain, res).await?;
        visited += 1;
        control.update(|s| {
            s.processed = visited;
            s.message = format!("Dominios revisados: {visited} de {total}");
        });
    }
    Ok(visited)
}
