use time::UtcOffset;
use uwh_common::uwhportal::schedule::*;

pub fn parse_json(
    json: &str,
    // Not used for JSON input — portal-format JSON timestamps are already UTC. The parameter
    // exists for signature symmetry with `parse_csv`, which needs the event-local offset to
    // interpret naive local times.
    _offset: UtcOffset,
    event_id: EventId,
) -> Result<Schedule, Box<dyn std::error::Error>> {
    let sendable: SendableSchedule = serde_json::from_str(json)?;
    Ok((sendable, event_id).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

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
        // `games` omitted
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

    /// A real 71-game tournament export, with only the club names replaced. Every structural
    /// relationship — game numbers, courts, times, seedings, group membership — is untouched,
    /// which is what makes the schedule checks below meaningful.
    const REAL_SHAPE_JSON: &str =
        include_str!("../tests/fixtures/portal-schedule-with-finals.json");

    #[test]
    fn real_shape_schedule_parses_and_passes_all_checks() {
        let event_id = EventId::from_partial("test-event");
        let schedule = parse_json(REAL_SHAPE_JSON, UtcOffset::UTC, event_id)
            .expect("real-shape portal export should parse");
        assert_eq!(schedule.games.len(), 71);
        crate::schedule_checks::run_schedule_checks(&schedule)
            .expect("schedule checks should pass");
    }

    #[test]
    fn real_shape_schedule_offers_only_real_teams_for_matching() {
        // Regression guard for the JSON-only defect fixed in the team-matching step: finals
        // slots carry a human label alongside their seeding and must not be offered as teams.
        // Measured on the source export: 35 names before the fix, 27 after, 8 excluded.
        let event_id = EventId::from_partial("test-event");
        let schedule = parse_json(REAL_SHAPE_JSON, UtcOffset::UTC, event_id).unwrap();

        let all_pending: BTreeSet<String> = schedule
            .games
            .values()
            .flat_map(|g| [g.light.clone(), g.dark.clone()])
            .filter_map(|t| t.pending().map(|n| n.to_string()))
            .collect();
        let offered: BTreeSet<String> = schedule
            .games
            .values()
            .flat_map(|g| [g.light.clone(), g.dark.clone()])
            .filter_map(|t| crate::unassigned_name(&t).map(|n| n.to_string()))
            .collect();

        assert_eq!(
            all_pending.len(),
            35,
            "unfiltered list changed: {all_pending:?}"
        );
        assert_eq!(offered.len(), 27, "offered for matching: {offered:?}");
        let excluded: Vec<&String> = all_pending.difference(&offered).collect();
        assert_eq!(
            excluded,
            [
                "Pool A 1st",
                "Pool A 2nd",
                "Pool A 3rd",
                "Pool A 4th",
                "Pool B 1st",
                "Pool B 2nd",
                "Pool B 3rd",
                "Pool B 4th",
            ]
            .iter()
            .collect::<Vec<_>>(),
            "the fix must exclude exactly the bracket placeholders and nothing else"
        );
    }
}
