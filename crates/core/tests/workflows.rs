use prospect_core::{
    AppPaths, Command, Db, Mutation, Profile, Prospect, Query, Settings, View,
    analytics::{OpportunityDraft, money_cents},
    config, db, groups,
    jobs::JobControl,
    messages,
};
use tempfile::TempDir;
fn settings() -> Settings {
    Settings {
        owner_name: "Demo Validación".into(),
        owner_whatsapp: "2221234567".into(),
        owner_credential: "Participé en proyectos de automatización.".into(),
        ..Default::default()
    }
}
fn record(id: &str) -> Prospect {
    Prospect {
        id: id.into(),
        nombre: format!("Empresa Demo {id}"),
        clase_id: "336310".into(),
        clase_actividad: "Fabricación de autopartes".into(),
        giro: "Autopartes".into(),
        giro_display: "autopartes".into(),
        estrato: "31 a 50 personas".into(),
        tipo_establecimiento: "fijo".into(),
        telefono: "2221234567".into(),
        correo: "ventas@negocio-demo.mx".into(),
        estado: "nuevo".into(),
        ..Default::default()
    }
}
fn request(id: &str, command: Command) -> Mutation {
    Mutation {
        operation_id: prospect_core::operation_id(),
        prospect_id: id.into(),
        profile: Profile::Automatizacion,
        command,
        notes: Some("Nota de validación".into()),
    }
}
async fn setup() -> (TempDir, Db) {
    let temp = TempDir::new().unwrap();
    let db = Db::open(AppPaths::at(temp.path()).unwrap()).unwrap();
    db.import_records(vec![record("1")]).await.unwrap();
    (temp, db)
}
fn contact(p: &Prospect) -> Command {
    let t = messages::Templates::load(std::path::Path::new("/nonexistent/fixtures")).unwrap();
    let m = messages::first(p, "email", &[], &settings(), &t, Profile::Automatizacion).unwrap();
    Command::Contact {
        channel: "email".into(),
        variant: m.variante,
        message: m.cuerpo,
        subject: m.asunto,
    }
}

