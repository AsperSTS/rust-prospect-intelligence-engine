//! Opt-in acceptance checks on a consistent copy of actual DENUE data.
//! Never reads credentials, requests sources or mutates the original database.
use prospect_core::{
    AppPaths, Db, Profile, Query, QueryGroup, QueryOrder, Settings, View,
    workspace::{Density, TableColumn, TableLayout},
};

#[tokio::test]
#[ignore = "Requires PROSPECT_REAL_DB with actual imported DENUE records; copies it into an isolated home"]
async fn expanded_real_data_supports_each_offer_views_maps_and_source_history() {
    let source = std::env::var("PROSPECT_REAL_DB")
        .expect("Set PROSPECT_REAL_DB to the real snapshot database");
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::at(temp.path()).unwrap();
    {
        let original = rusqlite::Connection::open_with_flags(
            &source,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let mut snapshot = rusqlite::Connection::open(&paths.db).unwrap();
        rusqlite::backup::Backup::new(&original, &mut snapshot)
            .unwrap()
            .run_to_completion(100, std::time::Duration::from_millis(5), None)
            .unwrap();
    }
    let db = Db::open(paths.clone()).unwrap();
    let counts = db.entity_counts().await.unwrap();
    let total: usize = counts.values().sum();
    assert!(
        total > 4709,
        "The expanded acceptance dataset needs real records beyond the initial coverage"
    );
    let statuses = db.source_statuses().await.unwrap();
    let settings = Settings::load_desktop(&paths).unwrap();
    assert_eq!(settings.entities.len(), 7);
    assert!(!paths.home.join("credentials.env").exists());
    for profile in [Profile::Automatizacion, Profile::WebMarketing] {
        let metrics = db.metrics(profile, "giro".into()).await.unwrap();
        for code in ["11", "14", "15", "19", "21", "22", "29"] {
            let entity = prospect_core::config::entity_name(code).to_owned();
            let expected = *counts
                .get(&entity)
                .expect("State absent from actual DENUE data");
            assert!(expected > 0);
            let q = Query {
                profile,
                entidad: entity.clone(),
                view: View::All,
                order: QueryOrder::Name,
                group: QueryGroup::Municipality,
                size: 30,
                ..Default::default()
            };
            let page = db.page(q.clone()).await.unwrap();
            assert_eq!(page.total, expected);
            assert!(!page.items.is_empty());
            assert!(
                page.items
                    .iter()
                    .all(|p| p.entidad == entity && !p.id.is_empty() && !p.nombre.is_empty())
            );
            assert!(
                metrics.points.iter().any(|p| p.entidad == entity
                    && (-90.0..=90.0).contains(&p.lat)
                    && (-180.0..=180.0).contains(&p.lon)),
                "State has no actual mapped records"
            );
            let layout = TableLayout {
                columns: vec![
                    TableColumn::Name,
                    TableColumn::Entity,
                    TableColumn::Municipality,
                    TableColumn::Priority,
                ],
                density: Density::Compact,
            };
            db.save_view(format!("Validación real {code}"), q.clone(), layout.clone())
                .await
                .unwrap();
            let saved = db
                .saved_views(profile)
                .await
                .unwrap()
                .into_iter()
                .find(|v| v.name == format!("Validación real {code}"))
                .unwrap();
            assert!(saved.query == q && saved.layout == layout);
            let restored = db.page(saved.query).await.unwrap();
            assert_eq!(restored.total, expected);
            assert!(
                restored
                    .items
                    .iter()
                    .map(|p| &p.id)
                    .eq(page.items.iter().map(|p| &p.id))
            );
        }
        let batches = db.denue_batch_summaries(profile).await.unwrap();
        assert!(!batches.is_empty());
        assert!(batches.iter().all(|b| b.pages <= 20
            && b.pages > 0
            && ["completed", "cancelled", "failed", "interrupted"].contains(&b.status.as_str())));
    }
    assert_eq!(db.source_statuses().await.unwrap(), statuses);
    assert_eq!(db.entity_counts().await.unwrap(), counts);
    db.call(|c| {
        prospect_core::db::verify(c)?;
        Ok(())
    })
    .await
    .unwrap();
    println!(
        "Actual DENUE acceptance: {total} records, seven states, both offers, saved views, map points and committed source batches. No credentials or source requests."
    );
}
