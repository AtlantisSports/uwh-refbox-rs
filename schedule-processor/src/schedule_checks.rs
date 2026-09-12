use indexmap::{IndexMap, IndexSet};
use log::{error, warn};
use std::time::Duration;
use time::OffsetDateTime;
use uwh_common::uwhportal::schedule::*;

// TODO: Validate final results calculation ( i.e. that the group exists or that the games exist )

pub fn run_schedule_checks(schedule: &Schedule) -> Result<(), Box<dyn std::error::Error>> {
    check_unique_game_numbers(schedule)?;
    check_for_multiple_standings(schedule);
    check_game_group_types(schedule);
    check_groups_have_games(schedule)?;
    check_unique_timing_rule_names(schedule)?;
    check_flag_gated_durations(schedule)?;
    check_game_timing_rules(schedule)?;
    check_game_overlap(schedule)?;
    check_same_team_in_game(schedule)?;
    check_unique_group_names(schedule)?;
    check_group_standings(schedule)?;
    let _ = check_final_results(schedule); // Ignore this error and complete the rest of the checks
    check_game_references(schedule)?;
    Ok(())
}

fn check_unique_game_numbers(schedule: &Schedule) -> Result<(), Box<dyn std::error::Error>> {
    let mut game_numbers: IndexSet<GameNumber> = IndexSet::new();
    let mut found_duplicate = false;

    for (_, game) in &schedule.games {
        if game_numbers.contains(&game.number) {
            error!("Duplicate game number found: {}", game.number);
            found_duplicate = true;
        } else {
            game_numbers.insert(game.number.clone());
        }
    }

    if found_duplicate {
        Err("Found duplicate game numbers".into())
    } else {
        Ok(())
    }
}

fn check_for_multiple_standings(schedule: &Schedule) {
    let mut game_groups: IndexMap<GameNumber, Vec<&Group>> = IndexMap::new();

    for group in &schedule.groups {
        for game in &group.game_numbers {
            game_groups.entry(game.clone()).or_default().push(group);
        }
    }

    for (game, groups) in game_groups {
        if groups.len() > 1 {
            let standings_groups: Vec<_> = groups
                .iter()
                .filter_map(|g| {
                    if g.standings_calculation.is_some() {
                        Some(&g.name)
                    } else {
                        None
                    }
                })
                .collect();
            if standings_groups.len() > 1 {
                warn!(
                    "Game {} is part of multiple groups that calculate standings: {:?}",
                    game, standings_groups
                );
            }
        }
    }
}

fn check_game_group_types(schedule: &Schedule) {
    let mut games_in_pods: IndexSet<GameNumber> = IndexSet::new();
    let mut games_in_divisions: IndexSet<GameNumber> = IndexSet::new();

    for group in &schedule.groups {
        match group.group_type {
            Some(GroupType::Pod) => {
                for game in &group.game_numbers {
                    games_in_pods.insert(game.clone());
                }
            }
            Some(GroupType::Division) => {
                for game in &group.game_numbers {
                    games_in_divisions.insert(game.clone());
                }
            }
            None => {}
        }
    }

    let mut games_not_in_pods: Vec<GameNumber> = Vec::new();
    let mut games_not_in_divisions: Vec<GameNumber> = Vec::new();

    for (_, game) in &schedule.games {
        if !games_in_pods.contains(&game.number) {
            games_not_in_pods.push(game.number.clone());
        }
        if !games_in_divisions.contains(&game.number) {
            games_not_in_divisions.push(game.number.clone());
        }
    }

    if !games_not_in_pods.is_empty() {
        warn!(
            "Games not in any Pod group: {}",
            format_game_numbers(games_not_in_pods)
        );
    }

    if !games_not_in_divisions.is_empty() {
        warn!(
            "Games not in any Division group: {}",
            format_game_numbers(games_not_in_divisions)
        );
    }
}

fn check_groups_have_games(schedule: &Schedule) -> Result<(), Box<dyn std::error::Error>> {
    let mut groups_have_games = true;

    for group in &schedule.groups {
        if group.game_numbers.is_empty()
            && !matches!(
                group.standings_calculation,
                Some(StandingsCalculation::Exclusion { .. })
            )
        {
            groups_have_games = false;
            error!("Group {} has no games assigned", group.name);
        }
    }

    if !groups_have_games {
        Err("Found a group missing game assignments".into())
    } else {
        Ok(())
    }
}

fn format_game_numbers(mut numbers: Vec<GameNumber>) -> String {
    numbers.sort_unstable_by(|a, b| {
        let (prefix_a, num_a) = split_prefix_number(a);
        let (prefix_b, num_b) = split_prefix_number(b);
        num_a.cmp(&num_b).then(prefix_a.cmp(&prefix_b))
    });

    let mut result = String::new();
    let mut start = &numbers[0];
    let mut end = start;

    for num in numbers.iter().skip(1) {
        let (prefix_end, num_end) = split_prefix_number(end);
        let (prefix_curr, num_curr) = split_prefix_number(num);

        if prefix_curr != prefix_end || num_curr != num_end + 1 {
            result.push_str(&format_range(start, end));
            start = num;
        }
        end = num;
    }

    result.push_str(&format_range(start, end));
    if result.ends_with(", ") {
        let new_length = result.len() - 2;
        result.truncate(new_length);
    }
    result
}

fn format_range(start: &String, end: &String) -> String {
    if start == end {
        format!("{}, ", start)
    } else {
        format!("{}-{}, ", start, end)
    }
}

