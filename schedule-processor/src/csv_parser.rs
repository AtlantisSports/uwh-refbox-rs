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
        if let Some((name, value)) = parse_timing_rule_row(&row, &indices) {
            timing_rules_raw.entry(name).or_insert(vec![]).push(value);
        }
    }

    let group_name_map: std::collections::HashMap<String, String> = groups
        .iter()
        .map(|g| (g.short_name.clone(), g.name.clone()))
        .collect();

    for (_, game) in games.iter_mut() {
        for team in [&mut game.light, &mut game.dark] {
            if let Some(SeededBy { group, .. }) = team.seeded_by_mut() {
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
                    if let Some(SeededBy { group, .. }) = team.seeded_by_mut() {
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

pub(crate) fn parse_timing_rule_row(
    row: &csv::StringRecord,
    indices: &Indices,
) -> Option<(String, String)> {
    let name = row.get(indices.timing_rule_name)?.to_string();
    let field = row.get(indices.timing_rule_field)?;
    let value = row.get(indices.timing_rule_value)?;

    if name.is_empty() && field.is_empty() && value.is_empty() {
        return None;
    }

    Some((name, format!("\"{field}\": {}", value.to_lowercase())))
}
