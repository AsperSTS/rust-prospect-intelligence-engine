use prospect_core::{
    AppPaths, Command, Db, Mutation, Profile, Prospect, Query, QueryGroup, QueryOrder, Settings,
    View,
    jobs::JobControl,
    workspace::{Density, DenuePageAudit, TableColumn, TableLayout},
};

fn record(id: &str, name: &str, municipality: &str, entity: &str) -> Prospect {
    Prospect {
        id: id.into(),
        nombre: name.into(),
        municipio: municipality.into(),
        entidad: entity.into(),
        correo: "ventas@example.org".into(),
        telefono: "2221234567".into(),
        ..Default::default()
    }
}

#[tokio::test]
async fn views_survive_restart_separate_offers_and_keep_legacy_filters() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::at(temp.path()).unwrap();
    let db = Db::open(paths.clone()).unwrap();
    let mut q = Query {
        view: View::All,
        entidad: "Puebla".into(),
        order: QueryOrder::Name,
        group: QueryGroup::Municipality,
        page: 3,
        size: 30,
        ..Default::default()
    };
    let layout = TableLayout {
        columns: vec![TableColumn::Name, TableColumn::State, TableColumn::Entity],
        density: Density::Compact,
    };
    let mut old = serde_json::to_value(&q).unwrap();
    old.as_object_mut().unwrap().remove("order");
    old.as_object_mut().unwrap().remove("group");
    db.call(move |c| {
        c.execute(
            "INSERT INTO saved_filters VALUES ('Anterior',?)",
            [old.to_string()],
        )?;
        Ok(())
    })
    .await
    .unwrap();
    db.save_view(" Industrial ".into(), q.clone(), layout.clone())
        .await
        .unwrap();
    q.profile = Profile::WebMarketing;
    q.entidad = "Jalisco".into();
    db.save_view("Industrial".into(), q.clone(), TableLayout::default())
        .await
        .unwrap();
    let invalid_layout = TableLayout {
        columns: vec![TableColumn::Name, TableColumn::State, TableColumn::State],
        ..Default::default()
    };
    assert!(
        db.save_view("Inválida".into(), q.clone(), invalid_layout)
            .await
            .is_err()
    );
    assert!(
        db.save_view("  ".into(), q.clone(), layout.clone())
            .await
            .is_err()
    );
    assert!(serde_json::from_str::<Query>(r#"{"order":"nombre; DROP TABLE prospectos"}"#).is_err());
    assert!(
        serde_json::from_str::<TableLayout>(r#"{"columns":["name"],"unexpected":true}"#).is_err()
    );
    drop(db);
    let db = Db::open(paths).unwrap();
    let automation = db.saved_views(Profile::Automatizacion).await.unwrap();
    assert_eq!(automation.len(), 1);
    assert_eq!(automation[0].name, "Industrial");
    assert_eq!(automation[0].layout, layout);
    assert_eq!(automation[0].query.entidad, "Puebla");
    assert_eq!(automation[0].query.page, 0);
    assert_eq!(automation[0].query.order, QueryOrder::Name);
    assert_eq!(automation[0].query.group, QueryGroup::Municipality);
    let web = db.saved_views(Profile::WebMarketing).await.unwrap();
    assert_eq!(web.len(), 1);
    assert_eq!(web[0].query.entidad, "Jalisco");
    assert_ne!(web[0].layout, layout);
    let legacy = db.saved_filters().await.unwrap();
    assert_eq!(legacy[0].0, "Anterior");
    assert_eq!(legacy[0].1.order, QueryOrder::Automatic);
    assert_eq!(legacy[0].1.group, QueryGroup::None);
    db.delete_view(Profile::WebMarketing, "Industrial".into())
        .await
        .unwrap();
    assert!(
        db.saved_views(Profile::WebMarketing)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        db.saved_views(Profile::Automatizacion).await.unwrap(),
        automation
    );
}

#[tokio::test]
async fn ordering_and_groups_cover_the_entire_result_before_pagination() {
    let temp = tempfile::tempdir().unwrap();
    let db = Db::open(AppPaths::at(temp.path()).unwrap()).unwrap();
    db.import_records(vec![
        record("1", "Zeta", "M1", "Puebla"),
        record("2", "Alfa", "M2", "Puebla"),
        record("3", "Beta", "M0", "Tlaxcala"),
        record("4", "Alfa", "M1", "Puebla"),
    ])
    .await
    .unwrap();
    let mut q = Query {
        size: 1,
        view: View::All,
        order: QueryOrder::Name,
        ..Default::default()
    };
    let mut ids = vec![];
    for page in 0..4 {
        q.page = page;
        let data = db.page(q.clone()).await.unwrap();
        assert_eq!(data.total, 4);
        ids.push(data.items[0].id.clone());
    }
    assert_eq!(ids, vec!["2", "4", "3", "1"]);
    q.group = QueryGroup::Municipality;
    let mut grouped = vec![];
    for page in 0..4 {
        q.page = page;
        grouped.push(db.page(q.clone()).await.unwrap().items[0].id.clone());
    }
    assert_eq!(grouped, vec!["4", "1", "2", "3"]);
    assert_eq!(
        db.page(Query {
            search: "Zeta".into(),
            ..q
        })
        .await
        .unwrap()
        .total,
        1
    );
}

#[tokio::test]
async fn audited_pages_count_real_field_changes_preserve_crm_and_rollback_together() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::at(temp.path()).unwrap();
    let db = Db::open(paths.clone()).unwrap();
    let a = record("a", "Antes", "Puebla", "Puebla");
    let b = record("b", "Igual", "Puebla", "Puebla");
    db.import_records(vec![a.clone(), b.clone()]).await.unwrap();
    db.mutate(
        Mutation {
            operation_id: prospect_core::operation_id(),
            prospect_id: "a".into(),
            profile: Profile::Automatizacion,
            command: Command::Note {},
            notes: Some("Conservar esta nota".into()),
        },
        Settings::default(),
    )
    .await
    .unwrap();
    db.mutate(
        Mutation {
            operation_id: prospect_core::operation_id(),
            prospect_id: "a".into(),
            profile: Profile::Automatizacion,
            command: Command::Snooze { days: 3 },
            notes: None,
        },
        Settings::default(),
    )
    .await
    .unwrap();
    let before = db.get("a".into(), Profile::Automatizacion).await.unwrap();
    let history = db.history("a".into()).await.unwrap();
    let mut changed = a.clone();
    changed.nombre = "Después".into();
    changed.correo.clear();
    changed.telefono.clear();
    let control = JobControl::new("DENUE · prueba aislada");
    let job = control.snapshot().id;
    let audit = |start| DenuePageAudit {
        job_id: job.clone(),
        profile: Profile::Automatizacion,
        scope: "audit-scope".into(),
        start,
        invalid: if start == 1 { 4 } else { 0 },
    };
    let summary = db
        .import_audited_page(
            vec![changed.clone(), b, record("c", "Nuevo", "Puebla", "Puebla")],
            audit(1),
            false,
        )
        .await
        .unwrap();
    assert_eq!(
        (summary.inserted, summary.updated, summary.unchanged),
        (1, 1, 1)
    );
    assert_eq!(summary.changes.records_with_losses, 1);
    assert_eq!(summary.changes.emptied.get("correo"), Some(&1));
    assert_eq!(summary.changes.emptied.get("telefono"), Some(&1));
    assert_eq!(summary.changes.modified.get("nombre"), Some(&1));
    let mut duplicate = changed.clone();
    duplicate.nombre = "No guardar".into();
    assert!(
        db.import_audited_page(vec![duplicate], audit(1), false)
            .await
            .is_err()
    );
    assert_eq!(
        db.get("a".into(), Profile::Automatizacion)
            .await
            .unwrap()
            .nombre,
        "Después"
    );
    assert_eq!(db.checkpoint("audit-scope".into()).await.unwrap(), 201);
    db.import_audited_page(vec![changed.clone()], audit(201), true)
        .await
        .unwrap();
    let mut invalid = changed;
    invalid.nombre.clear();
    assert!(
        db.import_audited_page(
            vec![
                record("rollback", "No insertar", "Puebla", "Puebla"),
                invalid
            ],
            DenuePageAudit {
                job_id: "failed".into(),
                profile: Profile::WebMarketing,
                scope: "failed-scope".into(),
                start: 1,
                invalid: 0
            },
            false
        )
        .await
        .is_err()
    );
    assert_eq!(
        db.page(Query {
            search: "No insertar".into(),
            ..Default::default()
        })
        .await
        .unwrap()
        .total,
        0
    );
    let after = db.get("a".into(), Profile::Automatizacion).await.unwrap();
    assert_eq!(after.notas, before.notas);
    assert_eq!(after.estado, before.estado);
    assert_eq!(after.proximo_seguimiento, before.proximo_seguimiento);
    assert_eq!(after.creado_en, before.creado_en);
    assert_eq!(db.history("a".into()).await.unwrap(), history);
    control.finish(&Ok(4));
    db.log_job(control.snapshot()).await.unwrap();
    drop(db);
    let db = Db::open(paths).unwrap();
    let batches = db
        .denue_batch_summaries(Profile::Automatizacion)
        .await
        .unwrap();
    assert_eq!(batches.len(), 1);
    let batch = &batches[0];
    assert_eq!(batch.status, "completed");
    assert_eq!(
        (
            batch.pages,
            batch.inserted,
            batch.updated,
            batch.unchanged,
            batch.invalid
        ),
        (2, 1, 1, 2, 4)
    );
    assert_eq!(batch.changes.records_with_losses, 1);
    assert_eq!(batch.changes.emptied, summary.changes.emptied);
    assert!(
        db.denue_batch_summaries(Profile::WebMarketing)
            .await
            .unwrap()
            .is_empty()
    );
}