fn split_prefix_number(s: &str) -> (String, i32) {
    let mut non_digit_parts = String::new();
    let mut numeric_part = String::new();
    let mut is_numeric = false;

    for c in s.chars() {
        if c.is_ascii_digit() {
            is_numeric = true;
            numeric_part.push(c);
        } else {
            if is_numeric {
                // Reset numeric part if non-digit appears after digits
                numeric_part.clear();
                is_numeric = false;
            }
            non_digit_parts.push(c);
        }
    }

    let number = numeric_part.parse::<i32>().unwrap_or(-1); // Use -1 for non-numeric cases
    (non_digit_parts, number)
}

fn check_unique_timing_rule_names(schedule: &Schedule) -> Result<(), Box<dyn std::error::Error>> {
    let mut timing_rule_names = IndexSet::new();

    let mut duplicates_found = false;

    for rule in &schedule.timing_rules {
        if !timing_rule_names.insert(rule.name.clone()) {
            error!("Duplicate timing rule name found: {}", rule.name);
            duplicates_found = true;
        }
    }

    if duplicates_found {
        Err("Found duplicate timing rule names".into())
    } else {
        Ok(())
    }
}

fn check_game_timing_rules(schedule: &Schedule) -> Result<(), Box<dyn std::error::Error>> {
    let timing_rules_set: IndexSet<_> = schedule
        .timing_rules
        .iter()
        .map(|rule| rule.name.clone())
        .collect();
    let mut failed_matches = Vec::new();

    for (_, game) in &schedule.games {
        if !timing_rules_set.contains(&game.timing_rule) {
            error!(
                "Game {} has a timing rule that does not match any in the timing_rules vec: {}",
                game.number, game.timing_rule
            );
            failed_matches.push(game.number.clone());
        }
    }

    if !failed_matches.is_empty() {
        Err("Found Games with invalid timing rules".into())
    } else {
        Ok(())
    }
}

fn check_game_overlap(schedule: &Schedule) -> Result<(), Box<dyn std::error::Error>> {
    let mut court_games: IndexMap<String, Vec<(&Game, Duration)>> = IndexMap::new();
    let occupied_time_map = calculate_occupied_times(schedule);

    for (_, game) in &schedule.games {
        let occupied_time = *occupied_time_map.get(&game.timing_rule).unwrap();
        court_games
            .entry(game.court.clone())
            .or_default()
            .push((game, occupied_time));
    }

    for games in court_games.values_mut() {
        games.sort_by_key(|a| a.0.start_time);
    }

    let mut games_overlap = false;

    for games in court_games.values() {
        for (i, (game, occupied_time)) in games.iter().enumerate() {
            let end_time = game.start_time + *occupied_time;

            for (other_game, _) in games.iter().skip(i + 1) {
                if other_game.start_time < end_time {
                    error!(
                        "Game {} overlaps with game {} on the same court (they must start at least {} apart)",
                        other_game.number,
                        game.number,
                        time::Duration::try_from(*occupied_time).unwrap(),
                    );
                    games_overlap = true;
                }
            }
        }
    }

    if games_overlap {
        Err("Found overlapping games".into())
    } else {
        Ok(())
    }
}

fn check_same_team_in_game(schedule: &Schedule) -> Result<(), Box<dyn std::error::Error>> {
    let mut same_team_in_game = false;

    for (_, game) in &schedule.games {
        if game.light == game.dark {
            error!(
                "Game {} has the same team assigned to both sides: {}",
                game.number, game.light
            );
            same_team_in_game = true;
        }
    }

    if same_team_in_game {
        Err("Found games with the same team assigned to both sides".into())
    } else {
        Ok(())
    }
}

fn check_unique_group_names(schedule: &Schedule) -> Result<(), Box<dyn std::error::Error>> {
    let mut group_names = IndexSet::new();
    let mut duplicate_name_found = false;
    let mut group_short_names = IndexSet::new();
    let mut duplicate_short_name_found = false;

    for group in &schedule.groups {
        if !group_names.insert(group.name.clone()) {
            error!("Duplicate group name found: {}", group.name);
            duplicate_name_found = true;
        }
        if !group_short_names.insert(group.short_name.clone()) {
            error!("Duplicate group short name found: {}", group.short_name);
            duplicate_short_name_found = true;
        }
    }

    if duplicate_name_found {
        Err("Found duplicate group name".into())
    } else if duplicate_short_name_found {
        Err("Found duplicate group short name".into())
    } else {
        Ok(())
    }
}

/// Checks that the given vec of teams is not empty and contains no duplicates. Returns true if the check fails.
fn check_list_for_empty_or_duplicates(
    teams: &[ScheduledTeam],
    group_name: &str,
    list_name: &str,
) -> bool {
    if teams.is_empty() {
        error!("The group {group_name} is missing its {list_name}");
        return true;
    }
    if teams.len() != teams.iter().collect::<IndexSet<_>>().len() {
        error!("The group {group_name} has duplicate teams in its {list_name}");
        return true;
    }
    false
}

