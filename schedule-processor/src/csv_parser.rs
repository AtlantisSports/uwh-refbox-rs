use indexmap::IndexMap;
use log::{debug, error, trace};
use regex::Regex;
use time::{Date, Time, UtcOffset, macros::format_description};
use uwh_common::uwhportal::schedule::*;

lazy_static::lazy_static! {
    static ref WINNER_LOSER_PATTERN: Regex = Regex::new(r"^(L|W)_([A-Za-z0-9_-]+)$").unwrap();
    static ref GROUP_SEED_PATTERN: Regex = Regex::new(r"^((Group|Pod) )?([A-Za-z0-9_-]+) Seed ([0-9]+)$").unwrap();
}

pub fn parse_csv(
    csv: &str,
    offset: UtcOffset,
    event_id: EventId,
) -> Result<Schedule, Box<dyn std::error::Error>> {
    // Parse the input file
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv.as_bytes());

    // Read the header row
    let header: Vec<_> = reader
        .headers()?
        .iter()
        .map(|h| h.trim().to_string())
        .collect();

    let mut detected_parallel = None;

    for p in 1..=5 {
        if header == expected_header(p) {
            detected_parallel = Some(p);
            break;
        }
    }

    let parallel = match detected_parallel {
        Some(d) => {
            debug!("Detected {d} games per row");
            d
        }
        None => {
            error!(
                "Invalid header row:\n{header:?}\nExpected:\n{:?}",
                expected_header(1)
            );
            return Err("Invalid header row".into());
        }
    };

    let indices = Indices::new(parallel);

    // Read the rows
    let mut games = IndexMap::new();
    let mut non_game_entries = Vec::new();
    let mut groups = Vec::new();
    let mut timing_rules_raw = IndexMap::new();
    for (i, row) in reader.records().enumerate() {
        debug!("Parsing row {}", i + 1);

        // Get the row
        let row = row?;

        // Get the game information
        let (row_games, row_non_games) = parse_games(&row, &indices, offset)?;

        // Get the group information
        let group = parse_group(&row, &indices)?;

        // Add the game and group to the schedule
        row_games
            .into_iter()
            .map(|g| games.insert(g.number.clone(), g))
            .for_each(drop);
        if let Some(group) = group {
            groups.push(group);
        }

        // Add the non-game entries to the schedule
        non_game_entries.extend(row_non_games);

        // Get the timing rule information
        // The header is spreadsheet row 1, so data row `i` is row `i + 2`.
        if let Some((name, value)) = parse_timing_rule_row(&row, &indices, i + 2)? {
            timing_rules_raw.entry(name).or_insert(vec![]).push(value);
        }
    }

    let group_name_map: std::collections::HashMap<String, String> = groups
        .iter()
        .map(|g| (g.short_name.clone(), g.name.clone()))
        .collect();

    for (_, game) in games.iter_mut() {
        for team in [&mut game.light, &mut game.dark] {
            if let Some(SeededBy {
                group: Some(group), ..
            }) = team.seeded_by_mut()
            {
                if let Some(name) = group_name_map.get(group.as_str()) {
                    *group = name.clone();
                }
            }
        }
    }

    for group in groups.iter_mut() {
        match group.standings_calculation {
            Some(StandingsCalculation::SwapIfUpset {
                ref mut starting_ranks,
            })
            | Some(StandingsCalculation::SlideIfUpset {
                ref mut starting_ranks,
                ..
            })
            | Some(StandingsCalculation::Exclusion {
                ref mut starting_ranks,
                ..
            }) => {
                for team in starting_ranks.iter_mut() {
                    if let Some(SeededBy {
                        group: Some(group), ..
                    }) = team.seeded_by_mut()
                    {
                        if let Some(name) = group_name_map.get(group.as_str()) {
                            *group = name.clone();
                        }
                    }
                }
            }
            Some(StandingsCalculation::Standard) | None => {}
        }
    }

    let mut timing_rules = vec![];
    for (name, values) in timing_rules_raw {
        let mut rule_string = format!("{{\"name\": \"{name}\", ");
        rule_string.push_str(&values.join(", "));
        rule_string.push('}');
        let rule: TimingRule = serde_json::from_str(&rule_string).map_err(|e| {
            format!("Failed to parse timing rule '{name}' from '{rule_string}': {e}")
        })?;
        timing_rules.push(rule);
    }

    Ok(Schedule {
        event_id,
        games,
        non_game_entries,
        groups,
        timing_rules,
        standings_order: None,
        final_results_order: None,
        referees_by_game_number: None,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Indices {
    pub(crate) date: usize,
    pub(crate) time: usize,
    pub(crate) div: Vec<usize>,
    pub(crate) pod: Vec<usize>,
    pub(crate) rule: Vec<usize>,
    pub(crate) game: Vec<usize>,
    pub(crate) court: Vec<usize>,
    pub(crate) light: Vec<usize>,
    pub(crate) dark: Vec<usize>,
    pub(crate) group: usize,
    pub(crate) short_name: usize,
    pub(crate) group_type: usize,
    pub(crate) standings: usize,
    pub(crate) games: usize,
    pub(crate) excluded_teams: usize,
    pub(crate) starting_standings: usize,
    pub(crate) final_results: usize,
    pub(crate) timing_rule_name: usize,
    pub(crate) timing_rule_field: usize,
    pub(crate) timing_rule_value: usize,
}

impl Indices {
    pub(crate) fn new(parallel: usize) -> Self {
        let mut div = vec![];
        let mut pod = vec![];
        let mut rule = vec![];
        let mut game = vec![];
        let mut court = vec![];
        let mut light = vec![];
        let mut dark = vec![];
        for i in 0..parallel {
            div.push(8 * i + 2);
            pod.push(8 * i + 3);
            rule.push(8 * i + 4);
            game.push(8 * i + 5);
            court.push(8 * i + 6);
            light.push(8 * i + 7);
            dark.push(8 * i + 8);
        }

        Self {
            date: 0,
            time: 1,
            div,
            pod,
            rule,
            game,
            court,
            light,
            dark,
            group: 8 * parallel + 2,
            short_name: 8 * parallel + 3,
            group_type: 8 * parallel + 4,
            standings: 8 * parallel + 5,
            games: 8 * parallel + 7,
            excluded_teams: 8 * parallel + 8,
            starting_standings: 8 * parallel + 9,
            final_results: 8 * parallel + 10,
            timing_rule_name: 8 * parallel + 11,
            timing_rule_field: 8 * parallel + 12,
            timing_rule_value: 8 * parallel + 13,
        }
    }
}

pub(crate) fn expected_header(parallel: usize) -> Vec<&'static str> {
    // Expected csv format (where p is parallel)):
    // Date, Time, (Div, Pod, Rule, Game, Court, Light, Dark,    ,){p} Group, ShortName, Type, Standings,     ,     , Games, Starting Standings, Final Results, Timing Rule Name, Timing Rule Field, Timing Rule Value
    // 0     1      8i+2 8i+3 8i+4  8i+5  8i+6   8i+7   8i+8  8i+9     8p+2   8p+3       8p+4  8p+5       8p+6  8p+7  8p+8   8p+9                8p+10          8p+11             8p+12              8p+13

    let mut expected_header = vec!["Date", "Time"];
    for _ in 0..parallel {
        expected_header.append(&mut vec![
            "Div", "Pod", "Rule", "Game", "Court", "Light", "Dark", "",
        ]);
    }
    expected_header.append(&mut vec![
        "Group",
        "Short Name",
        "Filter Type",
        "Ending Standings",
        "",
        "Games",
        "Excluded Teams",
        "Starting Seedings",
        "Final Results",
        "Timing Rule Name",
        "Timing Rule Field",
        "Timing Rule Value",
    ]);
    expected_header
}

pub(crate) fn parse_games(
    row: &csv::StringRecord,
    indices: &Indices,
    offset: UtcOffset,
) -> Result<(Vec<Game>, Vec<NonGameEntry>), Box<dyn std::error::Error>> {
    if (0..=(indices.group - 1)).all(|i| row.get(i).is_none_or(|cell| cell.trim().is_empty())) {
        return Ok((vec![], vec![]));
    }
    let date_format = format_description!("[year]-[month]-[day]");
    let time_format = format_description!("[hour repr:24 padding:none]:[minute]");
    let date = row.get(indices.date).ok_or("Missing Date cell")?;
    let date = Date::parse(date.trim(), &date_format)
        .map_err(|e| format!("Failed to parse date '{}': {e}", date.trim()))?;
    let time = row.get(indices.time).ok_or("Missing Time cell")?;
    let time = Time::parse(time.trim(), &time_format)
        .map_err(|e| format!("Failed to parse time '{}': {e}", time.trim()))?;
    let start_time = date.with_time(time).assume_offset(offset);

    let mut games = vec![];
    let mut non_games = vec![];
    for i in 0..indices.rule.len() {
        // Get the court
        let court = row
            .get(indices.court[i])
            .ok_or("Missing Court cell")?
            .trim()
            .into();

        // Check if the entry is a non-game entry
        let div = row.get(indices.div[i]).ok_or("Missing Div cell")?.trim();
        let pod = row.get(indices.pod[i]).ok_or("Missing Pod cell")?.trim();
        if div == "---" && pod == "---" {
            let title = row
                .get(indices.light[i])
                .ok_or("Missing Light cell")?
                .trim();
            let description = row.get(indices.dark[i]).and_then(|d| {
                if d.trim().is_empty() {
                    None
                } else {
                    Some(d.trim().to_string())
                }
            });

            let non_game = NonGameEntry {
                title: title.to_string(),
                description,
                start_time,
                court: Some(court),
                end_time: None,
            };

            non_games.push(non_game);
            continue;
        }

        // Get the timing rule
        let timing_rule = row
            .get(indices.rule[i])
            .ok_or("Missing Rule cell")?
            .trim()
            .into();

        // Get the game number
        let number = row.get(indices.game[i]).ok_or("Missing Game cell")?.trim();
        let number = number
            .parse()
            .map_err(|e| format!("Failed to parse game number '{number}': {e}"))?;

        // Get the teams
        let light = row
            .get(indices.light[i])
            .ok_or("Missing Light cell")?
            .trim();
        let light = match parse_team(light) {
            Ok(team) => team,
            Err(e) => {
                error!("Failed to parse Light team ({light:?}): {e}");
                return Err(e);
            }
        };
        let dark = row.get(indices.dark[i]).ok_or("Missing Dark cell")?.trim();
        let dark = match parse_team(dark) {
            Ok(team) => team,
            Err(e) => {
                error!("Failed to parse Dark team ({dark:?}): {e}");
                return Err(e);
            }
        };

        // Create the game
        let game = Game {
            timing_rule,
            court,
            number,
            dark,
            light,
            start_time,
            description: None,
            referee_assignments: None,
        };
        debug!("Parsed game: {}", game.number);
        trace!("    {game:?}");
        games.push(game);
    }

    Ok((games, non_games))
}

pub(crate) fn parse_team(description: &str) -> Result<ScheduledTeam, Box<dyn std::error::Error>> {
    if let Some(captures) = WINNER_LOSER_PATTERN.captures(description) {
        let number = captures.get(2).ok_or("Missing game number")?.as_str();
        let team = match captures.get(1).ok_or("Missing W/L")?.as_str() {
            "L" => ScheduledTeam::new_loser_of(number),
            "W" => ScheduledTeam::new_winner_of(number),
            _ => unreachable!(),
        };

        return Ok(team);
    }

    if let Some(captures) = GROUP_SEED_PATTERN.captures(description) {
        let group = captures.get(3).ok_or("Missing group name")?.as_str();
        let seed = captures.get(4).ok_or("Missing seed")?.as_str();
        let seed = seed
            .parse()
            .map_err(|e| format!("Failed to parse seed '{seed}': {e}"))?;
        return Ok(ScheduledTeam::new_seeded_by(seed, group));
    }

    Ok(ScheduledTeam::new_pending_assignment_name(description))
}

pub(crate) fn parse_final_results_team(
    description: &str,
) -> Result<ResultOf, Box<dyn std::error::Error>> {
    if let Some(captures) = WINNER_LOSER_PATTERN.captures(description) {
        let game_number = captures.get(2).ok_or("Missing game number")?.as_str();
        let game_number = game_number
            .parse()
            .map_err(|e| format!("Failed to parse game number '{game_number}': {e}"))?;
        let team = match captures.get(1).ok_or("Missing W/L")?.as_str() {
            "L" => ResultOf::Loser { game_number },
            "W" => ResultOf::Winner { game_number },
            _ => unreachable!(),
        };

        return Ok(team);
    }

    Err("Invalid final results team description".into())
}

pub(crate) fn parse_group(
    row: &csv::StringRecord,
    indices: &Indices,
) -> Result<Option<Group>, String> {
    let name = row
        .get(indices.group)
        .ok_or("Missing Group cell")?
        .trim()
        .to_string();

    if name.is_empty() {
        return Ok(None);
    }

    debug!("Parsing group: {name}");

    let short_name = row
        .get(indices.short_name)
        .ok_or("Missing Short Name cell")?
        .trim()
        .to_string();

    let group_type = row
        .get(indices.group_type)
        .ok_or("Missing Type cell")?
        .trim();
    let group_type = match group_type.to_lowercase().as_str().trim() {
        "division" => Some(GroupType::Division),
        "pod" | "group" | "group/pod" | "pod/group" => Some(GroupType::Pod),
        "nonfiltered" | "non filtered" | "non-filtered" | "unfiltered" => None,
        _ => {
            error!("Invalid Group Type: {group_type}");
            return Err("Invalid Group Type".into());
        }
    };

    let standings_type = row
        .get(indices.standings)
        .ok_or("Missing Standings cell")?
        .trim()
        .to_lowercase();

    let game_numbers = row
        .get(indices.games)
        .ok_or("Missing Games cell")?
        .trim()
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                Some(
                    s.parse::<GameNumber>()
                        .map_err(|_| "Failed to parse game number"),
                )
            }
        })
        .collect::<Result<Vec<GameNumber>, _>>()
        .map_err(|_| "Failed to parse Games cell")?;

    let starting_standings = row
        .get(indices.starting_standings)
        .ok_or("Missing Starting Standings cell")?
        .trim();
    let starting_standings = starting_standings
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() { None } else { Some(s) }
        })
        .map(parse_team)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to parse Starting Standings '{starting_standings}': {e}"))
        .map(|list| if list.is_empty() { None } else { Some(list) })?;

    let excluded_teams = row
        .get(indices.excluded_teams)
        .ok_or("Missing Excluded Teams cell")?
        .trim()
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() { None } else { Some(s) }
        })
        .map(parse_team)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to parse Excluded Teams: {e}"))
        .map(|list| if list.is_empty() { None } else { Some(list) })?;

    let standings_calculation = match standings_type.as_str() {
        "none" | "final" => None,
        "standard" => Some(StandingsCalculation::Standard),
        "swap if upset" => {
            if let Some(starting_ranks) = starting_standings {
                Some(StandingsCalculation::SwapIfUpset { starting_ranks })
            } else {
                return Err(
                    "Starting Standings must be provided for Swap If Upset standings calculation"
                        .into(),
                );
            }
        }
        "slide up if upset" => {
            if let Some(starting_ranks) = starting_standings {
                Some(StandingsCalculation::SlideIfUpset {
                    starting_ranks,
                    slide_direction: SlideDirection::Up,
                })
            } else {
                return Err("Starting Standings must be provided for Slide Up If Upset standings calculation".into());
            }
        }
        "slide down if upset" => {
            if let Some(starting_ranks) = starting_standings {
                Some(StandingsCalculation::SlideIfUpset {
                    starting_ranks,
                    slide_direction: SlideDirection::Down,
                })
            } else {
                return Err("Starting Standings must be provided for Slide Down If Upset standings calculation".into());
            }
        }
        "exclusion" | "excluded" => {
            if !game_numbers.is_empty() {
                return Err(format!(
                    "Group {name} with Exclusion standings calculation cannot have game numbers specified. It currently has: {:?}",
                    game_numbers
                ));
            }
            if let (Some(excluded_teams), Some(starting_ranks)) =
                (excluded_teams, starting_standings)
            {
                Some(StandingsCalculation::Exclusion {
                    excluded_teams,
                    starting_ranks,
                })
            } else {
                return Err("Excluded Teams and Starting Standings must be provided for Exclusion standings calculation".into());
            }
        }
        _ => return Err(format!("Invalid Standings cell: {standings_type:?}")),
    };

    let final_results = row
        .get(indices.final_results)
        .ok_or("Missing Final Results column")?
        .trim();
    let final_results = match final_results {
        "Standings" => Some(FinalResults::Standings),
        list => {
            if list.is_empty() {
                None
            } else {
                Some(FinalResults::ListOfGames {
                    list_of_games: list
                        .split(',')
                        .filter_map(|s| {
                            let s = s.trim();
                            if s.is_empty() { None } else { Some(s) }
                        })
                        .map(parse_final_results_team)
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| format!("Failed to parse Final Results cell: {e}"))?,
                })
            }
        }
    };

    let group = Group {
        name,
        short_name,
        group_type,
        final_results,
        game_numbers,
        standings_calculation,
    };

    debug!("Parsed group: {}", group.name);
    trace!("    {group:?}");
    Ok(Some(group))
}

