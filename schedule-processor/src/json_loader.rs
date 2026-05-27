use time::UtcOffset;
use uwh_common::uwhportal::schedule::*;

pub fn parse_json(
    json: &str,
    // Not used for JSON input — portal-format JSON timestamps are already
    // UTC. The parameter exists for signature symmetry with `parse_csv`,
    // which needs the event-local offset to interpret naive local times.
    _offset: UtcOffset,
    event_id: EventId,
) -> Result<Schedule, Box<dyn std::error::Error>> {
    let sendable: SendableSchedule = serde_json::from_str(json)?;
    Ok((sendable, event_id).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HAPPY_PATH_JSON: &str = r#"{
        "games": [
            {
                "number": "1",
                "court": "1",
                "startsOn": "2026-06-26T23:30:00Z",
                "timingRule": {"name": "RR"},
                "light": {"pendingAssignmentName": "Team A"},
                "dark": {"pendingAssignmentName": "Team B"}
            }
        ],
        "nonGameEntries": [],
        "groups": [
            {
                "name": "Test Group",
                "shortName": "TG",
                "type": "Division",
                "gameNumbers": ["1"],
                "standingsCalculation": {"type": "Standard"},
                "finalResultsCalculation": null
            }
        ],
        "timingRules": [
            {
                "name": "RR",
                "teamTimeoutCount": 1,
                "teamTimeoutsCountedPerHalf": true,
                "overtimeAllowed": false,
                "suddenDeathAllowed": false,
                "last2minStopTime": false,
                "halfPlayDuration": 600,
                "halfTimeDuration": 120,
                "teamTimeoutDuration": 60,
                "overtimeHalfPlayDuration": 0,
                "overtimeHalfTimeDuration": 0,
                "preOvertimeBreak": 0,
                "preSuddenDeathDuration": 0,
                "minimumBreak": 180
            }
        ],
        "standingsOrder": [{"name": "Test Group"}],
        "finalResultsOrder": null
    }"#;

    #[test]
    fn parses_happy_path_json() {
        let event_id = EventId::from_partial("test-event");
        let schedule = parse_json(HAPPY_PATH_JSON, UtcOffset::UTC, event_id).unwrap();
        assert_eq!(schedule.games.len(), 1);
        assert!(
            schedule.games.contains_key("1"),
            "games map should be keyed by Game.number"
        );
        assert_eq!(schedule.groups.len(), 1);
        assert_eq!(schedule.groups[0].name, "Test Group");
        assert_eq!(schedule.timing_rules.len(), 1);
        assert_eq!(schedule.standings_order.as_ref().unwrap().len(), 1);
        assert!(schedule.final_results_order.is_none());
    }

    #[test]
    fn injects_event_id() {
        let event_id = EventId::from_partial("my-event");
        let schedule = parse_json(HAPPY_PATH_JSON, UtcOffset::UTC, event_id.clone()).unwrap();
        assert_eq!(schedule.event_id, event_id);
    }

    #[test]
    fn missing_required_field_surfaces_serde_error() {
        // games omitted
        let bad_json = r#"{
            "nonGameEntries": [],
            "groups": [],
            "timingRules": []
        }"#;
        let event_id = EventId::from_partial("test-event");
        let err = parse_json(bad_json, UtcOffset::UTC, event_id)
            .expect_err("expected parse to fail with missing field");
        assert!(
            err.to_string().contains("games"),
            "expected error to mention `games`, got: {err}"
        );
    }
}