fn check_group_standings(schedule: &Schedule) -> Result<(), Box<dyn std::error::Error>> {
    let mut check_failed = false;
    for group in &schedule.groups {
        match &group.standings_calculation {
            None | Some(StandingsCalculation::Standard) => continue,
            Some(calculation) => {
                let mut game_teams: IndexMap<_, u32> = IndexMap::new();
                for game_number in group.game_numbers.iter() {
                    let game = &schedule.games[game_number];
                    *game_teams.entry(game.light.clone()).or_insert(0) += 1;
                    *game_teams.entry(game.dark.clone()).or_insert(0) += 1;
                }

                for (team, count) in game_teams.iter() {
                    if *count > 1 {
                        error!(
                            "Team {team} appears {count} times in the games of group {}",
                            group.name
                        );
                    }
                }

                let game_teams: IndexSet<_> = game_teams.into_keys().collect();

                match calculation {
                    StandingsCalculation::SwapIfUpset { starting_ranks } => {
                        if check_list_for_empty_or_duplicates(
                            starting_ranks,
                            &group.name,
                            "starting standings",
                        ) {
                            check_failed = true;
                            continue;
                        }
                        let starting_teams: IndexSet<_> = starting_ranks.iter().cloned().collect();
                        if game_teams != starting_teams {
                            error!(
                                concat!(
                                    "The group {} has a SwapIfUpset calculation, but the starting standings do not match the teams in the group\n",
                                    "    Teams in Starting Standings: {:?}\n",
                                    "    Teams in the group's games : {:?}"
                                ),
                                group.name, starting_teams, game_teams
                            );
                            check_failed = true;
                        }
                    }
                    StandingsCalculation::SlideIfUpset { starting_ranks, .. } => {
                        if check_list_for_empty_or_duplicates(
                            starting_ranks,
                            &group.name,
                            "starting standings",
                        ) {
                            check_failed = true;
                            continue;
                        }
                        let starting_teams: IndexSet<_> = starting_ranks.iter().cloned().collect();
                        if !game_teams.is_subset(&starting_teams) {
                            error!(
                                concat!(
                                    "The group {} has a SlideIfUpset calculation, but the starting standings do not contain the teams in the group's games\n",
                                    "    Teams in Starting Standings: {:?}\n",
                                    "    Teams in the group's games : {:?}"
                                ),
                                group.name, starting_teams, game_teams
                            );
                            check_failed = true;
                        }
                        if !game_teams.contains(starting_ranks.first().unwrap()) {
                            error!(
                                "The starting standings of group {} do not contain the top-ranked team: {:?}",
                                group.name,
                                starting_ranks.first().unwrap()
                            );
                            check_failed = true;
                        }
                        if !game_teams.contains(starting_ranks.last().unwrap()) {
                            error!(
                                "The starting standings of group {} do not contain the bottom-ranked team: {:?}",
                                group.name,
                                starting_ranks.last().unwrap()
                            );
                            check_failed = true;
                        }
                    }
                    StandingsCalculation::Exclusion {
                        starting_ranks,
                        excluded_teams,
                    } => {
                        if check_list_for_empty_or_duplicates(
                            starting_ranks,
                            &group.name,
                            "starting standings",
                        ) {
                            check_failed = true;
                            continue;
                        }
                        if check_list_for_empty_or_duplicates(
                            starting_ranks,
                            &group.name,
                            "excluded teams",
                        ) {
                            check_failed = true;
                            continue;
                        }
                        if excluded_teams.len() >= starting_ranks.len() {
                            error!(
                                "The group {} has an Exclusion calculation, but the number of excluded teams ({}) is greater than or equal to the number of starting teams ({})",
                                group.name,
                                excluded_teams.len(),
                                starting_ranks.len()
                            );
                            check_failed = true;
                            continue;
                        }
                    }
                    StandingsCalculation::Standard => unreachable!(),
                };
            }
        }
    }

    if check_failed {
        Err("Found invalid standings in a group".into())
    } else {
        Ok(())
    }
}

fn check_final_results(schedule: &Schedule) -> Result<(), Box<dyn std::error::Error>> {
    let mut check_failed = false;

    for group in &schedule.groups {
        if let Some(final_results) = &group.final_results {
            match final_results {
                FinalResults::Standings => {
                    if group.standings_calculation.is_none() {
                        error!(
                            "Group {} has Standings final results but no standings calculation",
                            group.name
                        );
                        check_failed = true;
                    }
                }
                FinalResults::ListOfGames { list_of_games } => {
                    for game_result in list_of_games.iter() {
                        if !group.game_numbers.contains(game_result.game_number()) {
                            error!(
                                "Game {} is referenced in the final results of group {} but is not part of the group",
                                game_result.game_number(),
                                group.name
                            );
                            check_failed = true;
                        }
                    }
                }
                FinalResults::ListOfPlacements { .. } => {
                    // Placements reference seeds and game results from other groups;
                    // cross-group validation is not performed here
                }
            }
        }
    }

    if check_failed {
        Err("Found invalid final results in a group".into())
    } else {
        Ok(())
    }
}