#[tokio::test]
async fn refresh_does_not_overwrite_crm_and_double_click_is_idempotent() {
    let (_temp, db) = setup().await;
    let p = db.get("1".into(), Profile::Automatizacion).await.unwrap();
    let req = request("1", contact(&p));
    let date = config::parse_date("2026-10-01").unwrap();
    db.mutate_at(req.clone(), settings(), date).await.unwrap();
    db.mutate_at(req, settings(), date).await.unwrap();
    let mut changed = record("1");
    changed.nombre = "Nuevo nombre público".into();
    db.import_records(vec![changed]).await.unwrap();
    let p = db.get("1".into(), Profile::Automatizacion).await.unwrap();
    assert_eq!(p.nombre, "Nuevo nombre público");
    assert_eq!(p.estado, "contactado");
    assert_eq!(p.notas, "Nota de validación");
    assert_eq!(p.proximo_seguimiento, "2026-10-04");
    assert_eq!(db.history("1".into()).await.unwrap().len(), 1);
}
#[tokio::test]
async fn followups_keep_actual_send_dates_and_response_cancels_sequence() {
    let (_temp, db) = setup().await;
    let p = db.get("1".into(), Profile::Automatizacion).await.unwrap();
    db.mutate_at(
        request("1", contact(&p)),
        settings(),
        config::parse_date("2026-10-01").unwrap(),
    )
    .await
    .unwrap();
    let follow = || Command::Followup {
        channel: "email".into(),
        variant: "FU-d3".into(),
        message: "Buen día. Puedo mostrarle un ejemplo sencillo.".into(),
    };
    db.mutate_at(
        request("1", follow()),
        settings(),
        config::parse_date("2026-10-06").unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        db.get("1".into(), Profile::Automatizacion)
            .await
            .unwrap()
            .proximo_seguimiento,
        "2026-10-10"
    );
    db.mutate_at(
        request("1", follow()),
        settings(),
        config::parse_date("2026-10-13").unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        db.get("1".into(), Profile::Automatizacion)
            .await
            .unwrap()
            .proximo_seguimiento,
        "2026-10-20"
    );
    db.mutate_at(
        request(
            "1",
            Command::Response {
                response: "interesado".into(),
            },
        ),
        settings(),
        config::parse_date("2026-10-14").unwrap(),
    )
    .await
    .unwrap();
    let p = db.get("1".into(), Profile::Automatizacion).await.unwrap();
    assert_eq!(p.estado, "respuesta_positiva");
    assert_eq!(p.etapa_max, 3);
    assert!(p.proximo_seguimiento.is_empty());
    assert!(db.mutate(request("1", follow()), settings()).await.is_err());
}
#[tokio::test]
async fn third_followup_closes_without_resetting_funnel() {
    let (_temp, db) = setup().await;
    let p = db.get("1".into(), Profile::Automatizacion).await.unwrap();
    db.mutate(request("1", contact(&p)), settings())
        .await
        .unwrap();
    for step in 1..=3 {
        db.mutate(
            request(
                "1",
                Command::Followup {
                    channel: "email".into(),
                    variant: format!("FU-{step}"),
                    message: "Hola. Puedo mostrarle un ejemplo sencillo.".into(),
                },
            ),
            settings(),
        )
        .await
        .unwrap();
    }
    let p = db.get("1".into(), Profile::Automatizacion).await.unwrap();
    assert_eq!(p.estado, "sin_respuesta");
    assert!(p.proximo_seguimiento.is_empty());
    assert_eq!(p.etapa_max, 1);
    assert_eq!(db.history("1".into()).await.unwrap().len(), 4);
}
#[tokio::test]
async fn batch_import_and_cancelled_analysis_are_atomic() {
    let (_temp, db) = setup().await;
    let mut invalid = record("3");
    invalid.nombre.clear();
    assert!(db.import_records(vec![record("2"), invalid]).await.is_err());
    assert_eq!(db.summary().await.unwrap().total, 1);
    let job = JobControl::new("test");
    job.token.cancel();
    assert!(db.analyze(settings(), job).await.is_err());
    let count = db
        .call(|c| {
            Ok(c.query_row("SELECT COUNT(*) FROM profile_scores", [], |r| {
                r.get::<_, i64>(0)
            })?)
        })
        .await
        .unwrap();
    assert_eq!(count, 0);
}
#[tokio::test]
async fn backup_and_return_preserve_new_rust_interactions() {
    let (temp, db) = setup().await;
    let p = db.get("1".into(), Profile::Automatizacion).await.unwrap();
    db.mutate(request("1", contact(&p)), settings())
        .await
        .unwrap();
    let copy = db
        .restore_to(temp.path().join("return-to-streamlit.db"))
        .await
        .unwrap();
    let con = rusqlite::Connection::open(copy).unwrap();
    db::verify(&con).unwrap();
    assert_eq!(
        con.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        con.query_row("SELECT COUNT(*) FROM interactions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(db.backup().await.unwrap().exists());
}
#[tokio::test]
async fn legacy_migration_copies_all_tables_and_never_overwrites_destination() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("legacy.db");
    let c = rusqlite::Connection::open(&src).unwrap();
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../config/db.json")).unwrap();
    c.execute_batch(schema["SCHEMA"].as_str().unwrap()).unwrap();
    c.execute(
        "INSERT INTO prospectos(id,nombre,estado,toques) VALUES ('1','Empresa Demo','ganado',1)",
        [],
    )
    .unwrap();
    let paths = AppPaths::at(temp.path().join("rust-data")).unwrap();
    let report = db::migrate_snapshot(&src, &paths).unwrap();
    assert_eq!(report.counts["prospectos"], 1);
    assert!(db::migrate_snapshot(&src, &paths).is_err());
    let db = Db::open(paths).unwrap();
    let p = db.get("1".into(), Profile::Automatizacion).await.unwrap();
    assert_eq!(p.estado, "cliente");
    assert_eq!(db.history("1".into()).await.unwrap().len(), 1);
    assert_eq!(
        c.query_row("SELECT estado FROM prospectos", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "ganado"
    );
}
#[tokio::test]
async fn literal_search_pagination_and_profiles_remain_separate() {
    let (_temp, db) = setup().await;
    let mut p = record("2");
    p.nombre = "Demo (A+B) Ñandú".into();
    p.clase_id = "461110".into();
    p.clase_actividad = "Comercio al por menor de abarrotes".into();
    db.import_records(vec![p]).await.unwrap();
    db.analyze(settings(), JobControl::new("test"))
        .await
        .unwrap();
    let page = db
        .page(Query {
            view: View::All,
            search: "(a+b)".into(),
            size: 1,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].id, "2");
    assert_eq!(
        db.page(Query {
            view: View::All,
            search: "ñANDÚ".into(),
            ..Default::default()
        })
        .await
        .unwrap()
        .total,
        1
    );
    let a = db.get("2".into(), Profile::Automatizacion).await.unwrap();
    let web = db.get("2".into(), Profile::WebMarketing).await.unwrap();
    assert_ne!(a.fit_score, web.fit_score);
    assert_eq!(a.estado, web.estado);
    assert_eq!(
        db.page(Query {
            view: View::All,
            size: 1,
            page: 1,
            ..Default::default()
        })
        .await
        .unwrap()
        .items
        .len(),
        1
    );
}
#[tokio::test]
async fn exports_preserve_strings_and_defend_against_formulas() {
    let (_temp, db) = setup().await;
    let mut p = record("2");
    p.nombre = "=HYPERLINK(\"https://malicious.invalid\")".into();
    db.import_records(vec![p]).await.unwrap();
    let csv = db
        .export(
            Query {
                view: View::All,
                ..Default::default()
            },
            "csv",
        )
        .await
        .unwrap();
    let text = std::fs::read_to_string(csv).unwrap();
    assert!(text.contains("'=HYPERLINK"));
    assert!(text.contains("2221234567"));
    let xlsx = db
        .export(
            Query {
                view: View::All,
                ..Default::default()
            },
            "xlsx",
        )
        .await
        .unwrap();
    assert_eq!(&std::fs::read(xlsx).unwrap()[..2], b"PK");
    assert_eq!(db::safe_cell("\t=2+2"), "'\t=2+2");
}
#[tokio::test]
async fn opportunities_keep_received_money_separate_from_quotes() {
    let (_temp, db) = setup().await;
    let draft = OpportunityDraft {
        prospect_id: "1".into(),
        profile: Profile::WebMarketing,
        title: "Sitio de demostración".into(),
        stage: "propuesta".into(),
        amount_cents: 1500000,
        received_cents: 0,
        currency: "MXN".into(),
        ..Default::default()
    };
    let id = db.save_opportunity(draft.clone()).await.unwrap();
    let m = db
        .metrics(Profile::WebMarketing, "giro".into())
        .await
        .unwrap();
    assert_eq!(m.won_mxn_cents, 0);
    assert_eq!(m.received_mxn_cents, 0);
    db.save_opportunity(OpportunityDraft {
        id: Some(id),
        stage: "ganado".into(),
        received_cents: 500000,
        ..draft
    })
    .await
    .unwrap();
    let m = db
        .metrics(Profile::WebMarketing, "giro".into())
        .await
        .unwrap();
    assert_eq!(m.won_mxn_cents, 1500000);
    assert_eq!(m.received_mxn_cents, 500000);
}
#[tokio::test]
async fn checkpoints_survive_reopen_and_source_failure_preserves_enrichment() {
    let (temp, db) = setup().await;
    db.import_page(vec![record("2")], "demo-source".into(), 201, false)
        .await
        .unwrap();
    assert_eq!(db.checkpoint("demo-source".into()).await.unwrap(), 201);
    db.call(|c|{c.execute("UPDATE prospectos SET dominio='negocio-demo.mx',correo_web='contacto@negocio-demo.mx',source_website=1 WHERE id='1'",[])?;c.execute("INSERT INTO web_signals(prospect_id,signal_type,signal_text,signal_source_url) VALUES ('1','inventario','Manejamos inventario','https://negocio-demo.mx')",[])?;Ok(())}).await.unwrap();
    db.apply_enrichment(
        "negocio-demo.mx".into(),
        prospect_core::enrich::Enrichment {
            status: "error".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(db.signals("1".into()).await.unwrap().len(), 1);
    assert_eq!(
        db.get("1".into(), Profile::Automatizacion)
            .await
            .unwrap()
            .correo_web,
        "contacto@negocio-demo.mx"
    );
    let reopened = Db::open(AppPaths::at(temp.path()).unwrap()).unwrap();
    assert_eq!(
        reopened.checkpoint("demo-source".into()).await.unwrap(),
        201
    );
}
#[test]
fn incoming_commands_are_strict_and_money_never_uses_eval() {
    assert!(serde_json::from_str::<Mutation>(r#"{"operation_id":"x","prospect_id":"1","profile":"automatizacion","command":{"type":"research","execute":"shell"},"notes":null}"#).is_err());
    assert_eq!(money_cents("15,000.25").unwrap(), 1500025);
    assert!(money_cents("1+1").is_err());
    assert!(money_cents("NaN").is_err());
    assert!(money_cents("-1").is_err());
    assert!(money_cents("1.234").is_err());
    assert_eq!(groups::normalize("ACERO S.A. DE C.V."), "acero");
}
#[tokio::test]
async fn inspector_blocks_private_destinations_and_does_not_execute_html() {
    for address in [
        "127.0.0.1",
        "10.0.0.1",
        "192.168.1.1",
        "169.254.169.254",
        "100.64.0.1",
        "198.18.0.1",
        "::1",
        "fe80::1",
        "::ffff:127.0.0.1",
        "2001:db8::1",
    ] {
        assert!(
            !prospect_core::enrich::public_ip(address.parse().unwrap()),
            "{address}"
        );
    }
    assert!(prospect_core::enrich::public_ip("8.8.8.8".parse().unwrap()));
    let url = url::Url::parse("http://127.0.0.1/secret").unwrap();
    assert!(prospect_core::enrich::safe_addresses(&url).await.is_err());
    let html = "<nav>Inventario</nav><script>alert('ISO 9001')</script><p>Ofrecemos servicio técnico y cotizaciones.</p><footer>Proveedores</footer>";
    let text = prospect_core::enrich::html_text(html);
    assert!(!text.contains("alert"));
    assert!(!text.contains("Inventario"));
    assert!(!text.contains("Proveedores"));
    assert!(text.contains("servicio técnico"));
}
