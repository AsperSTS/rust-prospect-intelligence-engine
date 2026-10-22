use prospect_core::{AppPaths, Db, Profile, Query, View};
use std::time::Instant;
#[tokio::test]
#[ignore = "Requires PROSPECT_PERF_DB, copies local data to an isolated temporary directory"]
async fn navigation_read_benchmark() {
    let source = std::env::var("PROSPECT_PERF_DB").expect("QA database path");
    let temp = tempfile::TempDir::new().unwrap();
    let paths = AppPaths::at(temp.path()).unwrap();
    std::fs::copy(source, &paths.db).unwrap();
    let db = Db::open(paths).unwrap();
    for profile in [Profile::Automatizacion, Profile::WebMarketing] {
        for dimension in ["giro", "municipio"] {
            let start = Instant::now();
            let metrics = db.metrics(profile, dimension.into()).await.unwrap();
            let cold = start.elapsed();
            let start = Instant::now();
            for _ in 0..10 {
                assert_eq!(
                    db.metrics(profile, dimension.into()).await.unwrap(),
                    metrics
                );
            }
            println!(
                "{} / {}: cold {:.2} ms, repeated {:.2} ms",
                profile.id(),
                dimension,
                cold.as_secs_f64() * 1000.0,
                start.elapsed().as_secs_f64() * 100.0
            );
        }
        let q = Query {
            profile,
            view: View::All,
            size: 60,
            ..Default::default()
        };
        let start = Instant::now();
        let page = db.page(q.clone()).await.unwrap();
        let cold = start.elapsed();
        let start = Instant::now();
        for _ in 0..10 {
            assert_eq!(db.page(q.clone()).await.unwrap(), page);
        }
        println!(
            "{} / base: {} rows, cold {:.2} ms, repeated {:.2} ms",
            profile.id(),
            page.total,
            cold.as_secs_f64() * 1000.0,
            start.elapsed().as_secs_f64() * 100.0
        );
    }
}

#[tokio::test]
async fn cached_views_separate_offers_filters_and_refresh_after_crm_changes() {
    use prospect_core::{
        Command, Mutation, Prospect, Settings, analytics::OpportunityDraft, jobs::JobControl,
    };
    let temp = tempfile::TempDir::new().unwrap();
    let db = Db::open(AppPaths::at(temp.path()).unwrap()).unwrap();
    db.import_records(vec![Prospect {
        id: "cache-1".into(),
        nombre: "Industria demo".into(),
        clase_id: "336310".into(),
        clase_actividad: "Autopartes".into(),
        estrato: "31 a 50 personas".into(),
        tipo_establecimiento: "fijo".into(),
        municipio: "Puebla".into(),
        entidad: "Puebla".into(),
        telefono: "2221234567".into(),
        latitud: Some(19.0),
        longitud: Some(-98.2),
        ..Default::default()
    }])
    .await
    .unwrap();
    db.analyze(Settings::default(), JobControl::new("cache-test"))
        .await
        .unwrap();
    for profile in [Profile::Automatizacion, Profile::WebMarketing] {
        let q = Query {
            profile,
            view: View::All,
            size: 60,
            ..Default::default()
        };
        let p = db.get("cache-1".into(), profile).await.unwrap();
        let page = db.page(q.clone()).await.unwrap();
        assert_eq!(page.items, vec![p.clone()]);
        let metrics = db.metrics(profile, "giro".into()).await.unwrap();
        assert_eq!(metrics.points[0].fit, p.fit_score);
        assert_eq!(metrics.points[0].priority, p.priority_score);
        assert_eq!(
            db.metrics(profile, "municipio".into())
                .await
                .unwrap()
                .points,
            metrics.points
        );
        assert_eq!(
            db.page(Query {
                search: "sin coincidencias".into(),
                ..q
            })
            .await
            .unwrap()
            .total,
            0
        );
    }
    let auto_before = db
        .metrics(Profile::Automatizacion, "giro".into())
        .await
        .unwrap();
    let web_before = db
        .metrics(Profile::WebMarketing, "giro".into())
        .await
        .unwrap();
    db.save_opportunity(OpportunityDraft {
        prospect_id: "cache-1".into(),
        profile: Profile::WebMarketing,
        title: "Web demo".into(),
        stage: "ganado".into(),
        currency: "MXN".into(),
        amount_cents: 100000,
        received_cents: 25000,
        ..Default::default()
    })
    .await
    .unwrap();
    assert_eq!(
        db.metrics(Profile::Automatizacion, "giro".into())
            .await
            .unwrap(),
        auto_before
    );
    let web_after = db
        .metrics(Profile::WebMarketing, "giro".into())
        .await
        .unwrap();
    assert_eq!(
        web_after.opportunity_count,
        web_before.opportunity_count + 1
    );
    assert_eq!(web_after.received_mxn_cents, 25000);
    db.mutate(
        Mutation {
            operation_id: prospect_core::operation_id(),
            prospect_id: "cache-1".into(),
            profile: Profile::WebMarketing,
            command: Command::Note {},
            notes: Some("Nueva nota".into()),
        },
        Settings::default(),
    )
    .await
    .unwrap();
    let page = db
        .page(Query {
            view: View::All,
            profile: Profile::WebMarketing,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(page.items[0].notas, "Nueva nota");
}