fn check_game_references(schedule: &Schedule) -> Result<(), Box<dyn std::error::Error>> {
    let occupied_time_map = calculate_occupied_times(schedule);
    let mut game_end_times: IndexMap<GameNumber, OffsetDateTime> = IndexMap::new();
    let mut games: IndexMap<GameNumber, &Game> = IndexMap::new();

    let mut check_failed = false;

    for (_, game) in &schedule.games {
        games.insert(game.number.clone(), game);
        game_end_times.insert(
            game.number.clone(),
            game.start_time + occupied_time_map[&game.timing_rule],
        );
    }

    let mut group_teams_count: IndexMap<String, (usize, OffsetDateTime)> = IndexMap::new();

    // First populate all the groups that directly reference games (i.e. not Exclusion groups)
    for group in &schedule.groups {
        if !matches!(
            group.standings_calculation,
            Some(StandingsCalculation::Exclusion { .. })
        ) {
            let mut teams = IndexSet::new();
            let mut last_game_end_time = OffsetDateTime::UNIX_EPOCH;
            for game_num in &group.game_numbers {
                if let Some(game) = games.get(game_num) {
                    teams.insert(game.light.clone());
                    teams.insert(game.dark.clone());
                    if game_end_times[&game.number] > last_game_end_time {
                        last_game_end_time = game_end_times[&game.number];
                    }
                } else {
                    error!(
                        "Game number {} in group {} does not exist",
                        game_num, group.name
                    );
                    check_failed = true;
                }
            }

            let team_count = match &group.standings_calculation {
                None => 0,
                Some(StandingsCalculation::Standard) => teams.len(),
                Some(StandingsCalculation::SwapIfUpset { starting_ranks }) => starting_ranks.len(),
                Some(StandingsCalculation::SlideIfUpset { starting_ranks, .. }) => {
                    starting_ranks.len()
                }
                Some(StandingsCalculation::Exclusion { .. }) => unreachable!(),
            };

            group_teams_count.insert(group.name.clone(), (team_count, last_game_end_time));
        }
    }

    // Now we can populate the Exculsion groups that reference other groups via their starting ranks,
    // but once the starting ranks are populated, those groups can be referenced by others
    for exclusion_group in &schedule.groups {
        if let Some(StandingsCalculation::Exclusion {
            starting_ranks,
            excluded_teams,
        }) = &exclusion_group.standings_calculation
        {
            let mut last_game_end_time = OffsetDateTime::UNIX_EPOCH;

            for team in starting_ranks {
                if let Some(SeededBy {
                    number,
                    group: Some(group),
                }) = team.seeded_by()
                {
                    if let Some((num_teams, end_time)) = group_teams_count.get(group) {
                        if *number > *num_teams as u32 {
                            error!(
                                "A starting seed in group {} references a non-existent seed number {} in group {}",
                                exclusion_group.name, number, group
                            );
                            check_failed = true;
                        } else if end_time > &last_game_end_time {
                            last_game_end_time = *end_time;
                        }
                    } else {
                        error!(
                            "A starting seed in group {} references a non-existent group: {}",
                            exclusion_group.name, group
                        );
                        check_failed = true;
                    }
                } else if let Some(result) = team.result_of() {
                    let n = result.game_number();
                    if let Some(end_time) = game_end_times.get(n) {
                        if end_time > &last_game_end_time {
                            last_game_end_time = *end_time;
                        }
                    } else {
                        error!(
                            "A starting seed in group {} references a non-existent game: {}",
                            exclusion_group.name, n
                        );
                        check_failed = true;
                    }
                } else {
                    warn!(
                        "A starting seed in group {} does not reference a game or a seeded team",
                        exclusion_group.name
                    );
                }
            }

            let team_count = starting_ranks.len() - excluded_teams.len();

            group_teams_count.insert(
                exclusion_group.name.clone(),
                (team_count, last_game_end_time),
            );
        }
    }

    if check_failed {
        return Err("Found invalid game references".into());
    }

    for (_, game) in &schedule.games {
        for team in [&game.light, &game.dark] {
            if let Some(result) = team.result_of() {
                let n = result.game_number();
                if let Some(&end_time) = game_end_times.get(n) {
                    if game.start_time < end_time {
                        error!(
                            "Game {} references a game that has not ended yet: {n}",
                            game.number
                        );
                        check_failed = true;
                    }
                } else {
                    error!("Game {} references a non-existent game: {n}", game.number);
                    check_failed = true;
                }
            } else if let Some(SeededBy {
                number,
                group: Some(group),
            }) = team.seeded_by()
            {
                if let Some((num_teams, last_end)) = group_teams_count.get(group) {
                    if u32::try_from(*num_teams).unwrap() < *number {
                        error!(
                            "Game {} expects {number} teams in group {group}, but only {num_teams} teams are scheduled to play in that group",
                            game.number
                        );
                        check_failed = true;
                    } else if game.start_time < *last_end {
                        error!(
                            "Game {} references a group that has not ended yet: {group}",
                            game.number
                        );
                        check_failed = true;
                    }
                } else {
                    error!(
                        "Game {} references a non-existent group: {group}",
                        game.number
                    );
                    check_failed = true;
                }
            }
        }
    }

    if check_failed {
        Err("Found invalid game references".into())
    } else {
        Ok(())
    }
}

/// A duration a timing rule leaves at zero even though the rule will use it.
// `Debug` only - nothing clones, copies or compares one.
#[derive(Debug)]
struct ZeroDuration {
    /// The Portal's own field name, so the organiser can find the box to fill in.
    field: &'static str,
    /// Why this rule needs the duration, and what to do instead of zeroing it.
    /// A full clause, because the remedy is not the same shape for every row:
    /// most switches are turned OFF to drop the duration, but half-time is
    /// dropped by turning single-period ON. A single shared closing sentence
    /// gets that one backwards.
    /// `None` where every rule needs the duration whatever its switches say.
    reason: Option<&'static str>,
}

