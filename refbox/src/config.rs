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

/// A third-party site standing in for the UWH Portal. Keeps its own
/// credential: the Portal's token stays in `UwhPortal`, so switching
/// sources never overwrites the other one's login.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomSite {
    pub url: String,
    pub token: String,
}

impl CustomSite {
    pub fn migrate(old: &Table) -> Self {
        let Self { mut url, mut token } = Default::default();
        get_string_value(old, "url", &mut url);
        get_string_value(old, "token", &mut token);
        Self { url, token }
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

/// Truncates a full level schedule to `target` total laps.
///
/// Whole levels are kept unchanged while the running lap total is still
/// short of `target`. The level whose laps would push the total past
/// `target` is kept but cut down to exactly the laps still needed to reach
/// it; every level after that one is dropped. General on purpose — it does
/// not know or care which level index or lap count that turns out to be,
/// so it works unchanged for any full schedule and any target.
///
/// Callers must pass a non-zero `target`: `target = 0` returns an empty
/// `Vec<Level>` (nothing to keep), and downstream code does not accept an
/// empty schedule — `TournamentManager::start_beep_test_now` in
/// `beep_test/cadence.rs` expects `Level(0)` to have a duration precisely
/// because it assumes `config.levels` is non-empty.
fn truncate_at_laps(levels: &[Level], target: u8) -> Vec<Level> {
    let mut out = Vec::new();
    let mut total: u8 = 0;
    for level in levels {
        if total >= target {
            break;
        }
        let count = level.count.min(target - total);
        out.push(Level {
            count,
            duration: level.duration,
        });
        total += count;
    }
    out
}

/// Court-length and test-length presets for the beep test.
///
/// Each court length (25m / 23m / 25yd / 22m / 21m) has two variants:
/// - **Full** — the tournament's official table end to end: 10 levels, 37
///   laps. Players run this.
/// - **Ref** — the same table truncated to the passing lap count (26).
///   Referees run this. The Ref levels are never typed out separately;
///   `config` derives them from the Full table via `truncate_at_laps`, so
///   they cannot drift from it.
///
/// The level times scale with court length: every shorter column is the 25m
/// times multiplied by that column's ratio and rounded up, matching the
/// adjusted tables the tournament uses. The ratios are 0.92 (23m), 0.9144
/// (25yd, which is 22.86m), 0.88 (22m) and 0.84 (21m) — see
/// `shorter_court_durations_are_the_25m_table_scaled_and_rounded_up`.
///
/// Ordered longest pool first. 25yd sits between 23m and 22m because 25
/// yards is 22.86m, shorter than 23m.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BeepTestPreset {
    Ref25,
    Ref23,
    Ref25Yd,
    Ref22,
    Ref21,
    Full25,
    Full23,
    Full25Yd,
    Full22,
    Full21,
}

impl BeepTestPreset {
    pub const ALL: [Self; 10] = [
        Self::Ref25,
        Self::Ref23,
        Self::Ref25Yd,
        Self::Ref22,
        Self::Ref21,
        Self::Full25,
        Self::Full23,
        Self::Full25Yd,
        Self::Full22,
        Self::Full21,
    ];

    /// Laps per level in the full schedule. Identical for every court
    /// length — only the times change. Sums to 37.
    const FULL_LAP_COUNTS: [u8; 10] = [3, 3, 3, 4, 4, 4, 4, 4, 4, 4];

    /// The number of laps that passes the test. The Ref schedules are the
    /// Full schedule truncated to exactly this many laps.
    const PASSING_LAPS: u8 = 26;

    /// The court length as it appears on the preset button, unit included.
    /// Four of the five pools are metric; the fifth is a 25-yard pool, and
    /// labelling it "25M" would name a different pool. The unit letter is not
    /// translated, matching how it has always been written into these labels.
    pub fn distance_label(self) -> &'static str {
        match self {
            Self::Ref25 | Self::Full25 => "25M",
            Self::Ref23 | Self::Full23 => "23M",
            Self::Ref25Yd | Self::Full25Yd => "25YD",
            Self::Ref22 | Self::Full22 => "22M",
            Self::Ref21 | Self::Full21 => "21M",
        }
    }

