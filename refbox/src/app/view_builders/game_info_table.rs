// Task 2 of a staged implementation: types and builder are consumed by Tasks 3–8.
// The dead_code allow is removed once the renderer + wiring tasks wire this in.
#![allow(dead_code)]
use super::*;
use uwh_common::{
    config::Game as GameConfig,
    game_snapshot::{GamePeriod, GameSnapshot},
    uwhportal::schedule::{Schedule, TeamList},
};

const TEAM_NAME_LEN_LIMIT: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum Variant {
    Full,
    Compact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum GameRole {
    Last,
    Current,
    Next,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct TeamLine {
    pub name: Option<String>,
    pub score: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) enum Row {
    GameBlock {
        role: GameRole,
        number: String,
        game_block: Option<String>,
        white: TeamLine,
        black: TeamLine,
    },
    SettingPair {
        left: (String, String),
        right: Option<(String, String)>,
    },
    Referee {
        label: String,
        name: String,
    },
}

pub(in super::super) fn game_info_rows(
    snapshot: &GameSnapshot,
    config: &GameConfig,
    using_uwhportal: bool,
    schedule: Option<&Schedule>,
    teams: Option<&TeamList>,
    last_game_scores: Option<BlackWhiteBundle<u8>>,
    variant: Variant,
) -> Vec<Row> {
    let between = snapshot.current_period == GamePeriod::BetweenGames;
    // The "current" game whose config + settings are displayed: the in-progress
    // game when playing, the upcoming game between games. Matches details_strings.
    let current_game_num: &GameNumber = if between {
        &snapshot.next_game_number
    } else {
        &snapshot.game_number
    };

    let mut rows = Vec::new();

    // --- Current game block ---
    rows.push(game_block_row(
        GameRole::Current,
        current_game_num,
        Some(time_string(config.game_block)),
        Some(snapshot.scores),
        using_uwhportal,
        schedule,
        teams,
    ));

    // --- Settings grid (belongs to the current game) ---
    let mut settings: Vec<(String, String)> = Vec::new();
    if config.single_half {
        settings.push((
            fl!("gi-game-length"),
            time_string(config.half_play_duration),
        ));
    } else {
        settings.push((
            fl!("gi-half-length"),
            time_string(config.half_play_duration),
        ));
        settings.push((
            fl!("gi-half-time-length"),
            time_string(config.half_time_duration),
        ));
    }
    settings.push((fl!("gi-timeouts"), team_timeouts_value(config)));
    if config.num_team_timeouts_allowed != 0 {
        settings.push((
            fl!("gi-timeout-duration"),
            time_string(config.team_timeout_duration),
        ));
    }
    settings.push((fl!("gi-overtime"), bool_string(config.overtime_allowed)));
    settings.push((
        fl!("gi-sudden-death"),
        bool_string(config.sudden_death_allowed),
    ));
    if config.overtime_allowed {
        settings.push((
            fl!("gi-pre-overtime-break"),
            time_string(config.pre_overtime_break),
        ));
    }
    if config.sudden_death_allowed {
        settings.push((
            fl!("gi-pre-sudden-death-break"),
            time_string(config.pre_sudden_death_duration),
        ));
    }
    if config.overtime_allowed {
        settings.push((
            fl!("gi-overtime-half-length"),
            time_string(config.ot_half_play_duration),
        ));
    }
    settings.push((
        fl!("gi-minimum-game-break"),
        time_string(config.minimum_break),
    ));
    if config.overtime_allowed {
        settings.push((
            fl!("gi-overtime-half-time-length"),
            time_string(config.ot_half_time_duration),
        ));
    }
    settings.push((
        fl!("gi-stop-clock-last-2"),
        stop_clock_value(schedule, current_game_num),
    ));

    let mut iter = settings.into_iter();
    while let Some(left) = iter.next() {
        let right = iter.next();
        rows.push(Row::SettingPair { left, right });
    }

    let _ = (variant, last_game_scores); // consumed in Tasks 3–4
    rows
}

fn team_timeouts_value(config: &GameConfig) -> String {
    if config.num_team_timeouts_allowed == 0 {
        "0".to_string()
    } else if config.timeouts_counted_per_half {
        format!("{}/{}", config.num_team_timeouts_allowed, fl!("half"))
    } else {
        format!("{}/{}", config.num_team_timeouts_allowed, fl!("game"))
    }
}

fn stop_clock_value(schedule: Option<&Schedule>, game_number: &GameNumber) -> String {
    match schedule.and_then(|s| s.get_game_timing(game_number)) {
        Some(rule) => bool_string(rule.last_2_min_stop_time),
        None => fl!("unknown"),
    }
}

// Builds a GameBlock row. `scores` populates the team lines when present; team
// names resolve from the schedule (portal only), else None.
fn game_block_row(
    role: GameRole,
    game_number: &GameNumber,
    game_block: Option<String>,
    scores: Option<BlackWhiteBundle<u8>>,
    using_uwhportal: bool,
    schedule: Option<&Schedule>,
    teams: Option<&TeamList>,
) -> Row {
    let (white_name, black_name, number) =
        resolve_game(game_number, using_uwhportal, schedule, teams);
    Row::GameBlock {
        role,
        number,
        game_block,
        white: TeamLine {
            name: white_name,
            score: scores.map(|s| s.white),
        },
        black: TeamLine {
            name: black_name,
            score: scores.map(|s| s.black),
        },
    }
}

// Returns (white_name, black_name, display_number). Names are Some only when the
// portal schedule has the game; the display number falls back to the raw number.
fn resolve_game(
    game_number: &GameNumber,
    using_uwhportal: bool,
    schedule: Option<&Schedule>,
    teams: Option<&TeamList>,
) -> (Option<String>, Option<String>, String) {
    if using_uwhportal {
        if let Some(game) = schedule.and_then(|s| s.games.get(game_number)) {
            let black = limit_team_name_len(&get_team_name(&game.dark, teams), TEAM_NAME_LEN_LIMIT);
            let white =
                limit_team_name_len(&get_team_name(&game.light, teams), TEAM_NAME_LEN_LIMIT);
            return (Some(white), Some(black), game.number.to_string());
        }
    }
    (None, None, game_number.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uwh_common::game_snapshot::GameSnapshot;

    fn cfg_all_on() -> GameConfig {
        GameConfig {
            single_half: false,
            overtime_allowed: true,
            sudden_death_allowed: true,
            num_team_timeouts_allowed: 1,
            ..Default::default()
        }
    }

    // Helper: collect the (label, value) pairs the settings grid emits, in order.
    fn setting_pairs(rows: &[Row]) -> Vec<(String, Option<String>)> {
        rows.iter()
            .filter_map(|r| match r {
                Row::SettingPair { left, right } => {
                    Some((left.0.clone(), right.as_ref().map(|p| p.0.clone())))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn current_block_always_present() {
        // GameSnapshot::default() is BetweenGames; the Current block is present in any state.
        let snapshot = GameSnapshot::default();
        let rows = game_info_rows(
            &snapshot,
            &cfg_all_on(),
            false,
            None,
            None,
            None,
            Variant::Full,
        );
        assert!(rows.iter().any(|r| matches!(
            r,
            Row::GameBlock {
                role: GameRole::Current,
                game_block: Some(_),
                ..
            }
        )));
    }

    #[test]
    fn settings_order_all_features_on() {
        let snapshot = GameSnapshot::default();
        let rows = game_info_rows(
            &snapshot,
            &cfg_all_on(),
            false,
            None,
            None,
            None,
            Variant::Full,
        );
        let pairs = setting_pairs(&rows);
        // Six rows, paired exactly as in the mockup.
        assert_eq!(
            pairs[0],
            (fl!("gi-half-length"), Some(fl!("gi-half-time-length")))
        );
        assert_eq!(
            pairs[1],
            (fl!("gi-timeouts"), Some(fl!("gi-timeout-duration")))
        );
        assert_eq!(pairs[2], (fl!("gi-overtime"), Some(fl!("gi-sudden-death"))));
        assert_eq!(
            pairs[3],
            (
                fl!("gi-pre-overtime-break"),
                Some(fl!("gi-pre-sudden-death-break"))
            )
        );
        assert_eq!(
            pairs[4],
            (
                fl!("gi-overtime-half-length"),
                Some(fl!("gi-minimum-game-break"))
            )
        );
        assert_eq!(
            pairs[5],
            (
                fl!("gi-overtime-half-time-length"),
                Some(fl!("gi-stop-clock-last-2"))
            )
        );
    }

    #[test]
    fn overtime_off_hides_overtime_rows() {
        let snapshot = GameSnapshot::default();
        let config = GameConfig {
            overtime_allowed: false,
            ..cfg_all_on()
        };
        let labels: Vec<String> = setting_pairs(&game_info_rows(
            &snapshot,
            &config,
            false,
            None,
            None,
            None,
            Variant::Full,
        ))
        .into_iter()
        .flat_map(|(l, r)| std::iter::once(l).chain(r))
        .collect();
        assert!(!labels.contains(&fl!("gi-pre-overtime-break")));
        assert!(!labels.contains(&fl!("gi-overtime-half-length")));
        assert!(!labels.contains(&fl!("gi-overtime-half-time-length")));
        assert!(labels.contains(&fl!("gi-minimum-game-break")));
        assert!(labels.contains(&fl!("gi-stop-clock-last-2")));
    }

    #[test]
    fn zero_timeouts_hides_duration() {
        let snapshot = GameSnapshot::default();
        let config = GameConfig {
            num_team_timeouts_allowed: 0,
            ..cfg_all_on()
        };
        let labels: Vec<String> = setting_pairs(&game_info_rows(
            &snapshot,
            &config,
            false,
            None,
            None,
            None,
            Variant::Full,
        ))
        .into_iter()
        .flat_map(|(l, r)| std::iter::once(l).chain(r))
        .collect();
        assert!(!labels.contains(&fl!("gi-timeout-duration")));
    }

    #[test]
    fn single_half_shows_game_length_hides_half_time() {
        let snapshot = GameSnapshot::default();
        let config = GameConfig {
            single_half: true,
            ..cfg_all_on()
        };
        let labels: Vec<String> = setting_pairs(&game_info_rows(
            &snapshot,
            &config,
            false,
            None,
            None,
            None,
            Variant::Full,
        ))
        .into_iter()
        .flat_map(|(l, r)| std::iter::once(l).chain(r))
        .collect();
        assert!(labels.contains(&fl!("gi-game-length")));
        assert!(!labels.contains(&fl!("gi-half-length")));
        assert!(!labels.contains(&fl!("gi-half-time-length")));
    }
}