/// Every duration this rule will actually use but has left at zero.
///
/// A duration is only checked when its own switch says the rule uses it: a rule
/// with overtime turned off may carry zeros in all three overtime fields, and
/// that is correct, not a fault. This mirrors the Portal's `AddDurationRules`,
/// which gates each field with `.When(...)` and refuses only what the rule uses.
///
/// The half-time gate is `single_period` being FALSE. A single-period game has no
/// half-time, so a zero there is legal; a two-period game must have a real one.
/// Writing this gate the other way round checks half-time only for single-period
/// games, which is exactly backwards.
///
/// The Portal's own predicates read `!= false` / `!= true` because its flags are
/// nullable, so an absent flag makes the gate fire. Our flags are plain `bool`
/// and `single_period` carries `#[serde(default)]`, so an absent `singlePeriod`
/// arrives as `false` and the gate fires just the same. Do not introduce an
/// `Option` here to "match" them - the behaviours already agree.
fn flag_gated_zero_durations(rule: &TimingRule) -> Vec<ZeroDuration> {
    // One row per duration: the value, whether this rule uses it, and the switch
    // that decides. A new duration is one row here, not a new branch elsewhere.
    let checks = [
        (rule.half_play_duration, true, "halfPlayDuration", None),
        (rule.minimum_break, true, "minimumBreak", None),
        (
            rule.half_time_duration,
            !rule.single_period,
            "halfTimeDuration",
            Some(
                "this rule is not marked as a single-period game. Give it a real \
                 length, or mark the rule as single-period.",
            ),
        ),
        (
            rule.team_timeout_duration,
            rule.team_timeout_count != 0,
            "teamTimeoutDuration",
            Some(
                "it allows team timeouts. Give it a real length, or set the team \
                 timeout count to zero.",
            ),
        ),
        (
            rule.ot_half_play_duration,
            rule.overtime_allowed,
            "overtimeHalfPlayDuration",
            Some("it allows overtime. Give it a real length, or turn overtime off."),
        ),
        (
            rule.ot_half_time_duration,
            rule.overtime_allowed,
            "overtimeHalfTimeDuration",
            Some("it allows overtime. Give it a real length, or turn overtime off."),
        ),
        (
            rule.pre_overtime_break,
            rule.overtime_allowed,
            "preOvertimeBreak",
            Some("it allows overtime. Give it a real length, or turn overtime off."),
        ),
        (
            rule.pre_sudden_death_duration,
            rule.sudden_death_allowed,
            "preSuddenDeathDuration",
            Some(
                "it allows sudden death. Give it a real length, or turn sudden \
                 death off.",
            ),
        ),
        // Ours only - the Portal has no Game Block check. Absent means "derive
        // it", which is legal, so absence is expressed as "not used" like every
        // other row rather than as a branch after the fold.
        (
            rule.game_block.unwrap_or(Duration::ZERO),
            rule.game_block.is_some(),
            "gameBlock",
            // The one OPTIONAL field, so its remedy is the opposite of the
            // always-required ones: leaving it out is legal and a zero is not.
            // Saying "every timing rule needs a real one" here would push an
            // organiser into inventing a slot length.
            Some(
                "it is the slot length for the whole game. Give it the real \
                 length, or leave the field out entirely and refbox works one \
                 out from the other durations.",
            ),
        ),
    ];

    checks
        .into_iter()
        .filter(|(value, is_used, _, _)| *is_used && value.is_zero())
        .map(|(_, _, field, reason)| ZeroDuration { field, reason })
        .collect()
}

/// Read by a tournament organiser who has to go and fix it, so it names the rule,
/// the field as the Portal labels it, and the switch that makes it required.
fn zero_duration_message(rule: &TimingRule, zero: &ZeroDuration) -> String {
    match zero.reason {
        Some(reason) => format!(
            "Timing rule '{}' sets {} to zero, but {}",
            rule.name, zero.field, reason
        ),
        None => format!(
            "Timing rule '{}' sets {} to zero. Every timing rule needs a real one.",
            rule.name, zero.field
        ),
    }
}

/// Refuse a timing rule that leaves a duration at zero while the switch that uses
/// it is on.
///
/// Mirrors the validator on the Portal's `fix/reject-zero-playing-durations`,
/// which is not on their main branch yet. Until it lands this is the only gate,
/// and it covers every rule a PERSON can author: no Portal client has a
/// timing-rule editor, so an authored rule reaches an event through this tool.
///
/// It is NOT the only way in, and a green run here is not proof that no game can
/// be played on a rule this would have refused:
///
/// - The Portal exposes `PUT /api/events/{slug}/schedule/timing-rules/{id}`
///   (`api/Controllers/EventScheduleController.cs:890`). No client calls it, but
///   anyone with edit rights on the event can call it directly, and it validates
///   nothing until their branch lands. THEIR change closes that, not this one.
/// - A refbox pointed at a custom or third-party site takes that site's timing
///   rules straight from it (`docs/third-party-integration.md`). Those never come
///   near this tool, and nothing on the refbox side re-checks them - the
///   normaliser that used to absorb a degenerate overtime was removed on the base
///   branch.
///
/// Should a Portal timing-rule editor ever be built, it needs this same guard.
///
/// Every offending rule is reported, not just the first: an organiser fixing a
/// schedule wants the whole list in one pass.
fn check_flag_gated_durations(schedule: &Schedule) -> Result<(), Box<dyn std::error::Error>> {
    let mut invalid_rules = Vec::new();

    for rule in &schedule.timing_rules {
        let zeros = flag_gated_zero_durations(rule);
        if zeros.is_empty() {
            continue;
        }
        for zero in &zeros {
            error!("{}", zero_duration_message(rule, zero));
        }
        invalid_rules.push(format!(
            "{} ({})",
            rule.name,
            zeros.iter().map(|z| z.field).collect::<Vec<_>>().join(", ")
        ));
    }

    if invalid_rules.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Found {} with a zero duration the rule still uses: {}",
            if invalid_rules.len() == 1 {
                "a timing rule"
            } else {
                "timing rules"
            },
            invalid_rules.join("; ")
        )
        .into())
    }
}

