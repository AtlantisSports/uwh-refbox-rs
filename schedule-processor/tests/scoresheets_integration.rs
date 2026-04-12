/// Integration test for scoresheet generation using a local mock JSON schedule.
///
/// This test bypasses the portal entirely by passing `schedule_override` directly.
/// Network calls for teams and referee names will fail gracefully (empty maps),
/// so team names fall back to IDs and referee fields are blank.
///
/// PDF generation requires Chrome/Chromium to be installed. If not present,
/// the test still passes as long as HTML files were generated.
use schedule_processor::scoresheets::{RenderInputs, SheetStyle, generate_scoresheets_for_event};
use std::time::Duration;
use uwh_common::uwhportal::UwhPortalClient;
use uwh_common::uwhportal::schedule::{DateRange, Event, EventId, Schedule};

const MOCK_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/Mock Schedules for testing/uwh-tournament-portal-schedule.json"
));

fn load_schedule() -> Schedule {
    let mut val: serde_json::Value = serde_json::from_str(MOCK_JSON).expect("mock JSON must parse");

    // The JSON file has no eventId field; inject a fake one
    val["eventId"] = serde_json::Value::String("events/test-mock-event".to_string());

    // The mock file has `games` as an array; GameList deserializes from an object keyed by number
    if let Some(arr) = val["games"].as_array().cloned() {
        let map: serde_json::Map<String, serde_json::Value> = arr
            .into_iter()
            .filter_map(|g| {
                let num = g["number"].as_str()?.to_string();
                Some((num, g))
            })
            .collect();
        val["games"] = serde_json::Value::Object(map);
    }

    serde_json::from_value(val).expect("Schedule must deserialize from patched mock JSON")
}

fn fake_event() -> Event {
    Event {
        id: EventId::from_partial("test-mock-event"),
        name: "Mock Tournament 2026".to_string(),
        slug: "mock-2026".to_string(),
        date_range: DateRange {
            start: time::macros::datetime!(2026-06-27 00:00 UTC),
            end: time::macros::datetime!(2026-06-29 23:59 UTC),
        },
        teams: None,
        schedule: None,
        courts: None,
    }
}

fn dead_client() -> UwhPortalClient {
    // Points at a port that should be unreachable; all network calls will fail
    UwhPortalClient::new(
        "http://localhost:1",
        None,
        false,
        Duration::from_millis(200),
    )
    .expect("client construction must not fail")
}

#[tokio::test]
async fn test_generate_scoresheets_detailed() {
    let schedule = load_schedule();
    let event = fake_event();
    let mut client = dead_client();

    let tmp = tempfile::tempdir().expect("temp dir");
    let output_dir = tmp.path().to_path_buf();

    let inputs = RenderInputs {
        left_logo: None,
        right_logo: None,
        output_dir: output_dir.clone(),
        style: SheetStyle::Detailed,
        prefer_portal_officials: false,
    };

    let result =
        generate_scoresheets_for_event(&mut client, &event, inputs, None, None, Some(schedule))
            .await;

    assert!(result.is_ok(), "scoresheet generation failed: {:?}", result);

    let entries: Vec<_> = std::fs::read_dir(&output_dir)
        .expect("output dir readable")
        .filter_map(|e| e.ok())
        .collect();

    // Combined HTML must always be produced
    assert!(
        output_dir.join("scoresheets-all.html").exists(),
        "expected scoresheets-all.html to be produced"
    );

    // PDF is produced if Chrome is available
    let pdfs: Vec<_> = entries
        .iter()
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("pdf"))
        .collect();
    if pdfs.is_empty() {
        println!("No PDF produced (Chrome not available or failed).");
    } else {
        println!("Generated {} PDF(s) in {:?}", pdfs.len(), output_dir);
    }
}

#[tokio::test]
async fn test_generate_scoresheets_simple() {
    let schedule = load_schedule();
    let event = fake_event();
    let mut client = dead_client();

    let tmp = tempfile::tempdir().expect("temp dir");
    let output_dir = tmp.path().to_path_buf();

    let inputs = RenderInputs {
        left_logo: None,
        right_logo: None,
        output_dir: output_dir.clone(),
        style: SheetStyle::Simple,
        prefer_portal_officials: false,
    };

    let result =
        generate_scoresheets_for_event(&mut client, &event, inputs, None, None, Some(schedule))
            .await;

    assert!(result.is_ok(), "scoresheet generation failed: {:?}", result);

    let entries: Vec<_> = std::fs::read_dir(&output_dir)
        .expect("output dir readable")
        .filter_map(|e| e.ok())
        .collect();

    // Combined HTML must always be produced
    assert!(
        output_dir.join("scoresheets-all.html").exists(),
        "expected scoresheets-all.html to be produced"
    );

    let pdfs: Vec<_> = entries
        .iter()
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("pdf"))
        .collect();
    if pdfs.is_empty() {
        println!("No PDF produced (Chrome not available or failed).");
    } else {
        println!("Generated {} PDF(s) in {:?}", pdfs.len(), output_dir);
    }
}
