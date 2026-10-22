use prospect_core::{Profile, Prospect, Settings, Signal, groups, messages, rules};
use serde_json::Value;
#[test]
fn python_golden_behavior() {
    let data: Value = serde_json::from_str(include_str!("fixtures/parity.json")).unwrap();
    let date = prospect_core::config::parse_date(data["date"].as_str().unwrap()).unwrap();
    let settings: Settings = serde_json::from_value(data["settings"].clone()).unwrap();
    let templates =
        messages::Templates::load(std::path::Path::new("/nonexistent/prospect-fixture")).unwrap();
    for (i, case) in data["cases"].as_array().unwrap().iter().enumerate() {
        let p: Prospect = serde_json::from_value(case["row"].clone()).unwrap();
        let signals: Vec<Signal> = serde_json::from_value(case["signals"].clone()).unwrap();
        let group = serde_json::from_value(case["expected"]["group"].clone()).unwrap();
        let analysis = rules::analyze(&p, &signals, group, false, date, Profile::Automatizacion);
        assert_eq!(
            serde_json::to_value(&analysis).unwrap(),
            case["expected"],
            "analysis case {i}"
        );
        assert_eq!(
            serde_json::to_value(
                messages::first(
                    &p,
                    "whatsapp",
                    &signals,
                    &settings,
                    &templates,
                    Profile::Automatizacion
                )
                .unwrap()
            )
            .unwrap(),
            case["whatsapp"],
            "whatsapp case {i}"
        );
        assert_eq!(
            serde_json::to_value(
                messages::first(
                    &p,
                    "email",
                    &signals,
                    &settings,
                    &templates,
                    Profile::Automatizacion
                )
                .unwrap()
            )
            .unwrap(),
            case["email"],
            "email case {i}"
        );
        for step in 1..=3 {
            assert_eq!(
                serde_json::to_value(messages::followup(
                    &p,
                    step,
                    &settings,
                    &templates,
                    Profile::Automatizacion
                ))
                .unwrap(),
                case["followups"][step as usize - 1],
                "followup case {i}/{step}"
            );
        }
    }
    let rows: Vec<Prospect> = serde_json::from_value(data["group_rows"].clone()).unwrap();
    assert_eq!(
        serde_json::to_value(groups::group(&rows)).unwrap(),
        data["groups"]
    );
    for case in data["texts"].as_array().unwrap() {
        let s = case["input"].as_str().unwrap();
        assert_eq!(rules::clean_text(s), case["clean"].as_str().unwrap());
        assert_eq!(
            serde_json::to_value(rules::detect(s, "https://acme.mx", &mut Default::default()))
                .unwrap(),
            case["signals"]
        );
    }
    for case in data["names"].as_array().unwrap() {
        assert_eq!(
            messages::clean_name(case["raw"].as_str().unwrap()),
            case["clean"].as_str().unwrap()
        );
    }
}
#[test]
fn all_integer_score_rounding_matches_python() {
    let data: Value = serde_json::from_str(include_str!("fixtures/parity.json")).unwrap();
    for fit in 0..=100 {
        let p = Prospect {
            estado: "revisado".into(),
            ..Default::default()
        };
        let (_, items, _) = rules::priority(
            &p,
            fit,
            &[],
            false,
            prospect_core::config::parse_date("2026-10-11").unwrap(),
        );
        let expected = data["rounding"][fit as usize].as_i64().unwrap();
        assert_eq!(items[0].pts, expected);
    }
}