fn calculate_occupied_times(schedule: &Schedule) -> IndexMap<String, Duration> {
    let mut occupied_time_map: IndexMap<String, Duration> = IndexMap::new();

    for rule in &schedule.timing_rules {
        // Mirrors the regulation-time derivation in uwh-common's TimingRule ->
        // GameConfig conversion. A single-period game has one period and no
        // half-time break, so counting two halves plus a break it never takes
        // holds the court far longer than the game does and reports overlaps
        // between games that do not overlap.
        let regulation = if rule.single_period {
            rule.half_play_duration
        } else {
            2 * rule.half_play_duration + rule.half_time_duration
        };
        occupied_time_map.insert(rule.name.clone(), regulation + rule.minimum_break);
    }
    occupied_time_map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schedule_with_rules(timing_rules: Vec<TimingRule>) -> Schedule {
        Schedule {
            event_id: EventId::from_partial("test-event"),
            games: Default::default(),
            non_game_entries: vec![],
            groups: vec![],
            timing_rules,
            standings_order: None,
            final_results_order: None,
            referees_by_game_number: None,
        }
    }

    fn a_valid_rule() -> TimingRule {
        TimingRule {
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
            game_block: Some(Duration::from_secs(1920)),
        }
    }

    #[test]
    fn a_rule_with_no_zero_durations_passes() {
        let schedule = schedule_with_rules(vec![a_valid_rule()]);
        assert!(check_flag_gated_durations(&schedule).is_ok());
    }

    #[test]
    fn an_absent_game_block_is_not_a_zero() {
        // `gameBlock` is optional. Absent means "derive it", which is legal;
        // only a present zero is an error.
        let mut rule = a_valid_rule();
        rule.game_block = None;
        let schedule = schedule_with_rules(vec![rule]);
        assert!(check_flag_gated_durations(&schedule).is_ok());
    }

    #[test]
    fn a_single_period_game_occupies_one_half_not_two() {
        // A court is held for the play plus the gap after it. One period of 12
        // minutes plus a 4-minute break is 16 minutes -- NOT 12 + 12 + 3 + 4.
        // Over-counting here reports overlaps between games that do not overlap.
        let mut rule = a_valid_rule();
        rule.single_period = true;
        let occupied = calculate_occupied_times(&schedule_with_rules(vec![rule]));
        assert_eq!(occupied["RR"], Duration::from_secs(720 + 240));
    }

    #[test]
    fn a_two_half_game_still_occupies_both_halves_and_the_break() {
        let occupied = calculate_occupied_times(&schedule_with_rules(vec![a_valid_rule()]));
        assert_eq!(occupied["RR"], Duration::from_secs(720 * 2 + 180 + 240));
    }

    /// The table is only worth anything if each row fires for its own field and
    /// stays quiet for every other, so each pair is asserted in both directions.
    fn fields_flagged(rule: &TimingRule) -> Vec<&'static str> {
        flag_gated_zero_durations(rule)
            .into_iter()
            .map(|z| z.field)
            .collect()
    }

    #[test]
    fn a_sound_rule_flags_nothing() {
        assert_eq!(fields_flagged(&a_valid_rule()), Vec::<&str>::new());
    }

    /// Both switch combinations leave every OTHER duration legal, so whichever
    /// field the caller zeroes is the only one that may be reported.
    const BOTH_SWITCH_SETTINGS: [(bool, bool, bool, u16); 2] =
        [(false, false, false, 1), (true, true, true, 0)];

    fn with_switches(settings: (bool, bool, bool, u16)) -> TimingRule {
        let (single_period, overtime, sudden_death, timeouts) = settings;
        let mut rule = a_valid_rule();
        rule.single_period = single_period;
        rule.overtime_allowed = overtime;
        rule.sudden_death_allowed = sudden_death;
        rule.team_timeout_count = timeouts;
        rule
    }

    #[test]
    fn a_zero_half_play_duration_is_refused_whatever_the_switches() {
        // Driven across both switch settings on purpose. Against a_valid_rule()'s
        // defaults alone this test passes even if the row's gate is changed from
        // `true` to `!single_period` - so the one thing its name claims would go
        // unguarded.
        for settings in BOTH_SWITCH_SETTINGS {
            let mut rule = with_switches(settings);
            rule.half_play_duration = Duration::ZERO;
            assert_eq!(
                fields_flagged(&rule),
                vec!["halfPlayDuration"],
                "switches {settings:?}"
            );
        }
    }

    #[test]
    fn a_zero_minimum_break_is_refused_whatever_the_switches() {
        for settings in BOTH_SWITCH_SETTINGS {
            let mut rule = with_switches(settings);
            rule.minimum_break = Duration::ZERO;
            assert_eq!(
                fields_flagged(&rule),
                vec!["minimumBreak"],
                "switches {settings:?}"
            );
        }
    }

    #[test]
    fn every_duration_on_the_type_has_a_row_in_the_table() {
        // Not circular: this asks `TimingRule` itself what it serialises, so a
        // duration added there fails here rather than shipping unguarded. Mirrors
        // `timing_rule_field_names_match_the_type` in csv_parser.rs.
        //
        // This carries the exhaustiveness rationale of the deleted
        // `every_duration_field_is_rejected_at_zero`. The per-pair tests each
        // cover one row; none of them notices a row that was never written.
        const NOT_DURATIONS: [&str; 7] = [
            "name",
            "teamTimeoutCount",
            "teamTimeoutsCountedPerHalf",
            "overtimeAllowed",
            "suddenDeathAllowed",
            "singlePeriod",
            "last2minStopTime",
        ];

        let mut rule = with_switches((false, true, true, 1));
        rule.half_play_duration = Duration::ZERO;
        rule.half_time_duration = Duration::ZERO;
        rule.team_timeout_duration = Duration::ZERO;
        rule.ot_half_play_duration = Duration::ZERO;
        rule.ot_half_time_duration = Duration::ZERO;
        rule.pre_overtime_break = Duration::ZERO;
        rule.pre_sudden_death_duration = Duration::ZERO;
        rule.minimum_break = Duration::ZERO;
        rule.game_block = Some(Duration::ZERO);

        let serde_json::Value::Object(map) = serde_json::to_value(&rule).unwrap() else {
            panic!("a TimingRule should serialise to a JSON object");
        };
        let mut on_the_type: Vec<&str> = map
            .keys()
            .map(String::as_str)
            .filter(|key| !NOT_DURATIONS.contains(key))
            .collect();
        on_the_type.sort_unstable();

        let mut covered = fields_flagged(&rule);
        covered.sort_unstable();

        assert_eq!(
            covered, on_the_type,
            "every duration on TimingRule needs a row in the `checks` table"
        );

        // Compile-time half of the same guard. The check above reads SERIALISED
        // keys, which cannot see an `Option` field that is `None` under
        // `skip_serializing_if` - `gameBlock`'s exact shape - so a future
        // duration of that shape would be invisible to it. This destructure has
        // no `..`, so adding any field to `TimingRule` stops this file compiling
        // until somebody decides whether it needs a row.
        let TimingRule {
            name: _,
            team_timeout_count: _,
            team_timeouts_counted_per_half: _,
            overtime_allowed: _,
            sudden_death_allowed: _,
            single_period: _,
            last_2_min_stop_time: _,
            half_play_duration: _,
            half_time_duration: _,
            team_timeout_duration: _,
            ot_half_play_duration: _,
            ot_half_time_duration: _,
            pre_overtime_break: _,
            pre_sudden_death_duration: _,
            minimum_break: _,
            game_block: _,
        } = a_valid_rule();
    }

    #[test]
    fn a_two_period_rule_needs_a_real_half_time() {
        let mut rule = a_valid_rule();
        rule.single_period = false;
        rule.half_time_duration = Duration::ZERO;
        assert_eq!(fields_flagged(&rule), vec!["halfTimeDuration"]);
    }

    #[test]
    fn a_single_period_rule_may_leave_half_time_at_zero() {
        // The gate is `singlePeriod != true`: a single-period game has no
        // half-time, so a zero is correct rather than a fault. The Portal
        // accepts this shape, and refusing it here would block uploads they
        // allow. Replaces `a_single_period_rule_still_needs_a_real_half_time`,
        // which encoded the blanket rule the PO overruled on 2026-09-12.
        let mut rule = a_valid_rule();
        rule.single_period = true;
        rule.half_time_duration = Duration::ZERO;
        assert_eq!(fields_flagged(&rule), Vec::<&str>::new());
    }

    #[test]
    fn a_rule_allowing_timeouts_needs_a_real_timeout_length() {
        let mut rule = a_valid_rule();
        rule.team_timeout_count = 1;
        rule.team_timeout_duration = Duration::ZERO;
        assert_eq!(fields_flagged(&rule), vec!["teamTimeoutDuration"]);
    }

    #[test]
    fn a_rule_allowing_no_timeouts_may_leave_the_timeout_length_at_zero() {
        let mut rule = a_valid_rule();
        rule.team_timeout_count = 0;
        rule.team_timeout_duration = Duration::ZERO;
        assert_eq!(fields_flagged(&rule), Vec::<&str>::new());
    }

    #[test]
    fn overtime_on_needs_all_three_overtime_durations() {
        // All three are reported at once, not just the first: an organiser
        // fixing the rule wants the whole list in one pass.
        let mut rule = a_valid_rule();
        rule.overtime_allowed = true;
        rule.ot_half_play_duration = Duration::ZERO;
        rule.ot_half_time_duration = Duration::ZERO;
        rule.pre_overtime_break = Duration::ZERO;
        assert_eq!(
            fields_flagged(&rule),
            vec![
                "overtimeHalfPlayDuration",
                "overtimeHalfTimeDuration",
                "preOvertimeBreak"
            ]
        );
    }

    #[test]
    fn overtime_off_may_leave_all_three_overtime_durations_at_zero() {
        // The commonest real shape by far: the round-robin rule in every one of
        // our exports carries zeros here with overtime switched off.
        let mut rule = a_valid_rule();
        rule.overtime_allowed = false;
        rule.ot_half_play_duration = Duration::ZERO;
        rule.ot_half_time_duration = Duration::ZERO;
        rule.pre_overtime_break = Duration::ZERO;
        assert_eq!(fields_flagged(&rule), Vec::<&str>::new());
    }

    #[test]
    fn sudden_death_on_needs_a_real_pre_sudden_death_break() {
        let mut rule = a_valid_rule();
        rule.sudden_death_allowed = true;
        rule.pre_sudden_death_duration = Duration::ZERO;
        assert_eq!(fields_flagged(&rule), vec!["preSuddenDeathDuration"]);
    }

    #[test]
    fn sudden_death_off_may_leave_the_pre_sudden_death_break_at_zero() {
        let mut rule = a_valid_rule();
        rule.sudden_death_allowed = false;
        rule.pre_sudden_death_duration = Duration::ZERO;
        assert_eq!(fields_flagged(&rule), Vec::<&str>::new());
    }

    #[test]
    fn a_present_zero_game_block_is_still_refused() {
        // Ours only - the Portal has no Game Block check - so nothing upstream
        // would catch this if the row were dropped.
        let mut rule = a_valid_rule();
        rule.game_block = Some(Duration::ZERO);
        assert_eq!(fields_flagged(&rule), vec!["gameBlock"]);
    }

    #[test]
    fn the_message_names_the_rule_the_field_and_the_switch() {
        let mut rule = a_valid_rule();
        rule.overtime_allowed = true;
        rule.ot_half_play_duration = Duration::ZERO;
        let zeros = flag_gated_zero_durations(&rule);
        let msg = zero_duration_message(&rule, &zeros[0]);
        assert!(msg.contains("RR"), "must name the rule, got: {msg}");
        assert!(
            msg.contains("overtimeHalfPlayDuration"),
            "must name the field, got: {msg}"
        );
        assert!(
            msg.contains("it allows overtime"),
            "must name the switch that makes it required, got: {msg}"
        );
    }

    #[test]
    fn the_half_time_message_names_the_real_remedy() {
        // Half-time is the only gate phrased as a negative. A single shared
        // closing sentence ("or turn that setting off") inverted on it and told
        // the organiser to do the opposite of the fix, so each row now carries
        // its own remedy. Asserted as a whole sentence because that is what is
        // read; asserting the fragments separately still passes if they are
        // assembled into nonsense.
        let mut rule = a_valid_rule();
        rule.half_time_duration = Duration::ZERO;
        let zeros = flag_gated_zero_durations(&rule);
        assert_eq!(
            zero_duration_message(&rule, &zeros[0]),
            "Timing rule 'RR' sets halfTimeDuration to zero, but this rule is not marked \
             as a single-period game. Give it a real length, or mark the rule as \
             single-period."
        );
    }

    #[test]
    fn a_positive_gate_message_says_to_turn_that_feature_off() {
        let mut rule = a_valid_rule();
        rule.overtime_allowed = true;
        rule.ot_half_play_duration = Duration::ZERO;
        let zeros = flag_gated_zero_durations(&rule);
        assert_eq!(
            zero_duration_message(&rule, &zeros[0]),
            "Timing rule 'RR' sets overtimeHalfPlayDuration to zero, but it allows \
             overtime. Give it a real length, or turn overtime off."
        );
    }

    #[test]
    fn an_always_required_duration_does_not_invent_a_switch() {
        let mut rule = a_valid_rule();
        rule.half_play_duration = Duration::ZERO;
        let zeros = flag_gated_zero_durations(&rule);
        let msg = zero_duration_message(&rule, &zeros[0]);
        assert!(
            !msg.contains("but"),
            "there is no switch to name for an always-required duration, got: {msg}"
        );
        assert!(msg.contains("Every timing rule needs"), "got: {msg}");
    }

    #[test]
    fn check_flag_gated_durations_names_every_offending_rule_not_just_the_first() {
        // The blanket check this replaced returned on the first bad rule, so an
        // organiser fixed one, re-ran, and found another.
        let mut first = a_valid_rule();
        first.half_play_duration = Duration::ZERO;
        let mut second = a_valid_rule();
        second.name = "FINALS".to_string();
        second.minimum_break = Duration::ZERO;

        let err = check_flag_gated_durations(&schedule_with_rules(vec![first, second]))
            .expect_err("both rules are invalid");
        let msg = err.to_string();
        assert!(msg.contains("RR"), "should name the first rule, got: {msg}");
        assert!(
            msg.contains("FINALS"),
            "should name the second rule too, got: {msg}"
        );
    }

    #[test]
    fn the_production_finals_shape_is_refused() {
        // The exact shape every one of our JSON exports carried before this
        // branch: overtime allowed, all three overtime durations zero, only the
        // pre-sudden-death break filled in. The Portal refuses it (verified live
        // against their API), and `normalize_degenerate_overtime` used to be the
        // thing that stopped it crashing refbox at the end of such a game.
        let mut rule = a_valid_rule();
        rule.name = "FINALS".to_string();
        rule.overtime_allowed = true;
        rule.sudden_death_allowed = true;
        rule.ot_half_play_duration = Duration::ZERO;
        rule.ot_half_time_duration = Duration::ZERO;
        rule.pre_overtime_break = Duration::ZERO;
        rule.pre_sudden_death_duration = Duration::from_secs(60);

        assert!(
            check_flag_gated_durations(&schedule_with_rules(vec![rule])).is_err(),
            "the shape that crashed the app must not reach an upload"
        );
    }

    #[test]
    fn run_schedule_checks_refuses_a_rule_whose_switched_on_duration_is_zero() {
        // Guards the WIRING, not the check. Every other test here calls
        // `check_flag_gated_durations` directly, so deleting its line in
        // `run_schedule_checks` would leave the whole suite green and the gate
        // silently gone. That exact gap got past the Game Block review.
        let mut rule = a_valid_rule();
        rule.overtime_allowed = true;
        rule.ot_half_play_duration = Duration::ZERO;

        let refused = run_schedule_checks(&schedule_with_rules(vec![rule]))
            .expect_err("a zero duration the rule uses must stop the schedule loading");
        assert!(
            refused.to_string().contains("zero duration"),
            "must be refused for the zero duration, not something else: {refused}"
        );

        run_schedule_checks(&schedule_with_rules(vec![a_valid_rule()]))
            .expect("the same schedule with sound durations passes");
    }
}