    /// The court length in millimetres — a pool's identity, independent of how
    /// a button spells it. 25 yards is 22860mm, which is why it sorts between
    /// 23m and 22m.
    ///
    /// Deliberately separate from `distance_label`: that is display text and may
    /// be reworded or translated, while this is what pairs each referee schedule
    /// with the full schedule for the same pool. Test-only, because pairing them
    /// is something only the tests need to do — the screen names both halves of
    /// a row explicitly.
    #[cfg(test)]
    pub fn court_millimetres(self) -> u32 {
        match self {
            Self::Ref25 | Self::Full25 => 25_000,
            Self::Ref23 | Self::Full23 => 23_000,
            Self::Ref25Yd | Self::Full25Yd => 22_860,
            Self::Ref22 | Self::Full22 => 22_000,
            Self::Ref21 | Self::Full21 => 21_000,
        }
    }

    /// Whether this is the shorter, 26-lap referee schedule, as opposed to
    /// the full, 37-lap player schedule.
    pub fn is_ref(self) -> bool {
        matches!(
            self,
            Self::Ref25 | Self::Ref23 | Self::Ref25Yd | Self::Ref22 | Self::Ref21
        )
    }

    /// The ten full-schedule level durations in seconds, Level 1 first. Ref
    /// and Full share the same table for a given court length — Ref is a
    /// truncation of Full, not a separately-tracked table.
    fn full_level_secs(self) -> [u64; 10] {
        match self {
            Self::Ref25 | Self::Full25 => [36, 34, 32, 30, 28, 26, 24, 22, 20, 18],
            Self::Ref23 | Self::Full23 => [34, 32, 30, 28, 26, 24, 23, 21, 19, 17],
            Self::Ref25Yd | Self::Full25Yd => [33, 32, 30, 28, 26, 24, 22, 21, 19, 17],
            Self::Ref22 | Self::Full22 => [32, 30, 29, 27, 25, 23, 22, 20, 18, 16],
            Self::Ref21 | Self::Full21 => [31, 29, 27, 26, 24, 22, 21, 19, 17, 16],
        }
    }

    /// The full (37-lap) schedule's ten levels for this preset's court
    /// length.
    fn full_levels(self) -> Vec<Level> {
        Self::FULL_LAP_COUNTS
            .iter()
            .zip(self.full_level_secs())
            .map(|(&count, secs)| Level {
                count,
                duration: std::time::Duration::from_secs(secs),
            })
            .collect()
    }

    /// The complete beep-test configuration for this preset.
    pub fn config(self) -> BeepTest {
        let full = self.full_levels();
        let levels = if self.is_ref() {
            truncate_at_laps(&full, Self::PASSING_LAPS)
        } else {
            full
        };
        BeepTest {
            pre: std::time::Duration::from_secs(10),
            levels,
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
        BeepTestPreset::Ref25.config()
    }
}

impl BeepTest {
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
    #[serde(default)]
    pub source: GameSource,
    #[serde(default)]
    pub remembered_remote: RemoteSource,
    pub game: Game,
    pub beep_test: BeepTest,
    pub hardware: Hardware,
    pub uwhportal: UwhPortal,
    #[serde(default)]
    pub custom_site: CustomSite,
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
            mut confirm_score,
            mut audible_countdown,
            mut source,
            mut remembered_remote,
            mut game,
            mut beep_test,
            mut hardware,
            mut uwhportal,
            mut custom_site,
            mut sound,
            mut language,
            mut display_mode,
            mut front_display_layout,
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
        get_boolean_value(old, "confirm_score", &mut confirm_score);
        get_boolean_value(old, "audible_countdown", &mut audible_countdown);
        if let Some(old_source) = old.get("source") {
            if let Some(old_source) = old_source.as_str() {
                if let Ok(old_source) = old_source.parse() {
                    source = old_source;
                }
            }
        }
        if let Some(old_remembered_remote) = old.get("remembered_remote") {
            if let Some(old_remembered_remote) = old_remembered_remote.as_str() {
                if let Ok(old_remembered_remote) = old_remembered_remote.parse() {
                    remembered_remote = old_remembered_remote;
                }
            }
        }
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
        if let Some(old_custom_site) = old.get("custom_site") {
            if let Some(old_custom_site) = old_custom_site.as_table() {
                custom_site = CustomSite::migrate(old_custom_site);
            }
        }
        if let Some(old_sound) = old.get("sound") {
            if let Some(old_sound) = old_sound.as_table() {
                sound = SoundSettings::migrate(old_sound);
            }
        }
        get_serde_value(old, "language", &mut language);
        get_serde_value(old, "display_mode", &mut display_mode);
        get_serde_value(old, "front_display_layout", &mut front_display_layout);

