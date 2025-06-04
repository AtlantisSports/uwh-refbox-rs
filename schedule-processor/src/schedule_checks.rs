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
        games.sort_by(|a, b| a.0.start_time.cmp(&b.0.start_time));
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
                if let Some(SeededBy { number, group }) = team.seeded_by() {
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
            } else if let Some(SeededBy { number, group }) = team.seeded_by() {
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

fn calculate_occupied_times(schedule: &Schedule) -> IndexMap<String, Duration> {
    let mut occupied_time_map: IndexMap<String, Duration> = IndexMap::new();

    for rule in &schedule.timing_rules {
        let occupied_time =
            2 * rule.half_play_duration + rule.half_time_duration + rule.minimum_break;
        occupied_time_map.insert(rule.name.clone(), occupied_time);
    }
    occupied_time_map
}
