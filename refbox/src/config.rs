use super::fl;
use crate::app::languages::Language;
use crate::sound_controller::SoundSettings;
use derivative::Derivative;
use enum_derive_2018::EnumFromStr;
use macro_attr_2018::macro_attr;
use matrix_drawing::transmitted_data::Brightness;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use toml::Table;
pub use uwh_common::config::Game;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hardware {
    pub screen_x: i32,
    pub screen_y: i32,
    pub white_on_right: bool,
    pub brightness: Brightness,
}

impl Default for Hardware {
    fn default() -> Self {
        Self {
            screen_x: 945,
            screen_y: 691,
            white_on_right: false,
            brightness: Brightness::Low,
        }
    }
}

impl Hardware {
    pub fn migrate(old: &Table) -> Self {
        let Self {
            mut screen_x,
            mut screen_y,
            mut white_on_right,
            mut brightness,
        } = Default::default();

        get_integer_value(old, "screen_x", &mut screen_x);
        get_integer_value(old, "screen_y", &mut screen_y);
        get_boolean_value(old, "white_on_right", &mut white_on_right);
        if let Some(old_brightness) = old.get("brightness") {
            if let Some(old_brightness) = old_brightness.as_str() {
                if let Ok(old_brightness) = old_brightness.parse() {
                    brightness = old_brightness;
                }
            }
        }

        Self {
            screen_x,
            screen_y,
            white_on_right,
            brightness,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UwhPortal {
    pub token: String,
}

impl UwhPortal {
    pub fn migrate(old: &Table) -> Self {
        let Self { mut token } = Default::default();
        get_string_value(old, "token", &mut token);
        Self { token }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Level {
    pub count: u8,
    #[serde(with = "secs_only_duration")]
    pub duration: std::time::Duration,
}

impl Level {
    pub fn migrate(old: &Table) -> Self {
        let Self {
            mut count,
            mut duration,
        } = Default::default();

        if let Some(value) = old.get("count") {
            if let Some(value) = value.as_integer().and_then(|i| i.try_into().ok()) {
                count = value;
            }
        }

        if let Some(value) = old.get("duration") {
            if let Some(value) = value.as_integer().and_then(|i| i.try_into().ok()) {
                duration = std::time::Duration::from_secs(value);
            }
        }

        Self { count, duration }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeepTest {
    /// Duration of the warm-up period shown on the display as "Level 0".
    /// After the warm-up, the schedule proceeds through `levels`
    /// (Level 1, Level 2, ...).
    #[serde(with = "secs_only_duration")]
    pub pre: std::time::Duration,
    pub levels: Vec<Level>,
}

/// Court-length presets for the beep test.
///
/// Each preset defines a whole schedule: the warm-up and seven levels
/// totalling 26 laps. 26 is the passing lap count, and the lap counts are
/// arranged (3, 3, 4, 4, 4, 4, 4) so that lap 26 is the final lap of Level 7
/// — the test ends exactly on the pass mark.
///
/// The level times scale with court length: the 21m and 23m columns are the
/// 25m times multiplied by 21/25 and 23/25 and rounded up, matching the
/// adjusted tables the tournament uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeepTestPreset {
    M25,
    M23,
    M21,
}

impl BeepTestPreset {
    pub const ALL: [Self; 3] = [Self::M25, Self::M23, Self::M21];

    /// Laps per level. Identical for every court length — only the times
    /// change. Sums to 26.
    const LAP_COUNTS: [u8; 7] = [3, 3, 4, 4, 4, 4, 4];

    /// Court length in metres, used for the DISTANCE tile's label.
    pub fn metres(self) -> u8 {
        match self {
            Self::M25 => 25,
            Self::M23 => 23,
            Self::M21 => 21,
        }
    }

    /// The seven level durations in seconds, Level 1 first.
    fn level_secs(self) -> [u64; 7] {
        match self {
            Self::M25 => [36, 34, 32, 30, 28, 26, 24],
            Self::M23 => [34, 32, 30, 28, 26, 24, 23],
            Self::M21 => [31, 29, 27, 26, 24, 22, 21],
        }
    }

    /// The complete beep-test configuration for this court length.
    pub fn config(self) -> BeepTest {
        BeepTest {
            pre: std::time::Duration::from_secs(10),
            levels: Self::LAP_COUNTS
                .iter()
                .zip(self.level_secs())
                .map(|(&count, secs)| Level {
                    count,
                    duration: std::time::Duration::from_secs(secs),
                })
                .collect(),
        }
    }

    /// Which preset the given staged level list matches, if any.
    ///
    /// The EDIT LEVELS screen stages only the levels — not the whole config —
    /// so detection compares level lists rather than configs. `None` means the
    /// operator has hand-edited the schedule, and the screen highlights no
    /// preset.
    pub fn detect_levels(levels: &[Level]) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|p| p.config().levels.as_slice() == levels)
    }
}

impl Default for BeepTest {
    fn default() -> Self {
        BeepTestPreset::M25.config()
    }
}

impl BeepTest {
    /// The incorrect 10-level table that shipped as the default before the
    /// 26-lap correction: 5 laps at Level 9 and a 38-lap run. A config file
    /// holding exactly this table is holding the old default rather than a
    /// deliberate operator choice, so `migrate` replaces it.
    fn legacy_default_levels() -> Vec<Level> {
        [
            (3u8, 36u64),
            (3, 34),
            (3, 32),
            (4, 30),
            (4, 28),
            (4, 26),
            (4, 24),
            (4, 22),
            (5, 20),
            (4, 18),
        ]
        .iter()
        .map(|&(count, secs)| Level {
            count,
            duration: std::time::Duration::from_secs(secs),
        })
        .collect()
    }

    pub fn migrate(old: &Table) -> Self {
        let Self {
            mut pre,
            mut levels,
        } = Default::default();

        if let Some(value) = old.get("pre") {
            if let Some(value) = value.as_integer().and_then(|i| i.try_into().ok()) {
                pre = std::time::Duration::from_secs(value);
            }
        }

        if let Some(values) = old.get("levels") {
            if let Some(values) = values.as_array() {
                // An override in the config file replaces the default levels entirely.
                levels.clear();
                for value in values {
                    if let Some(table) = value.as_table() {
                        levels.push(Level::migrate(table))
                    }
                }
                // ...unless it is the old, incorrect shipped default, in which
                // case carry the operator forward onto the corrected table.
                if levels == Self::legacy_default_levels() {
                    levels = BeepTestPreset::M25.config().levels;
                }
            }
        }

        Self { pre, levels }
    }
}

mod secs_only_duration {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(dur: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(dur.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Duration::from_secs(u64::deserialize(deserializer)?))
    }
}

#[derive(Derivative, Serialize, Deserialize)]
#[derivative(Clone, Debug, Default, PartialEq, Eq)]
pub struct Config {
    pub mode: Mode,
    pub hide_time: bool,
    #[derivative(Default(value = "true"))]
    pub collect_scorer_cap_num: bool,
    pub track_fouls_and_warnings: bool,
    #[derivative(Default(value = "true"))]
    pub show_behind_schedule_time: bool,
    #[derivative(Default(value = "true"))]
    pub confirm_score: bool,
    #[serde(default)]
    pub audible_countdown: bool,
    pub game: Game,
    pub beep_test: BeepTest,
    pub hardware: Hardware,
    pub uwhportal: UwhPortal,
    pub sound: SoundSettings,
    pub language: Option<Language>,
    #[serde(default)]
    pub display_mode: crate::app::theme::DisplayMode,
    #[serde(default)]
    pub front_display_layout: crate::sim_frame::FrontDisplayLayout,
}

impl Config {
    pub fn migrate(old: &Table) -> Self {
        let Self {
            mut mode,
            mut hide_time,
            mut collect_scorer_cap_num,
            mut track_fouls_and_warnings,
            mut show_behind_schedule_time,
            confirm_score,
            mut audible_countdown,
            mut game,
            mut beep_test,
            mut hardware,
            mut uwhportal,
            mut sound,
            language,
            display_mode,
            front_display_layout,
        } = Default::default();

        if let Some(old_mode) = old.get("mode") {
            if let Some(old_mode) = old_mode.as_str() {
                if let Ok(old_mode) = old_mode.parse() {
                    mode = old_mode;
                }
            }
        }
        get_boolean_value(old, "hide_time", &mut hide_time);
        get_boolean_value(old, "collect_scorer_cap_num", &mut collect_scorer_cap_num);
        get_boolean_value(
            old,
            "track_fouls_and_warnings",
            &mut track_fouls_and_warnings,
        );
        get_boolean_value(
            old,
            "show_behind_schedule_time",
            &mut show_behind_schedule_time,
        );
        get_boolean_value(old, "audible_countdown", &mut audible_countdown);
        if let Some(old_game) = old.get("game") {
            if let Some(old_game) = old_game.as_table() {
                game = Game::migrate(old_game);
            }
        }
        if let Some(old_beep_test) = old.get("beep_test") {
            if let Some(old_beep_test) = old_beep_test.as_table() {
                beep_test = BeepTest::migrate(old_beep_test);
            }
        }
        if let Some(old_hardware) = old.get("hardware") {
            if let Some(old_hardware) = old_hardware.as_table() {
                hardware = Hardware::migrate(old_hardware);
            }
        }
        if let Some(old_uwhportal) = old.get("uwhportal") {
            if let Some(old_uwhportal) = old_uwhportal.as_table() {
                uwhportal = UwhPortal::migrate(old_uwhportal);
            }
        }
        if let Some(old_sound) = old.get("sound") {
            if let Some(old_sound) = old_sound.as_table() {
                sound = SoundSettings::migrate(old_sound);
            }
        }

        Self {
            mode,
            hide_time,
            collect_scorer_cap_num,
            track_fouls_and_warnings,
            show_behind_schedule_time,
            confirm_score,
            audible_countdown,
            game,
            beep_test,
            hardware,
            uwhportal,
            sound,
            language,
            display_mode,
            front_display_layout,
        }
    }
}

macro_attr! {
    #[derive(Debug, Clone, Copy, Derivative, PartialEq, Eq, Serialize, Deserialize, EnumFromStr!)]
    #[derivative(Default)]
    pub enum Mode {
        #[derivative(Default)]
        Hockey6V6,
        Hockey3V3,
        Rugby,
        BeepTest,
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hockey6V6 => f.write_str(&fl!("hockey6v6")),
            Self::Hockey3V3 => f.write_str(&fl!("hockey3v3")),
            Self::Rugby => f.write_str(&fl!("rugby")),
            Self::BeepTest => f.write_str(&fl!("beep-test")),
        }
    }
}

fn get_integer_value<T: DeserializeOwned + TryFrom<i64>>(table: &Table, key: &str, save: &mut T) {
    if let Some(value) = table.get(key) {
        if let Some(value) = value.as_integer() {
            if let Ok(value) = value.try_into() {
                *save = value;
            }
        }
    }
}

fn get_boolean_value(table: &Table, key: &str, save: &mut bool) {
    if let Some(value) = table.get(key) {
        if let Some(value) = value.as_bool() {
            *save = value;
        }
    }
}

fn get_string_value(table: &Table, key: &str, save: &mut String) {
    if let Some(value) = table.get(key) {
        if let Some(value) = value.as_str() {
            *save = value.to_string();
        }
    }
}

#[cfg(test)]
mod test {
    use crate::sound_controller::Volume;

    use super::*;
    use std::time::Duration;

    #[test]
    fn test_ser_hardware() {
        let hw: Hardware = Default::default();
        let serialized = toml::to_string(&hw).unwrap();
        let deser = toml::from_str(&serialized);
        assert_eq!(deser, Ok(hw));
    }

    #[test]
    fn test_ser_uwhportal() {
        let u: UwhPortal = Default::default();
        let serialized = toml::to_string(&u).unwrap();
        let deser = toml::from_str(&serialized);
        assert_eq!(deser, Ok(u));
    }

    #[test]
    fn test_ser_config() {
        let config: Config = Default::default();
        let serialized = toml::to_string(&config).unwrap();
        let deser = toml::from_str(&serialized);
        assert_eq!(deser, Ok(config));
    }

    #[test]
    fn test_ser_beep_test() {
        let bt: BeepTest = Default::default();
        let serialized = toml::to_string(&bt).unwrap();
        let deser = toml::from_str(&serialized);
        assert_eq!(deser, Ok(bt));
    }

    #[test]
    fn test_migrate_beep_test_absent() {
        let old: Table = Default::default();
        let config = Config::migrate(&old);
        assert_eq!(config.beep_test, BeepTest::default());
    }

    #[test]
    fn test_migrate_beep_test_present() {
        let mut old: Table = Default::default();
        let mut bt: Table = Default::default();
        bt.insert("pre".to_string(), toml::Value::Integer(20));
        let mut levels: Vec<toml::Value> = Vec::new();
        let mut level: Table = Default::default();
        level.insert("count".to_string(), toml::Value::Integer(2));
        level.insert("duration".to_string(), toml::Value::Integer(15));
        levels.push(toml::Value::Table(level));
        bt.insert("levels".to_string(), toml::Value::Array(levels));
        old.insert("beep_test".to_string(), toml::Value::Table(bt));
        let config = Config::migrate(&old);
        assert_eq!(config.beep_test.pre, std::time::Duration::from_secs(20));
        // An override in the config file replaces the default levels entirely.
        assert_eq!(config.beep_test.levels.len(), 1);
        assert_eq!(config.beep_test.levels[0].count, 2);
        assert_eq!(
            config.beep_test.levels[0].duration,
            std::time::Duration::from_secs(15)
        );
    }

    // Every preset ends on lap 26 — the passing lap — with lap 26 the final
    // lap of Level 7. This is the invariant the whole feature rests on.
    #[test]
    fn every_preset_ends_on_lap_26() {
        for preset in BeepTestPreset::ALL {
            let config = preset.config();
            assert_eq!(config.levels.len(), 7, "{preset:?} has 7 levels");
            let laps: u32 = config.levels.iter().map(|l| u32::from(l.count)).sum();
            assert_eq!(laps, 26, "{preset:?} totals 26 laps");
        }
    }

    // Lap counts are identical across court lengths — only the times scale.
    #[test]
    fn preset_lap_counts_are_identical() {
        let expected: Vec<u8> = vec![3, 3, 4, 4, 4, 4, 4];
        for preset in BeepTestPreset::ALL {
            let counts: Vec<u8> = preset.config().levels.iter().map(|l| l.count).collect();
            assert_eq!(counts, expected, "{preset:?} lap counts");
        }
    }

    // The confirmed 25m table, including its 13:00 total run time.
    #[test]
    fn preset_25m_matches_confirmed_table() {
        let config = BeepTestPreset::M25.config();
        let secs: Vec<u64> = config.levels.iter().map(|l| l.duration.as_secs()).collect();
        assert_eq!(secs, vec![36, 34, 32, 30, 28, 26, 24]);
        assert_eq!(config.pre, std::time::Duration::from_secs(10));

        let total: u64 = config.pre.as_secs()
            + config
                .levels
                .iter()
                .map(|l| l.duration.as_secs() * u64::from(l.count))
                .sum::<u64>();
        assert_eq!(total, 780, "25m run is 13:00 including the warm-up");
    }

    // The confirmed 21m table, read off the operator's sheet.
    #[test]
    fn preset_21m_matches_confirmed_table() {
        let config = BeepTestPreset::M21.config();
        let secs: Vec<u64> = config.levels.iter().map(|l| l.duration.as_secs()).collect();
        assert_eq!(secs, vec![31, 29, 27, 26, 24, 22, 21]);
    }

    // 23m has no official table — it is DEFINED as ceil(25m x 0.92), the same
    // method the official 21m table documents for itself (ceil(25m x 0.84)).
    // This test enforces the derivation, so the numbers cannot drift away from
    // the rule that produced them. 21m is included to prove the rule holds for
    // the column we can check against a real sheet.
    #[test]
    fn shorter_court_durations_are_the_25m_table_scaled_and_rounded_up() {
        let base = BeepTestPreset::M25;
        for (preset, ratio) in [
            (BeepTestPreset::M23, 0.92_f64),
            (BeepTestPreset::M21, 0.84_f64),
        ] {
            for (i, (short, long)) in preset
                .config()
                .levels
                .iter()
                .zip(base.config().levels.iter())
                .enumerate()
            {
                let expected = (long.duration.as_secs() as f64 * ratio).ceil() as u64;
                assert_eq!(
                    short.duration.as_secs(),
                    expected,
                    "{preset:?} level {} should be ceil({}s x {ratio})",
                    i + 1,
                    long.duration.as_secs()
                );
            }
        }
    }

    // detect_levels is the inverse of config().levels: it recognises each
    // preset exactly and returns None for a hand-edited schedule. This drives
    // which preset button is highlighted on the EDIT LEVELS screen.
    #[test]
    fn detect_levels_round_trips_and_rejects_custom() {
        for preset in BeepTestPreset::ALL {
            assert_eq!(
                BeepTestPreset::detect_levels(&preset.config().levels),
                Some(preset)
            );
        }
        let mut custom = BeepTestPreset::M25.config().levels;
        custom[0].duration = std::time::Duration::from_secs(35);
        assert_eq!(BeepTestPreset::detect_levels(&custom), None);

        // A truncated list must not match — a prefix is not the schedule.
        let short = &BeepTestPreset::M25.config().levels[..3];
        assert_eq!(BeepTestPreset::detect_levels(short), None);
    }

    // The built-in default is the 25m preset.
    #[test]
    fn default_is_the_25m_preset() {
        assert_eq!(BeepTest::default(), BeepTestPreset::M25.config());
    }

    // A config file still holding the old, incorrect 10-level default table
    // (which had 5 laps at Level 9 and ran to 38 laps) is migrated to the
    // corrected 25m preset. That exact table was never a deliberate operator
    // choice — it was the shipped default — so replacing it is safe.
    #[test]
    fn migrate_replaces_legacy_default_table() {
        let mut bt = toml::value::Table::new();
        let legacy: Vec<toml::Value> = [
            (3u8, 36u64),
            (3, 34),
            (3, 32),
            (4, 30),
            (4, 28),
            (4, 26),
            (4, 24),
            (4, 22),
            (5, 20),
            (4, 18),
        ]
        .iter()
        .map(|&(count, secs)| {
            let mut t = toml::value::Table::new();
            t.insert("count".to_string(), toml::Value::Integer(count.into()));
            t.insert("duration".to_string(), toml::Value::Integer(secs as i64));
            toml::Value::Table(t)
        })
        .collect();
        bt.insert("levels".to_string(), toml::Value::Array(legacy));

        let migrated = BeepTest::migrate(&bt);
        assert_eq!(migrated.levels, BeepTestPreset::M25.config().levels);
    }

    // A genuinely hand-edited table is preserved untouched.
    #[test]
    fn migrate_preserves_custom_levels() {
        let mut bt = toml::value::Table::new();
        let mut lvl = toml::value::Table::new();
        lvl.insert("count".to_string(), toml::Value::Integer(2));
        lvl.insert("duration".to_string(), toml::Value::Integer(45));
        bt.insert(
            "levels".to_string(),
            toml::Value::Array(vec![toml::Value::Table(lvl)]),
        );

        let migrated = BeepTest::migrate(&bt);
        assert_eq!(migrated.levels.len(), 1);
        assert_eq!(migrated.levels[0].count, 2);
        assert_eq!(
            migrated.levels[0].duration,
            std::time::Duration::from_secs(45)
        );
    }

    #[test]
    fn test_migrate_hardware() {
        let mut old: Table = Default::default();
        old.insert("screen_x".to_string(), toml::Value::Integer(123));
        old.insert("screen_y".to_string(), toml::Value::Integer(456));
        old.insert("white_on_right".to_string(), toml::Value::Boolean(true));
        let hw = Hardware::migrate(&old);
        assert_eq!(hw.screen_x, 123);
        assert_eq!(hw.screen_y, 456);
        assert!(hw.white_on_right);
    }

    #[test]
    fn test_migrate_uwhportal() {
        let mut old: Table = Default::default();
        old.insert(
            "url".to_string(),
            toml::Value::String("https://localhost/api/v1/".to_string()),
        );
        old.insert(
            "token".to_string(),
            toml::Value::String("token".to_string()),
        );
        // url field is no longer persisted; migrate should silently ignore it
        let u = UwhPortal::migrate(&old);
        assert_eq!(u.token, "token");
    }

    #[test]
    fn config_missing_display_mode_defaults_to_light() {
        // A config TOML written before this field existed must still load.
        let toml_without_field = toml::to_string(&Config::default())
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with("display_mode"))
            .collect::<Vec<_>>()
            .join("\n");
        let parsed: Config = toml::from_str(&toml_without_field).unwrap();
        assert_eq!(parsed.display_mode, crate::app::theme::DisplayMode::Light);
    }

    #[test]
    fn config_display_mode_round_trips() {
        let config = Config {
            display_mode: crate::app::theme::DisplayMode::HighContrast,
            ..Default::default()
        };
        let serialized = toml::to_string(&config).unwrap();
        let deser: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(
            deser.display_mode,
            crate::app::theme::DisplayMode::HighContrast
        );
    }

    #[test]
    fn config_missing_front_display_layout_defaults_to_default() {
        // A config TOML written before this field existed must still load.
        let toml_without_field = toml::to_string(&Config::default())
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with("front_display_layout"))
            .collect::<Vec<_>>()
            .join("\n");
        let parsed: Config = toml::from_str(&toml_without_field).unwrap();
        assert_eq!(
            parsed.front_display_layout,
            crate::sim_frame::FrontDisplayLayout::Default
        );
    }

    #[test]
    fn config_front_display_layout_round_trips() {
        let config = Config {
            front_display_layout: crate::sim_frame::FrontDisplayLayout::Corners,
            ..Default::default()
        };
        let serialized = toml::to_string(&config).unwrap();
        let deser: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(
            deser.front_display_layout,
            crate::sim_frame::FrontDisplayLayout::Corners
        );
    }

    #[test]
    fn test_migrate_audible_countdown_defaults_false_when_absent() {
        let old: Table = Default::default();
        let config = Config::migrate(&old);
        assert!(!config.audible_countdown);
    }

    #[test]
    fn test_migrate_audible_countdown_respects_present_true() {
        let mut old: Table = Default::default();
        old.insert("audible_countdown".to_string(), toml::Value::Boolean(true));
        let config = Config::migrate(&old);
        assert!(config.audible_countdown);
    }

    #[test]
    fn test_migrate_show_behind_schedule_time_defaults_true_when_absent() {
        let old: Table = Default::default();
        let config = Config::migrate(&old);
        assert!(config.show_behind_schedule_time);
    }

    #[test]
    fn test_migrate_show_behind_schedule_time_respects_present_false() {
        let mut old: Table = Default::default();
        old.insert(
            "show_behind_schedule_time".to_string(),
            toml::Value::Boolean(false),
        );
        let config = Config::migrate(&old);
        assert!(!config.show_behind_schedule_time);
    }

    #[test]
    fn test_migrate_config() {
        let mut old: Table = Default::default();
        old.insert("mode".to_string(), toml::Value::String("Rugby".to_string()));
        old.insert("hide_time".to_string(), toml::Value::Boolean(true));
        old.insert(
            "collect_scorer_cap_num".to_string(),
            toml::Value::Boolean(true),
        );
        let mut game: Table = Default::default();
        game.insert("half_play_duration".to_string(), toml::Value::Integer(123));
        old.insert("game".to_string(), toml::Value::Table(game));
        let mut hardware: Table = Default::default();
        hardware.insert("screen_x".to_string(), toml::Value::Integer(123));
        hardware.insert("screen_y".to_string(), toml::Value::Integer(456));
        hardware.insert("white_on_right".to_string(), toml::Value::Boolean(true));
        old.insert("hardware".to_string(), toml::Value::Table(hardware));
        let mut uwhportal: Table = Default::default();
        uwhportal.insert(
            "url".to_string(),
            toml::Value::String("https://localhost/api/v1/".to_string()),
        );
        uwhportal.insert(
            "token".to_string(),
            toml::Value::String("token".to_string()),
        );
        old.insert("uwhportal".to_string(), toml::Value::Table(uwhportal));
        let mut sound: Table = Default::default();
        sound.insert("sound_enabled".to_string(), toml::Value::Boolean(false));
        sound.insert(
            "whistle_vol".to_string(),
            toml::Value::String("Max".to_string()),
        );
        old.insert("sound".to_string(), toml::Value::Table(sound));
        let config = Config::migrate(&old);
        assert_eq!(config.mode, Mode::Rugby);
        assert!(config.hide_time);
        assert!(config.collect_scorer_cap_num);
        assert_eq!(config.game.half_play_duration, Duration::from_secs(123));
        assert_eq!(config.hardware.screen_x, 123);
        assert_eq!(config.hardware.screen_y, 456);
        assert!(config.hardware.white_on_right);
        assert_eq!(config.uwhportal.token, "token");
        assert!(!config.sound.sound_enabled);
        assert_eq!(config.sound.whistle_vol, Volume::Max);
    }
}