/// The JSON field names that may appear in the spreadsheet's "Timing Rule
/// Field" column.
///
/// The cell is pasted into the assembled JSON object as the key, and
/// `TimingRule` deliberately does not use `deny_unknown_fields` — refbox reads
/// the same type from the Portal and must keep tolerating fields the Portal
/// adds later. Here that leniency hides operator typos: a name serde does not
/// recognise is dropped. For the twelve required fields the rule then fails as
/// a missing field, which is loud but cryptic; for the two carrying
/// `#[serde(default)]` — `last2minStopTime` and `gameBlock` — the value simply
/// vanishes with no error at all. So the spelling is checked here, against a
/// spreadsheet, where the operator can act on what they are told.
///
/// `name` is deliberately NOT in this list. It is a field of `TimingRule`, but
/// it arrives from the separate "Timing Rule Name" column; in the field column
/// it can only ever be a mistake, so it gets its own message rather than being
/// advertised as valid. `timing_rule_field_names_match_the_type` keeps this
/// list honest against `TimingRule` itself.
const TIMING_RULE_FIELDS: [&str; 15] = [
    "teamTimeoutCount",
    "teamTimeoutsCountedPerHalf",
    "overtimeAllowed",
    "suddenDeathAllowed",
    "singlePeriod",
    "last2minStopTime",
    "halfPlayDuration",
    "halfTimeDuration",
    "teamTimeoutDuration",
    "overtimeHalfPlayDuration",
    "overtimeHalfTimeDuration",
    "preOvertimeBreak",
    "preSuddenDeathDuration",
    "minimumBreak",
    "gameBlock",
];