        Self {
            mode,
            hide_time,
            collect_scorer_cap_num,
            track_fouls_and_warnings,
            show_behind_schedule_time,
            confirm_score,
            audible_countdown,
            source,
            remembered_remote,
            game,
            beep_test,
            hardware,
            uwhportal,
            custom_site,
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

macro_attr! {
    /// Where this refbox gets its games. `Manual` means the operator enters
    /// everything by hand; the other two are remote sources.
    #[derive(Debug, Clone, Copy, Derivative, PartialEq, Eq, Serialize, Deserialize, EnumFromStr!)]
    #[derivative(Default)]
    pub enum GameSource {
        #[derivative(Default)]
        Manual,
        Portal,
        Custom,
    }
}

macro_attr! {
    /// Which remote source to return to when leaving `Manual`. Kept separately
    /// from `GameSource` so that switching to manual and back does not lose the
    /// operator's choice of remote.
    #[derive(Debug, Clone, Copy, Derivative, PartialEq, Eq, Serialize, Deserialize, EnumFromStr!)]
    #[derivative(Default)]
    pub enum RemoteSource {
        #[derivative(Default)]
        Portal,
        Custom,
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

/// Reads a value that `Config` stores through serde rather than as a bare TOML scalar —
/// the unit-variant enums, and `Option`s of them. `save` is left at its default if the
/// key is absent or holds something that will not deserialize.
fn get_serde_value<T: DeserializeOwned>(table: &Table, key: &str, save: &mut T) {
    if let Some(value) = table.get(key) {
        if let Ok(value) = value.clone().try_into() {
            *save = value;
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

    // Every full preset covers the whole official table: 10 levels, 37
    // laps, in the same lap-count shape regardless of court length.
    #[test]
    fn every_full_preset_has_37_laps_over_ten_levels() {
        let expected_counts: Vec<u8> = vec![3, 3, 3, 4, 4, 4, 4, 4, 4, 4];
        for preset in BeepTestPreset::ALL.into_iter().filter(|p| !p.is_ref()) {
            let config = preset.config();
            assert_eq!(config.levels.len(), 10, "{preset:?} has 10 levels");
            let counts: Vec<u8> = config.levels.iter().map(|l| l.count).collect();
            assert_eq!(counts, expected_counts, "{preset:?} lap counts");
            let laps: u32 = config.levels.iter().map(|l| u32::from(l.count)).sum();
            assert_eq!(laps, 37, "{preset:?} totals 37 laps");
        }
    }

    // Every ref preset is the first 26 laps of its full counterpart: the
    // same durations level for level, with only the final level's count
    // reduced to land exactly on the pass mark. Comparing against the full
    // preset programmatically (rather than a typed-out expected list) is
    // what proves the truncation, not a second copy of the data.
    #[test]
    fn every_ref_preset_is_a_26_lap_prefix_of_its_full_counterpart() {
        for ref_preset in BeepTestPreset::ALL.into_iter().filter(|p| p.is_ref()) {
            let full_preset = BeepTestPreset::ALL
                .into_iter()
                .find(|p| !p.is_ref() && p.court_millimetres() == ref_preset.court_millimetres())
                .expect("every ref preset has a full counterpart at its court length");
            let ref_levels = ref_preset.config().levels;
            let full_levels = full_preset.config().levels;

            let laps: u32 = ref_levels.iter().map(|l| u32::from(l.count)).sum();
            assert_eq!(laps, 26, "{ref_preset:?} totals 26 laps");
            assert!(
                ref_levels.len() <= full_levels.len(),
                "{ref_preset:?} has more levels than {full_preset:?}"
            );

            for (i, (r, f)) in ref_levels.iter().zip(full_levels.iter()).enumerate() {
                assert_eq!(
                    r.duration,
                    f.duration,
                    "{ref_preset:?} level {} duration should match {full_preset:?}",
                    i + 1
                );
                if i + 1 == ref_levels.len() {
                    assert!(
                        r.count <= f.count,
                        "{ref_preset:?} level {} count should not exceed {full_preset:?}'s",
                        i + 1
                    );
                } else {
                    assert_eq!(
                        r.count,
                        f.count,
                        "{ref_preset:?} level {} count should match {full_preset:?} \
                         until the last level",
                        i + 1
                    );
                }
            }
        }
    }

    // truncate_at_laps is only ever called with PASSING_LAPS in production,
    // so its general behaviour (documented above the function) is exercised
    // directly here rather than only indirectly through the preset tests.
    #[test]
    fn truncate_at_laps_stops_exactly_on_a_level_boundary() {
        let levels = vec![
            Level {
                count: 3,
                duration: Duration::from_secs(10),
            },
            Level {
                count: 4,
                duration: Duration::from_secs(20),
            },
            Level {
                count: 5,
                duration: Duration::from_secs(30),
            },
        ];
        // 3 + 4 = 7 lands exactly on the boundary between level 2 and level 3.
        let truncated = truncate_at_laps(&levels, 7);
        assert_eq!(truncated.len(), 2);
        assert_eq!(truncated[0].count, 3);
        assert_eq!(truncated[1].count, 4);
    }

    #[test]
    fn truncate_at_laps_with_target_past_the_total_returns_everything() {
        let levels = vec![
            Level {
                count: 3,
                duration: Duration::from_secs(10),
            },
            Level {
                count: 4,
                duration: Duration::from_secs(20),
            },
        ];
        // Total laps available is 7; ask for far more than that.
        let truncated = truncate_at_laps(&levels, 100);
        assert_eq!(truncated, levels);
    }

    #[test]
    fn truncate_at_laps_with_zero_target_returns_an_empty_schedule() {
        let levels = vec![Level {
            count: 3,
            duration: Duration::from_secs(10),
        }];
        let truncated = truncate_at_laps(&levels, 0);
        assert_eq!(truncated, vec![]);
    }

    // Every column shorter than 25m is DEFINED as ceil(25m x ratio), the
    // method the tournament's adjusted tables print at the head of each
    // column. This test enforces that derivation across all ten levels of all
    // four shorter columns, so the numbers cannot drift away from the rule
    // that produced them.
    //
    // The ratios are external data and cannot be derived from the presets, so
    // they are listed here. The final assertion pins the list against the
    // number of full presets, so a new court length cannot be added without a
    // ratio to check it against.
    #[test]
    fn shorter_court_durations_are_the_25m_table_scaled_and_rounded_up() {
        let base = BeepTestPreset::Full25.config().levels;
        let ratios = [
            (BeepTestPreset::Full23, 0.92_f64),
            (BeepTestPreset::Full25Yd, 0.9144_f64),
            (BeepTestPreset::Full22, 0.88_f64),
            (BeepTestPreset::Full21, 0.84_f64),
        ];
        for (preset, ratio) in ratios {
            for (i, (short, long)) in preset.config().levels.iter().zip(base.iter()).enumerate() {
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

        // Coverage, not a row count: counting rows passes if one preset is
        // listed twice and another omitted, leaving a court length unchecked.
        for preset in BeepTestPreset::ALL {
            if preset.is_ref() || preset == BeepTestPreset::Full25 {
                continue;
            }
            assert_eq!(
                ratios.iter().filter(|(p, _)| *p == preset).count(),
                1,
                "{preset:?} needs exactly one ratio listed here"
            );
        }
    }

    // detect_levels is the inverse of config().levels: it recognises each of
    // the ten presets exactly and returns None for a hand-edited schedule.
    // This drives which preset button is highlighted on the EDIT LEVELS
    // screen.
    //
    // It also proves the ten schedules are pairwise distinct: detect_levels
    // returns the FIRST preset that matches, so if two court lengths ever
    // produced the same table this round trip would return the wrong one and
    // fail. That matters here because 25yd (x0.9144) and 23m (x0.92) differ by
    // less than one percent and agree on eight of their ten levels.
    #[test]
    fn detect_levels_round_trips_and_rejects_custom() {
        for preset in BeepTestPreset::ALL {
            assert_eq!(
                BeepTestPreset::detect_levels(&preset.config().levels),
                Some(preset)
            );
        }
        let mut custom = BeepTestPreset::Full25.config().levels;
        custom[0].duration = std::time::Duration::from_secs(35);
        assert_eq!(BeepTestPreset::detect_levels(&custom), None);

        // A truncated list must not match — a prefix is not the schedule.
        let short = &BeepTestPreset::Full25.config().levels[..3];
        assert_eq!(BeepTestPreset::detect_levels(short), None);
    }

    // Totals including the 10s warm-up, every one read off the tournament's own
    // adjusted tables rather than recomputed here: each Full total is that
    // column's printed "End" time, and each Ref total is that column's printed
    // start time for lap 27 — the exact instant lap 26, the pass mark,
    // finishes.
    //
    // These are checks against a real external document, not incidental
    // restatements of the lap-count/derivation tests above — do not "simplify"
    // them away as redundant. What makes each one independent: the derivation
    // test compares a table against the RATIO written in this file, so a wrong
    // ratio with a table built to match it passes there. These totals compare
    // against the SHEET, so that pair fails here. Both halves are needed.
    //
    // The coverage assertion at the end means a new court length cannot be added
    // without its two times from the sheet.
    #[test]
    fn preset_totals_match_the_official_sheets() {
        fn total_secs(config: &BeepTest) -> u64 {
            config.pre.as_secs()
                + config
                    .levels
                    .iter()
                    .map(|l| l.duration.as_secs() * u64::from(l.count))
                    .sum::<u64>()
        }

        // (preset, seconds, the time printed on that column of the sheet)
        let sheet = [
            (BeepTestPreset::Full25, 988_u64, "16:28"),
            (BeepTestPreset::Full23, 930, "15:30"),
            (BeepTestPreset::Full25Yd, 923, "15:23"),
            (BeepTestPreset::Full22, 887, "14:47"),
            (BeepTestPreset::Full21, 851, "14:11"),
            (BeepTestPreset::Ref25, 770, "12:50"),
            (BeepTestPreset::Ref23, 723, "12:03"),
            (BeepTestPreset::Ref25Yd, 716, "11:56"),
            (BeepTestPreset::Ref22, 691, "11:31"),
            (BeepTestPreset::Ref21, 662, "11:02"),
        ];

        for (preset, expected, printed) in sheet {
            assert_eq!(
                total_secs(&preset.config()),
                expected,
                "{preset:?} should run {printed} on the sheet, warm-up included"
            );
        }

        // Coverage rather than a row count, for the same reason as the ratio
        // list above.
        for preset in BeepTestPreset::ALL {
            assert_eq!(
                sheet.iter().filter(|(p, _, _)| *p == preset).count(),
                1,
                "{preset:?} needs exactly one time from the sheet checked here"
            );
        }
    }

    // ALL is a hand-written list and `is_ref` a hand-written pattern; unlike
    // `distance_label` and `full_level_secs`, neither is checked by the compiler
    // when a court length is added. Leaving a new preset out of ALL would hide
    // it from the preset-highlight detection with every other test still green,
    // and leaving it out of `is_ref` would silently make a referee schedule run
    // the full 37 laps.
    //
    // So pin the shape rather than the members: no entry twice, an even split
    // between referee and full, and every court length appearing exactly once
    // in each half.
    #[test]
    fn all_pairs_every_court_length_with_a_ref_and_a_full_schedule() {
        let all = BeepTestPreset::ALL;

        for (i, a) in all.iter().enumerate() {
            for b in all.iter().skip(i + 1) {
                assert_ne!(a, b, "ALL lists {a:?} twice");
            }
        }

        let refs = all.iter().filter(|p| p.is_ref()).count();
        assert_eq!(
            refs * 2,
            all.len(),
            "ALL should be half referee schedules, half full ones"
        );

        for preset in all {
            let pool = preset.court_millimetres();
            let label = preset.distance_label();
            assert_eq!(
                all.iter().filter(|p| p.court_millimetres() == pool).count(),
                2,
                "court length {label} should appear exactly twice in ALL"
            );
            assert_eq!(
                all.iter()
                    .filter(|p| p.court_millimetres() == pool && p.is_ref())
                    .count(),
                1,
                "court length {label} needs exactly one referee schedule"
            );
        }
    }

    // The built-in default is the referee 25m preset.
    #[test]
    fn default_is_the_ref_25m_preset() {
        assert_eq!(BeepTest::default(), BeepTestPreset::Ref25.config());
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
    fn test_ser_custom_site() {
        let c: CustomSite = Default::default();
        let serialized = toml::to_string(&c).unwrap();
        let deser = toml::from_str(&serialized);
        assert_eq!(deser, Ok(c));
    }

    #[test]
    fn test_migrate_custom_site() {
        let mut old: Table = Default::default();
        old.insert(
            "url".to_string(),
            toml::Value::String("http://scoreboard.local:8099/api/events/1234-A".to_string()),
        );
        old.insert(
            "token".to_string(),
            toml::Value::String("custom-token".to_string()),
        );
        let c = CustomSite::migrate(&old);
        assert_eq!(c.url, "http://scoreboard.local:8099/api/events/1234-A");
        assert_eq!(c.token, "custom-token");
    }

    /// A config file written before a key existed: serialise today's default,
    /// then drop the key, so parsing has to fall back to the field's default.
    /// Every installation in the field is missing all three of the new keys.
    fn config_toml_without(key: &str) -> String {
        let serialized = toml::to_string(&Config::default()).unwrap();
        let mut table: Table = toml::from_str(&serialized).unwrap();
        assert!(
            table.remove(key).is_some(),
            "key {key:?} was not present in a serialised default Config, so removing it proves nothing"
        );
        toml::to_string(&table).unwrap()
    }

    #[test]
    fn config_source_round_trips() {
        for source in [GameSource::Manual, GameSource::Portal, GameSource::Custom] {
            let config = Config {
                source,
                ..Default::default()
            };
            let serialized = toml::to_string(&config).unwrap();
            let parsed: Config = toml::from_str(&serialized).unwrap();
            assert_eq!(parsed.source, source);
        }
    }

    #[test]
    fn config_remembered_remote_round_trips() {
        for remote in [RemoteSource::Portal, RemoteSource::Custom] {
            let config = Config {
                remembered_remote: remote,
                ..Default::default()
            };
            let serialized = toml::to_string(&config).unwrap();
            let parsed: Config = toml::from_str(&serialized).unwrap();
            assert_eq!(parsed.remembered_remote, remote);
        }
    }

    #[test]
    fn config_custom_site_round_trips() {
        let config = Config {
            custom_site: CustomSite {
                url: "http://scoreboard.local:8099/api/events/1234-A".to_string(),
                token: "custom-token".to_string(),
            },
            ..Default::default()
        };
        let serialized = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(parsed.custom_site, config.custom_site);
    }

    #[test]
    fn config_missing_source_defaults_to_manual() {
        let parsed: Config = toml::from_str(&config_toml_without("source")).unwrap();
        assert_eq!(parsed.source, GameSource::Manual);
    }

    #[test]
    fn config_missing_remembered_remote_defaults_to_portal() {
        let parsed: Config = toml::from_str(&config_toml_without("remembered_remote")).unwrap();
        assert_eq!(parsed.remembered_remote, RemoteSource::Portal);
    }

    #[test]
    fn config_missing_custom_site_defaults_to_empty() {
        let parsed: Config = toml::from_str(&config_toml_without("custom_site")).unwrap();
        assert_eq!(parsed.custom_site, CustomSite::default());
        assert!(parsed.custom_site.url.is_empty());
        assert!(parsed.custom_site.token.is_empty());
    }

    #[test]
    fn migrate_without_new_keys_uses_defaults() {
        // The migrate path, as distinct from serde defaults: an old table that
        // predates all three keys must still produce a usable config.
        let old: Table = Default::default();
        let config = Config::migrate(&old);
        assert_eq!(config.source, GameSource::Manual);
        assert_eq!(config.remembered_remote, RemoteSource::Portal);
        assert_eq!(config.custom_site, CustomSite::default());
    }

    #[test]
    fn migrate_reads_the_new_keys_when_present() {
        let mut old: Table = Default::default();
        old.insert(
            "source".to_string(),
            toml::Value::String("Custom".to_string()),
        );
        old.insert(
            "remembered_remote".to_string(),
            toml::Value::String("Custom".to_string()),
        );
        let mut site: Table = Default::default();
        site.insert(
            "url".to_string(),
            toml::Value::String("http://scoreboard.local:8099/api/events/1234-A".to_string()),
        );
        site.insert(
            "token".to_string(),
            toml::Value::String("custom-token".to_string()),
        );
        old.insert("custom_site".to_string(), toml::Value::Table(site));

        let config = Config::migrate(&old);
        assert_eq!(config.source, GameSource::Custom);
        assert_eq!(config.remembered_remote, RemoteSource::Custom);
        assert_eq!(
            config.custom_site.url,
            "http://scoreboard.local:8099/api/events/1234-A"
        );
        assert_eq!(config.custom_site.token, "custom-token");
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

    // The four settings below were dropped by `migrate` until 2026-08-11: they were
    // destructured out of the defaults without `mut` and never read back from the old
    // table, so any config file that needed migrating silently lost them and the reset
    // was then written back over the file. `language` is the one that bites in practice,
    // because a v0.4.0/v0.4.1 file always needs migrating (it has no
    // `show_behind_schedule_time` key).

    #[test]
    fn test_migrate_confirm_score_defaults_true_when_absent() {
        let old: Table = Default::default();
        let config = Config::migrate(&old);
        assert!(config.confirm_score);
    }

    #[test]
    fn test_migrate_confirm_score_respects_present_false() {
        let mut old: Table = Default::default();
        old.insert("confirm_score".to_string(), toml::Value::Boolean(false));
        let config = Config::migrate(&old);
        assert!(
            !config.confirm_score,
            "an operator who turned score confirmation off must not get it back on"
        );
    }

    #[test]
    fn test_migrate_language_stays_unset_when_absent() {
        let old: Table = Default::default();
        let config = Config::migrate(&old);
        assert_eq!(config.language, None);
    }

    #[test]
    fn test_migrate_language_is_preserved() {
        let mut old: Table = Default::default();
        old.insert(
            "language".to_string(),
            toml::Value::String("German".to_string()),
        );
        let config = Config::migrate(&old);
        assert_eq!(
            config.language,
            Some(Language::German),
            "a chosen interface language must survive migration"
        );
    }

    #[test]
    fn test_migrate_display_mode_is_preserved() {
        let mut old: Table = Default::default();
        old.insert(
            "display_mode".to_string(),
            toml::Value::String("HighContrast".to_string()),
        );
        let config = Config::migrate(&old);
        assert_eq!(
            config.display_mode,
            crate::app::theme::DisplayMode::HighContrast,
            "High Contrast is chosen for poolside readability and must survive migration"
        );
    }

    #[test]
    fn test_migrate_front_display_layout_is_preserved() {
        let mut old: Table = Default::default();
        old.insert(
            "front_display_layout".to_string(),
            toml::Value::String("Corners".to_string()),
        );
        let config = Config::migrate(&old);
        assert_eq!(
            config.front_display_layout,
            crate::sim_frame::FrontDisplayLayout::Corners
        );
    }

    /// The realistic path: a config file written by v0.4.0/v0.4.1 has `confirm_score`
    /// and `language` but no `show_behind_schedule_time`, so it fails to deserialize
    /// against the current `Config` and `main.rs` falls back to `migrate`. Both
    /// operator-set values must survive, and the key that did not exist yet must fall
    /// back to its default.
    #[test]
    fn test_migrate_v041_shaped_config_keeps_operator_settings() {
        let mut old: Table = Default::default();
        old.insert("mode".to_string(), toml::Value::String("Rugby".to_string()));
        old.insert("hide_time".to_string(), toml::Value::Boolean(true));
        old.insert("confirm_score".to_string(), toml::Value::Boolean(false));
        old.insert(
            "language".to_string(),
            toml::Value::String("German".to_string()),
        );
        // deliberately absent: show_behind_schedule_time, display_mode,
        // front_display_layout — none of them existed in v0.4.1

        let config = Config::migrate(&old);

        assert!(!config.confirm_score, "confirm_score must survive");
        assert_eq!(
            config.language,
            Some(Language::German),
            "language must survive"
        );
        assert_eq!(config.mode, Mode::Rugby);
        assert!(config.hide_time);
        assert!(
            config.show_behind_schedule_time,
            "a key absent from the old format falls back to its default"
        );
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