/// Read one row's timing-rule cells.
///
/// `spreadsheet_row` is the row number as the operator sees it in the sheet
/// (the header is row 1), so an error can send them straight to the cell.
pub(crate) fn parse_timing_rule_row(
    row: &csv::StringRecord,
    indices: &Indices,
    spreadsheet_row: usize,
) -> Result<Option<(String, String)>, String> {
    let (Some(name), Some(field), Some(value)) = (
        row.get(indices.timing_rule_name),
        row.get(indices.timing_rule_field),
        row.get(indices.timing_rule_value),
    ) else {
        return Ok(None);
    };

    // The csv reader does not trim, and every other parser in this file trims
    // its own cells. An operator cannot see a trailing space, so a cell that
    // differs only by whitespace must not be treated as a different name.
    let (name, field, value) = (name.trim(), field.trim(), value.trim());

    if name.is_empty() && field.is_empty() && value.is_empty() {
        return Ok(None);
    }

    let empty_columns: Vec<&str> = [
        ("'Timing Rule Name'", name),
        ("'Timing Rule Field'", field),
        ("'Timing Rule Value'", value),
    ]
    .into_iter()
    .filter(|(_, cell)| cell.is_empty())
    .map(|(column, _)| column)
    .collect();

    if !empty_columns.is_empty() {
        return Err(format!(
            "The timing rule on spreadsheet row {spreadsheet_row} is only partly filled in — \
             {} left empty. A timing rule row needs 'Timing Rule Name', 'Timing Rule Field' \
             and 'Timing Rule Value' all filled in, or all three left blank.",
            empty_columns.join(" and ")
        ));
    }

    if field == "name" {
        return Err(format!(
            "The timing rule on spreadsheet row {spreadsheet_row} puts 'name' in the \
             'Timing Rule Field' column. A rule's name belongs in the 'Timing Rule Name' \
             column, which on this row already says '{name}'."
        ));
    }

    if !TIMING_RULE_FIELDS.contains(&field) {
        return Err(format!(
            "Timing rule '{name}' has an unrecognised field name '{field}' on spreadsheet \
             row {spreadsheet_row}. Check the spelling of that cell in the 'Timing Rule Field' \
             column. Expected one of: {}",
            TIMING_RULE_FIELDS.join(", ")
        ));
    }

    Ok(Some((
        name.to_string(),
        format!("\"{field}\": {}", value.to_lowercase()),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const FIXTURE: &str = include_str!("../tests/fixtures/timing-rule-fields.csv");

    /// Build a row with just the three timing-rule cells filled in, using the
    /// single-game-per-row column layout.
    fn timing_rule_row(name: &str, field: &str, value: &str) -> (csv::StringRecord, Indices) {
        let indices = Indices::new(1);
        let mut cells = vec![String::new(); indices.timing_rule_value + 1];
        cells[indices.timing_rule_name] = name.to_string();
        cells[indices.timing_rule_field] = field.to_string();
        cells[indices.timing_rule_value] = value.to_string();
        (csv::StringRecord::from(cells), indices)
    }

    fn parse(name: &str, field: &str, value: &str) -> Result<Option<(String, String)>, String> {
        let (row, indices) = timing_rule_row(name, field, value);
        parse_timing_rule_row(&row, &indices, 7)
    }

    #[test]
    fn known_field_name_is_accepted() {
        assert_eq!(
            parse("RR", "gameBlock", "1800"),
            Ok(Some(("RR".to_string(), "\"gameBlock\": 1800".to_string())))
        );
    }

    #[test]
    fn misspelled_field_name_is_rejected_naming_rule_column_and_row() {
        for misspelling in ["Game Block", "gameblock", "GameBlock", "minimum_break"] {
            let err = match parse("RR", misspelling, "1800") {
                Err(e) => e,
                other => panic!("'{misspelling}' should be rejected, got {other:?}"),
            };
            assert!(err.contains("RR"), "should name the rule: {err}");
            assert!(
                err.contains(&format!("'{misspelling}'")),
                "should quote the offending column: {err}"
            );
            assert!(err.contains("row 7"), "should name the row: {err}");
        }
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_rather_than_rejected() {
        // A trailing space is invisible in a spreadsheet. Rejecting it would
        // print two strings the operator cannot tell apart.
        assert_eq!(
            parse(" RR ", "gameBlock ", " 1800 "),
            Ok(Some(("RR".to_string(), "\"gameBlock\": 1800".to_string())))
        );
    }

    #[test]
    fn name_in_the_field_column_points_at_the_name_column() {
        // Accepting it would build {"name": "RR", "name": Round Robin} and fail
        // as a raw JSON syntax error, which is what this change exists to stop.
        let err = parse("RR", "name", "Round Robin").expect_err("should be rejected");
        assert!(err.contains("'Timing Rule Name'"), "{err}");
        assert!(
            !TIMING_RULE_FIELDS.contains(&"name"),
            "'name' must not be advertised as a valid field column value"
        );
    }

    #[test]
    fn partly_filled_row_is_rejected_naming_the_empty_columns() {
        for (name, field, value, expected) in [
            ("RR", "gameBlock", "", "'Timing Rule Value'"),
            ("RR", "", "1800", "'Timing Rule Field'"),
            ("", "gameBlock", "1800", "'Timing Rule Name'"),
            ("", "", "1800", "'Timing Rule Name' and 'Timing Rule Field'"),
        ] {
            let err = match parse(name, field, value) {
                Err(e) => e,
                other => panic!("({name:?},{field:?},{value:?}) should be rejected: {other:?}"),
            };
            assert!(
                err.contains(expected),
                "expected {expected} named in: {err}"
            );
            assert!(err.contains("row 7"), "should name the row: {err}");
        }
    }

    #[test]
    fn wholly_empty_row_is_not_a_timing_rule_row() {
        assert_eq!(parse("", "", ""), Ok(None));
        // Whitespace-only cells are a blank row too, not a partly filled one.
        assert_eq!(parse("  ", " ", "\t"), Ok(None));
    }

    #[test]
    fn row_too_short_for_the_timing_rule_columns_is_skipped() {
        // Guards this function's contract only: `parse_csv` builds a
        // non-flexible reader, so a short record fails earlier as
        // `UnequalLengths` and never reaches here.
        let indices = Indices::new(1);
        let row = csv::StringRecord::from(vec!["2026-06-26", "09:00"]);
        assert_eq!(parse_timing_rule_row(&row, &indices, 7), Ok(None));
    }

    #[test]
    fn timing_rule_field_names_match_the_type() {
        // Not circular: this asks `TimingRule` itself what it serialises, so
        // adding, removing or renaming a field there fails here rather than
        // silently reintroducing the drop this check exists to prevent.
        let rule = TimingRule {
            name: "RR".to_string(),
            team_timeout_count: 1,
            team_timeouts_counted_per_half: false,
            overtime_allowed: false,
            sudden_death_allowed: false,
            single_period: false,
            last_2_min_stop_time: false,
            half_play_duration: Duration::from_secs(720),
            half_time_duration: Duration::from_secs(180),
            team_timeout_duration: Duration::from_secs(60),
            ot_half_play_duration: Duration::from_secs(300),
            ot_half_time_duration: Duration::from_secs(60),
            pre_overtime_break: Duration::from_secs(180),
            pre_sudden_death_duration: Duration::from_secs(60),
            minimum_break: Duration::from_secs(240),
            // Must be Some: the field is `skip_serializing_if = "Option::is_none"`.
            game_block: Some(Duration::from_secs(1920)),
        };
        let serde_json::Value::Object(map) = serde_json::to_value(&rule).unwrap() else {
            panic!("a TimingRule should serialise to a JSON object");
        };

        let mut serialised: Vec<&str> = map.keys().map(String::as_str).collect();
        serialised.sort_unstable();
        let mut accepted: Vec<&str> = TIMING_RULE_FIELDS.to_vec();
        accepted.push("name"); // arrives from its own spreadsheet column
        accepted.sort_unstable();

        assert_eq!(
            serialised, accepted,
            "TIMING_RULE_FIELDS has drifted from TimingRule's own JSON names"
        );
    }

    #[test]
    fn a_real_sheet_parses_and_one_misspelt_cell_stops_it() {
        let event_id = EventId::from_full("events/fixture").unwrap();

        let schedule = parse_csv(FIXTURE, UtcOffset::UTC, event_id.clone())
            .expect("the fixture sheet should parse");
        let rule = schedule
            .timing_rules
            .iter()
            .find(|r| r.name == "RR")
            .expect("fixture defines rule RR");
        assert_eq!(rule.game_block, Some(Duration::from_secs(1920)));
        assert!(rule.last_2_min_stop_time);

        // Exactly one cell differs: the Game Block setting name.
        let misspelt = FIXTURE.replace(",gameBlock,", ",Game Block,");
        assert_ne!(
            misspelt, FIXTURE,
            "the fixture must contain a gameBlock row"
        );
        let err = parse_csv(&misspelt, UtcOffset::UTC, event_id)
            .expect_err("a misspelt setting name should stop the whole sheet")
            .to_string();
        assert!(err.contains("'Game Block'"), "{err}");
        assert!(err.contains("RR"), "{err}");
    }
}
