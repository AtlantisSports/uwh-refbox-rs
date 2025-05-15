use crate::config::Game as GameConfig;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt, time::Duration};
use time::{
    OffsetDateTime,
    format_description::well_known::{
        Iso8601,
        iso8601::{self, EncodedConfig, TimePrecision},
    },
};

const CONFIG: EncodedConfig = iso8601::Config::DEFAULT
    .set_year_is_six_digits(false)
    .set_time_precision(TimePrecision::Second {
        decimal_digits: None,
    })
    .encode();
pub const FORMAT: Iso8601<CONFIG> = Iso8601::<CONFIG>;
time::serde::format_description!(iso8601_4dig_year_no_subsecs, OffsetDateTime, FORMAT);

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NonGameEntry {
    #[serde(with = "iso8601_4dig_year_no_subsecs", rename = "startsOn")]
    pub start_time: OffsetDateTime,
    #[serde(with = "iso8601_4dig_year_no_subsecs::option", rename = "endsOn")]
    pub end_time: Option<OffsetDateTime>,
    pub court: Option<String>,
    pub title: String,
    pub description: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct ScheduledTeam {
    #[serde(rename = "teamId")]
    team_id: Option<TeamId>,
    #[serde(rename = "pendingAssignmentName")]
    pending_assignment_name: Option<String>,
    #[serde(rename = "resultOf")]
    result_of: Option<ResultOf>,
    #[serde(rename = "seededBy")]
    seeded_by: Option<SeededBy>,
}

impl fmt::Display for ScheduledTeam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(id) = &self.team_id {
            write!(f, "Team with ID {}", id.full())
        } else if let Some(name) = &self.pending_assignment_name {
            write!(f, "Pending Assignment {}", name)
        } else if let Some(result_of) = &self.result_of {
            match result_of {
                ResultOf::Winner { game_number } => write!(f, "Winner of game {}", game_number),
                ResultOf::Loser { game_number } => write!(f, "Loser of game {}", game_number),
            }
        } else if let Some(seeded_by) = &self.seeded_by {
            write!(f, "Group {} Seed {}", seeded_by.group, seeded_by.number)
        } else {
            write!(f, "Unknown team")
        }
    }
}

impl ScheduledTeam {
    pub fn new_team_id(id: TeamId) -> Self {
        Self {
            team_id: Some(id),
            pending_assignment_name: None,
            result_of: None,
            seeded_by: None,
        }
    }

    pub fn new_pending_assignment_name<S: ToString>(name: S) -> Self {
        Self {
            team_id: None,
            pending_assignment_name: Some(name.to_string()),
            result_of: None,
            seeded_by: None,
        }
    }

    pub fn new_winner_of<S: ToString>(game_number: S) -> Self {
        Self {
            team_id: None,
            pending_assignment_name: None,
            result_of: Some(ResultOf::Winner {
                game_number: game_number.to_string(),
            }),
            seeded_by: None,
        }
    }

    pub fn new_loser_of<S: ToString>(game_number: S) -> Self {
        Self {
            team_id: None,
            pending_assignment_name: None,
            result_of: Some(ResultOf::Loser {
                game_number: game_number.to_string(),
            }),
            seeded_by: None,
        }
    }

    pub fn new_seeded_by<S: ToString>(seed: u32, group: S) -> Self {
        Self {
            team_id: None,
            pending_assignment_name: None,
            result_of: None,
            seeded_by: Some(SeededBy {
                number: seed,
                group: group.to_string(),
            }),
        }
    }

    pub fn assigned(&self) -> Option<&TeamId> {
        self.team_id.as_ref()
    }

    pub fn pending(&self) -> Option<&str> {
        if self.team_id.is_none() {
            self.pending_assignment_name.as_deref()
        } else {
            None
        }
    }

    pub fn result_of(&self) -> Option<&ResultOf> {
        if self.team_id.is_none() {
            self.result_of.as_ref()
        } else {
            None
        }
    }

    pub fn seeded_by(&self) -> Option<&SeededBy> {
        if self.team_id.is_none() {
            self.seeded_by.as_ref()
        } else {
            None
        }
    }

    pub fn seeded_by_mut(&mut self) -> Option<&mut SeededBy> {
        if self.team_id.is_none() {
            self.seeded_by.as_mut()
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(tag = "type")]
pub enum ResultOf {
    Winner {
        #[serde(rename = "gameNumber")]
        game_number: GameNumber,
    },
    Loser {
        #[serde(rename = "gameNumber")]
        game_number: GameNumber,
    },
}

impl ResultOf {
    pub fn game_number(&self) -> &GameNumber {
        match self {
            ResultOf::Winner { game_number } => game_number,
            ResultOf::Loser { game_number } => game_number,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct SeededBy {
    pub number: u32,
    #[serde(with = "item_name")]
    pub group: String,
}

pub type GameNumber = String;

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Game {
    pub number: GameNumber,
    pub dark: ScheduledTeam,
    pub light: ScheduledTeam,
    #[serde(with = "iso8601_4dig_year_no_subsecs", rename = "startsOn")]
    pub start_time: OffsetDateTime,
    pub court: String,
    #[serde(with = "item_name", rename = "timingRule")]
    pub timing_rule: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimingRule {
    pub name: String,
    #[serde(rename = "teamTimeoutCount")]
    pub team_timeout_count: u16,
    #[serde(rename = "teamTimeoutsCountedPerHalf")]
    pub team_timeouts_counted_per_half: bool,
    #[serde(rename = "overtimeAllowed")]
    pub overtime_allowed: bool,
    #[serde(rename = "suddenDeathAllowed")]
    pub sudden_death_allowed: bool,
    #[serde(with = "secs_only_duration", rename = "halfPlayDuration")]
    pub half_play_duration: Duration,
    #[serde(with = "secs_only_duration", rename = "halfTimeDuration")]
    pub half_time_duration: Duration,
    #[serde(with = "secs_only_duration", rename = "teamTimeoutDuration")]
    pub team_timeout_duration: Duration,
    #[serde(with = "secs_only_duration", rename = "overtimeHalfPlayDuration")]
    pub ot_half_play_duration: Duration,
    #[serde(with = "secs_only_duration", rename = "overtimeHalfTimeDuration")]
    pub ot_half_time_duration: Duration,
    #[serde(with = "secs_only_duration", rename = "preOvertimeBreak")]
    pub pre_overtime_break: Duration,
    #[serde(with = "secs_only_duration", rename = "preSuddenDeathDuration")]
    pub pre_sudden_death_duration: Duration,
    #[serde(with = "secs_only_duration", rename = "minimumBreak")]
    pub minimum_break: Duration,
}

#[allow(clippy::from_over_into)]
impl Into<GameConfig> for TimingRule {
    fn into(self) -> GameConfig {
        let TimingRule {
            name: _,
            team_timeout_count,
            team_timeouts_counted_per_half,
            overtime_allowed,
            sudden_death_allowed,
            half_play_duration,
            half_time_duration,
            team_timeout_duration,
            ot_half_play_duration,
            ot_half_time_duration,
            pre_overtime_break,
            pre_sudden_death_duration,
            minimum_break,
        } = self;

        let GameConfig {
            penalty_shot_duration,
            nominal_break,
            post_game_duration,
            ..
        } = Default::default();

        GameConfig {
            num_team_timeouts_allowed: team_timeout_count,
            timeouts_counted_per_half: team_timeouts_counted_per_half,
            overtime_allowed,
            sudden_death_allowed,
            single_half: half_play_duration == Duration::ZERO,
            half_play_duration,
            half_time_duration,
            team_timeout_duration,
            penalty_shot_duration,
            ot_half_play_duration,
            ot_half_time_duration,
            pre_overtime_break,
            pre_sudden_death_duration,
            post_game_duration,
            nominal_break,
            minimum_break,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupType {
    Division,
    Pod,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlideDirection {
    Up,
    Down,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StandingsCalculation {
    Standard,
    SwapIfUpset {
        #[serde(rename = "startingRanks")]
        starting_ranks: Vec<ScheduledTeam>,
    },
    SlideIfUpset {
        #[serde(rename = "startingRanks")]
        starting_ranks: Vec<ScheduledTeam>,
        #[serde(rename = "slideDirection")]
        slide_direction: SlideDirection,
    },
}

mod option_standings_calculation {
    use super::{ScheduledTeam, SlideDirection, StandingsCalculation};
    use serde::{Deserialize, Deserializer, Serialize, Serializer, ser::SerializeStruct};

    pub fn serialize<S>(
        value: &Option<StandingsCalculation>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(standings_calculation) => standings_calculation.serialize(serializer),
            None => {
                let mut state = serializer.serialize_struct("StandingsCalculation", 1)?;
                state.serialize_field("type", "None")?;
                state.end()
            }
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<StandingsCalculation>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(tag = "type")]
        enum StandingsCalculationHelper {
            Standard,
            SwapIfUpset {
                #[serde(rename = "startingRanks")]
                starting_ranks: Vec<ScheduledTeam>,
            },
            SlideIfUpset {
                #[serde(rename = "startingRanks")]
                starting_ranks: Vec<ScheduledTeam>,
                #[serde(rename = "slideDirection")]
                slide_direction: SlideDirection,
            },
            #[serde(rename = "None")]
            None,
        }

        let helper = StandingsCalculationHelper::deserialize(deserializer)?;
        Ok(match helper {
            StandingsCalculationHelper::Standard => Some(StandingsCalculation::Standard),
            StandingsCalculationHelper::SwapIfUpset { starting_ranks } => {
                Some(StandingsCalculation::SwapIfUpset { starting_ranks })
            }
            StandingsCalculationHelper::SlideIfUpset {
                starting_ranks,
                slide_direction,
            } => Some(StandingsCalculation::SlideIfUpset {
                starting_ranks,
                slide_direction,
            }),
            StandingsCalculationHelper::None => None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FinalResults {
    Standings,
    ListOfGames {
        #[serde(rename = "listOfGames")]
        list_of_games: Vec<ResultOf>,
    },
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    #[serde(rename = "shortName")]
    pub short_name: String,
    #[serde(rename = "type")]
    pub group_type: GroupType,
    #[serde(rename = "gameNumbers")]
    pub game_numbers: Vec<GameNumber>,
    #[serde(rename = "standingsCalculation", with = "option_standings_calculation")]
    pub standings_calculation: Option<StandingsCalculation>,
    #[serde(rename = "finalResultsCalculation")]
    pub final_results: Option<FinalResults>,
}

pub type GameList = BTreeMap<GameNumber, Game>;

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schedule {
    #[serde(rename = "eventId")]
    pub event_id: EventId,
    pub games: GameList,
    #[serde(rename = "nonGameEntries")]
    pub non_game_entries: Vec<NonGameEntry>,
    pub groups: Vec<Group>,
    #[serde(rename = "timingRules")]
    pub timing_rules: Vec<TimingRule>,
    #[serde(rename = "standingsOrder")]
    pub standings_order: Option<Vec<usize>>,
    #[serde(rename = "finalResultsOrder")]
    pub final_results_order: Option<Vec<usize>>,
}

impl Schedule {
    pub fn get_game_and_timing(
        &self,
        game_number: &GameNumber,
    ) -> (Option<&Game>, Option<&TimingRule>) {
        let game = self.games.get(game_number);
        let timing_rule =
            game.and_then(|g| self.timing_rules.iter().find(|tr| tr.name == g.timing_rule));
        (game, timing_rule)
    }

    pub fn get_game_timing(&self, game_number: &GameNumber) -> Option<&TimingRule> {
        self.games
            .get(game_number)
            .and_then(|g| self.timing_rules.iter().find(|tr| tr.name == g.timing_rule))
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

mod item_name {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(name: &str, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::Serialize;
        #[derive(Serialize)]
        struct ItemName<'a> {
            name: &'a str,
        }

        ItemName { name }.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct ItemName {
            name: String,
        }

        let item_name = ItemName::deserialize(deserializer)?;
        Ok(item_name.name)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, PartialOrd, Ord)]
pub struct EventId(String);

impl core::fmt::Debug for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <String as core::fmt::Debug>::fmt(&self.0, f)
    }
}

impl<'de> Deserialize<'de> for EventId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        EventId::from_full(s).map_err(serde::de::Error::custom)
    }
}

impl EventId {
    pub fn from_full<S: ToString>(full_id: S) -> Result<Self, &'static str> {
        let full_id = full_id.to_string();
        if full_id.starts_with("events/") && full_id.len() >= "events/".len() + 3 {
            Ok(Self(full_id))
        } else {
            Err("Invalid format for full_id. It should start with 'events/'")
        }
    }

    pub fn from_partial<S: ToString>(id: S) -> Self {
        Self(format!("events/{}", id.to_string()))
    }

    pub fn partial(&self) -> &str {
        self.0.strip_prefix("events/").unwrap_or(&self.0)
    }

    pub fn full(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Event ID {}", self.0)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, PartialOrd, Ord)]
pub struct TeamId(String);

impl core::fmt::Debug for TeamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <String as core::fmt::Debug>::fmt(&self.0, f)
    }
}

impl<'de> Deserialize<'de> for TeamId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        TeamId::from_full(s).map_err(serde::de::Error::custom)
    }
}

impl TeamId {
    pub fn from_full<S: ToString>(full_id: S) -> Result<Self, &'static str> {
        let full_id = full_id.to_string();
        if full_id.starts_with("teams/") && full_id.len() >= "teams/".len() + 3 {
            Ok(Self(full_id))
        } else {
            Err("Invalid format for full_id. It should start with 'teams/'")
        }
    }

    pub fn from_partial<S: ToString>(id: S) -> Self {
        Self(format!("teams/{}", id.to_string()))
    }

    pub fn partial(&self) -> &str {
        self.0.strip_prefix("teams/").unwrap_or(&self.0)
    }

    pub fn full(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TeamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Team ID {}", self.0)
    }
}

pub type TeamList = BTreeMap<TeamId, String>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateRange {
    #[serde(with = "iso8601_4dig_year_no_subsecs", rename = "startsOn")]
    pub start: OffsetDateTime,
    #[serde(with = "iso8601_4dig_year_no_subsecs", rename = "endsOn")]
    pub end: OffsetDateTime,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub name: String,
    pub slug: String,
    #[serde(rename = "dateRange")]
    pub date_range: DateRange,
    pub teams: Option<TeamList>,
    pub schedule: Option<Schedule>,
    pub courts: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Cred, Error, FetchOptions, RemoteCallbacks, Repository};
    use std::{env, fs::File, io::BufReader, path::Path};
    use time::macros::datetime;

    const SCHEDULE_JSON_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/uwhportal_postman_schedule_example.json"
    );

    #[test]
    fn test_serialize_winner() {
        let winner = ScheduledTeam::new_winner_of("2");
        let serialized = serde_json::to_string(&winner).unwrap();
        assert_eq!(
            serialized,
            r#"{"resultOf":{"type":"Winner","gameNumber":"2"}}"#
        );
    }

    #[test]
    fn test_serialize_loser() {
        let loser = ScheduledTeam::new_loser_of("2");
        let serialized = serde_json::to_string(&loser).unwrap();
        assert_eq!(
            serialized,
            r#"{"resultOf":{"type":"Loser","gameNumber":"2"}}"#
        );
    }

    #[test]
    fn test_serialize_team_id() {
        let team_id = ScheduledTeam::new_team_id(TeamId::from_partial("1-A"));
        let serialized = serde_json::to_string(&team_id).unwrap();
        assert_eq!(serialized, r#"{"teamId":"teams/1-A"}"#);
    }

    #[test]
    fn test_serialize_pending_assignment() {
        let pending_assignment = ScheduledTeam::new_pending_assignment_name("Team B");
        let serialized = serde_json::to_string(&pending_assignment).unwrap();
        assert_eq!(serialized, r#"{"pendingAssignmentName":"Team B"}"#);
    }

    #[test]
    fn test_serialize_seeded_by() {
        let seeded_by = ScheduledTeam::new_seeded_by(1, "A Group");
        let serialized = serde_json::to_string(&seeded_by).unwrap();
        assert_eq!(
            serialized,
            r#"{"seededBy":{"number":1,"group":{"name":"A Group"}}}"#
        );
    }

    #[test]
    fn test_deserialize_winner() {
        let winner_json = r#"{"resultOf":{"type":"Winner","gameNumber":"2"}}"#;
        let deserialized: ScheduledTeam = serde_json::from_str(winner_json).unwrap();
        assert_eq!(deserialized, ScheduledTeam::new_winner_of("2"));
    }

    #[test]
    fn test_deserialize_loser() {
        let loser_json = r#"{"resultOf":{"type":"Loser","gameNumber":"2"}}"#;
        let deserialized: ScheduledTeam = serde_json::from_str(loser_json).unwrap();
        assert_eq!(deserialized, ScheduledTeam::new_loser_of("2"));
    }

    #[test]
    fn test_deserialize_team_id() {
        let team_id_json = r#"{"teamId":"teams/1-A"}"#;
        let deserialized: ScheduledTeam = serde_json::from_str(team_id_json).unwrap();
        assert_eq!(
            deserialized,
            ScheduledTeam::new_team_id(TeamId::from_partial("1-A"))
        );
    }

    #[test]
    fn test_deserialize_pending_assignment() {
        let pending_assignment_json = r#"{"pendingAssignmentName":"Team B"}"#;
        let deserialized: ScheduledTeam = serde_json::from_str(pending_assignment_json).unwrap();
        assert_eq!(
            deserialized,
            ScheduledTeam::new_pending_assignment_name("Team B")
        );
    }

    #[test]
    fn test_deserialize_seeded_by() {
        let seeded_by_json = r#"{"seededBy":{"number":1,"group":{"name":"A Group"}}}"#;
        let deserialized: ScheduledTeam = serde_json::from_str(seeded_by_json).unwrap();
        assert_eq!(deserialized, ScheduledTeam::new_seeded_by(1, "A Group"));
    }

    #[test]
    fn test_serialize_game() {
        let game = Game {
            number: "1".to_string(),
            light: ScheduledTeam::new_team_id(TeamId::from_partial("1-A")),
            dark: ScheduledTeam::new_team_id(TeamId::from_partial("2-A")),
            start_time: datetime!(2023-08-07 0:00 UTC),
            court: "A".to_string(),
            description: None,
            timing_rule: "RR".to_string(),
        };
        let serialized = serde_json::to_string(&game).unwrap();
        assert_eq!(
            serialized,
            r#"{"number":"1","dark":{"teamId":"teams/2-A"},"light":{"teamId":"teams/1-A"},"startsOn":"2023-08-07T00:00:00Z","court":"A","timingRule":{"name":"RR"}}"#
        );
    }

    #[test]
    fn test_deserialize_game() {
        let game_json = r#"{"number":"1","dark":{"teamId":"teams/2-A"},"light":{"teamId":"teams/1-A"},"startsOn":"2023-08-07T00:00:00Z","court":"A","timingRule":{"name":"RR"}}"#;
        let deserialized: Game = serde_json::from_str(game_json).unwrap();
        assert_eq!(
            deserialized,
            Game {
                number: "1".to_string(),
                light: ScheduledTeam::new_team_id(TeamId::from_partial("1-A")),
                dark: ScheduledTeam::new_team_id(TeamId::from_partial("2-A")),
                start_time: datetime!(2023-08-07 0:00 UTC),
                court: "A".to_string(),
                description: None,
                timing_rule: "RR".to_string(),
            }
        );
    }

    #[test]
    fn test_serialize_timing_rule() {
        let timing_rule = TimingRule {
            name: "RR".to_string(),
            team_timeout_count: 1,
            team_timeouts_counted_per_half: true,
            overtime_allowed: true,
            sudden_death_allowed: true,
            half_play_duration: Duration::from_secs(900),
            half_time_duration: Duration::from_secs(180),
            team_timeout_duration: Duration::from_secs(60),
            ot_half_play_duration: Duration::from_secs(300),
            ot_half_time_duration: Duration::from_secs(180),
            pre_overtime_break: Duration::from_secs(180),
            pre_sudden_death_duration: Duration::from_secs(60),
            minimum_break: Duration::from_secs(240),
        };
        let serialized = serde_json::to_string(&timing_rule).unwrap();
        assert_eq!(
            serialized,
            r#"{"name":"RR","teamTimeoutCount":1,"teamTimeoutsCountedPerHalf":true,"overtimeAllowed":true,"suddenDeathAllowed":true,"halfPlayDuration":900,"halfTimeDuration":180,"teamTimeoutDuration":60,"overtimeHalfPlayDuration":300,"overtimeHalfTimeDuration":180,"preOvertimeBreak":180,"preSuddenDeathDuration":60,"minimumBreak":240}"#
        );
    }

    #[test]
    fn test_deserialize_timing_rule() {
        let timing_rule_json = r#"{"name":"RR","teamTimeoutCount":1,"teamTimeoutsCountedPerHalf":true,"overtimeAllowed":true,"suddenDeathAllowed":true,"halfPlayDuration":900,"halfTimeDuration":180,"teamTimeoutDuration":60,"overtimeHalfPlayDuration":300,"overtimeHalfTimeDuration":180,"preOvertimeBreak":180,"preSuddenDeathDuration":60,"minimumBreak":240}"#;
        let deserialized: TimingRule = serde_json::from_str(timing_rule_json).unwrap();
        assert_eq!(
            deserialized,
            TimingRule {
                name: "RR".to_string(),
                team_timeout_count: 1,
                team_timeouts_counted_per_half: true,
                overtime_allowed: true,
                sudden_death_allowed: true,
                half_play_duration: Duration::from_secs(900),
                half_time_duration: Duration::from_secs(180),
                team_timeout_duration: Duration::from_secs(60),
                ot_half_play_duration: Duration::from_secs(300),
                ot_half_time_duration: Duration::from_secs(180),
                pre_overtime_break: Duration::from_secs(180),
                pre_sudden_death_duration: Duration::from_secs(60),
                minimum_break: Duration::from_secs(240),
            }
        );
    }

    #[test]
    fn test_serialize_standard_standings() {
        let standings = StandingsCalculation::Standard;
        let serialized = serde_json::to_string(&standings).unwrap();
        assert_eq!(serialized, r#"{"type":"Standard"}"#);
    }

    #[test]
    fn test_deserialize_standard_standings() {
        let standings_json = r#"{"type":"Standard"}"#;
        let deserialized: StandingsCalculation = serde_json::from_str(standings_json).unwrap();
        assert_eq!(deserialized, StandingsCalculation::Standard);
    }

    #[test]
    fn test_serialize_sawp_if_upset_standings() {
        let standings = StandingsCalculation::SwapIfUpset {
            starting_ranks: vec![
                ScheduledTeam::new_seeded_by(1, "A Group"),
                ScheduledTeam::new_seeded_by(2, "A Group"),
            ],
        };
        let serialized = serde_json::to_string(&standings).unwrap();
        assert_eq!(
            serialized,
            r#"{"type":"SwapIfUpset","startingRanks":[{"seededBy":{"number":1,"group":{"name":"A Group"}}},{"seededBy":{"number":2,"group":{"name":"A Group"}}}]}"#
        );
    }

    #[test]
    fn test_deserialize_sawp_if_upset_standings() {
        let standings_json = r#"{"type":"SwapIfUpset","startingRanks":[{"seededBy":{"number":1,"group":{"name":"A Group"}}},{"seededBy":{"number":2,"group":{"name":"A Group"}}}]}"#;
        let deserialized: StandingsCalculation = serde_json::from_str(standings_json).unwrap();
        assert_eq!(
            deserialized,
            StandingsCalculation::SwapIfUpset {
                starting_ranks: vec![
                    ScheduledTeam::new_seeded_by(1, "A Group"),
                    ScheduledTeam::new_seeded_by(2, "A Group"),
                ],
            }
        );
    }

    #[test]
    fn test_serialize_slide_if_upset_standings() {
        let standings = StandingsCalculation::SlideIfUpset {
            slide_direction: SlideDirection::Down,
            starting_ranks: vec![
                ScheduledTeam::new_winner_of("2"),
                ScheduledTeam::new_loser_of("2"),
            ],
        };
        let serialized = serde_json::to_string(&standings).unwrap();
        assert_eq!(
            serialized,
            r#"{"type":"SlideIfUpset","startingRanks":[{"resultOf":{"type":"Winner","gameNumber":"2"}},{"resultOf":{"type":"Loser","gameNumber":"2"}}],"slideDirection":"Down"}"#
        );
    }

    #[test]
    fn test_deserialize_slide_if_upset_standings() {
        let standings_json = r#"{"type":"SlideIfUpset","startingRanks":[{"resultOf":{"type":"Winner","gameNumber":"2"}},{"resultOf":{"type":"Loser","gameNumber":"2"}}],"slideDirection":"Down"}"#;
        let deserialized: StandingsCalculation = serde_json::from_str(standings_json).unwrap();
        assert_eq!(
            deserialized,
            StandingsCalculation::SlideIfUpset {
                slide_direction: SlideDirection::Down,
                starting_ranks: vec![
                    ScheduledTeam::new_winner_of("2"),
                    ScheduledTeam::new_loser_of("2")
                ],
            }
        );
    }

    #[test]
    fn test_serialize_standings_final_results() {
        let final_results = FinalResults::Standings;
        let serialized = serde_json::to_string(&final_results).unwrap();
        assert_eq!(serialized, r#"{"type":"Standings"}"#);
    }

    #[test]
    fn test_deserialize_standings_final_results() {
        let final_results_json = r#"{"type":"Standings"}"#;
        let deserialized: FinalResults = serde_json::from_str(final_results_json).unwrap();
        assert_eq!(deserialized, FinalResults::Standings);
    }

    #[test]
    fn test_serialize_list_of_games_final_results() {
        let final_results = FinalResults::ListOfGames {
            list_of_games: vec![
                ResultOf::Winner {
                    game_number: "4".to_string(),
                },
                ResultOf::Loser {
                    game_number: "4".to_string(),
                },
            ],
        };
        let serialized = serde_json::to_string(&final_results).unwrap();
        assert_eq!(
            serialized,
            r#"{"type":"ListOfGames","listOfGames":[{"type":"Winner","gameNumber":"4"},{"type":"Loser","gameNumber":"4"}]}"#
        );
    }

    #[test]
    fn test_deserialize_list_of_games_final_results() {
        let final_results_json = r#"{"type":"ListOfGames","listOfGames":[{"type":"Winner","gameNumber":"4"},{"type":"Loser","gameNumber":"4"}]}"#;
        let deserialized: FinalResults = serde_json::from_str(final_results_json).unwrap();
        assert_eq!(
            deserialized,
            FinalResults::ListOfGames {
                list_of_games: vec![
                    ResultOf::Winner {
                        game_number: "4".to_string()
                    },
                    ResultOf::Loser {
                        game_number: "4".to_string()
                    },
                ],
            }
        );
    }

    #[test]
    fn test_serialize_group() {
        let group = Group {
            name: "A Group".to_string(),
            short_name: "A".to_string(),
            group_type: GroupType::Pod,
            game_numbers: vec!["1".to_string()],
            standings_calculation: Some(StandingsCalculation::Standard),
            final_results: Some(FinalResults::Standings),
        };
        let serialized = serde_json::to_string(&group).unwrap();
        assert_eq!(
            serialized,
            r#"{"name":"A Group","shortName":"A","type":"Pod","gameNumbers":["1"],"standingsCalculation":{"type":"Standard"},"finalResultsCalculation":{"type":"Standings"}}"#
        );
    }

    #[test]
    fn test_deserialize_group() {
        let group_json = r#"{"name":"A Group","shortName":"A","type":"Pod","gameNumbers":["1"],"standingsCalculation":{"type":"Standard"},"finalResultsCalculation":{"type":"Standings"}}"#;
        let deserialized: Group = serde_json::from_str(group_json).unwrap();
        assert_eq!(
            deserialized,
            Group {
                name: "A Group".to_string(),
                short_name: "A".to_string(),
                group_type: GroupType::Pod,
                game_numbers: vec!["1".to_string()],
                standings_calculation: Some(StandingsCalculation::Standard),
                final_results: Some(FinalResults::Standings),
            }
        );
    }

    #[test]
    #[ignore]
    fn test_serialize_and_deserialize_postman_schedule() {
        let schedule = Schedule {
            event_id: EventId::from_partial("1-A"),
            games: vec![
                Game {
                    number: "1".to_string(),
                    light: ScheduledTeam::new_team_id(TeamId::from_partial("1-A")),
                    dark: ScheduledTeam::new_team_id(TeamId::from_partial("2-A")),
                    start_time: datetime!(2023-08-07 0:00 UTC),
                    court: "A".to_string(),
                    description: None,
                    timing_rule: "RR".to_string(),
                },
                Game {
                    number: "2".to_string(),
                    light: ScheduledTeam::new_pending_assignment_name("Team A"),
                    dark: ScheduledTeam::new_pending_assignment_name("Team B"),
                    start_time: datetime!(2023-08-07 1:00 UTC),
                    court: "A".to_string(),
                    description: None,
                    timing_rule: "RR".to_string(),
                },
                Game {
                    number: "3".to_string(),
                    light: ScheduledTeam::new_seeded_by(1, "A Group"),
                    dark: ScheduledTeam::new_seeded_by(2, "A Group"),
                    start_time: datetime!(2023-08-07 2:00 UTC),
                    court: "A".to_string(),
                    description: None,
                    timing_rule: "RR".to_string(),
                },
                Game {
                    number: "4".to_string(),
                    light: ScheduledTeam::new_winner_of("2"),
                    dark: ScheduledTeam::new_loser_of("2"),
                    start_time: datetime!(2023-08-07 3:00 UTC),
                    court: "A".to_string(),
                    description: None,
                    timing_rule: "RR".to_string(),
                },
            ]
            .into_iter()
            .map(|game| (game.number.clone(), game))
            .collect(),
            non_game_entries: vec![],
            groups: vec![
                Group {
                    name: "A Group".to_string(),
                    short_name: "A".to_string(),
                    group_type: GroupType::Pod,
                    game_numbers: vec!["1".to_string()],
                    standings_calculation: Some(StandingsCalculation::Standard),
                    final_results: Some(FinalResults::Standings),
                },
                Group {
                    name: "B Group - No calculations".to_string(),
                    short_name: "B".to_string(),
                    group_type: GroupType::Pod,
                    game_numbers: vec!["2".to_string()],
                    standings_calculation: None,
                    final_results: None,
                },
                Group {
                    name: "C Group - Swap if Upset".to_string(),
                    short_name: "C".to_string(),
                    group_type: GroupType::Pod,
                    game_numbers: vec!["3".to_string()],
                    standings_calculation: Some(StandingsCalculation::SwapIfUpset {
                        starting_ranks: vec![
                            ScheduledTeam::new_seeded_by(1, "A Group"),
                            ScheduledTeam::new_seeded_by(2, "A Group"),
                        ],
                    }),
                    final_results: None,
                },
                Group {
                    name: "d Group - Slide if Upset".to_string(),
                    short_name: "D".to_string(),
                    group_type: GroupType::Pod,
                    game_numbers: vec!["4".to_string()],
                    standings_calculation: Some(StandingsCalculation::SlideIfUpset {
                        slide_direction: SlideDirection::Down,
                        starting_ranks: vec![
                            ScheduledTeam::new_winner_of("2"),
                            ScheduledTeam::new_loser_of("2"),
                        ],
                    }),
                    final_results: Some(FinalResults::ListOfGames {
                        list_of_games: vec![
                            ResultOf::Winner {
                                game_number: "4".to_string(),
                            },
                            ResultOf::Loser {
                                game_number: "4".to_string(),
                            },
                        ],
                    }),
                },
            ],
            timing_rules: vec![TimingRule {
                name: "RR".to_string(),
                team_timeout_count: 1,
                team_timeouts_counted_per_half: true,
                overtime_allowed: true,
                sudden_death_allowed: true,
                half_play_duration: Duration::from_secs(900),
                half_time_duration: Duration::from_secs(180),
                team_timeout_duration: Duration::from_secs(60),
                ot_half_play_duration: Duration::from_secs(300),
                ot_half_time_duration: Duration::from_secs(180),
                pre_overtime_break: Duration::from_secs(180),
                pre_sudden_death_duration: Duration::from_secs(60),
                minimum_break: Duration::from_secs(240),
            }],
            standings_order: None,
            final_results_order: None,
        };
        let serialized = serde_json::to_string(&schedule).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        let file = File::open(SCHEDULE_JSON_PATH).unwrap();
        let reader = BufReader::new(file);
        let expected: serde_json::Value = serde_json::from_reader(reader).unwrap();

        assert_eq!(deserialized, expected);
    }

    #[test]
    #[ignore]
    fn check_postman_example_is_updated() {
        const UWHPORTAL_REPO_PATH: &str = "../../uwhscores";
        const POSTMAN_JSON_PATH: &str = "assets/Underwater.postman_collection.json";

        // Open the Git repository
        let repo = Repository::open(UWHPORTAL_REPO_PATH).unwrap();

        // Get the HEAD reference (usually "refs/heads/master" or similar)
        let head_ref = repo.head().unwrap();

        // Extract the branch name from the HEAD reference
        let branch_name = head_ref
            .shorthand()
            .ok_or(Error::from_str("HEAD is not pointing to a valid branch"))
            .unwrap();

        // Iterate through remotes and find the one that matches the branch name
        let mut default_remote_name: Option<String> = None;
        for remote in repo.remotes().unwrap().iter() {
            let remote_name = remote.unwrap();
            let remote_ref = format!("refs/remotes/{}/{}", remote_name, branch_name);

            // Check if the remote_ref exists
            if repo.revparse_single(&remote_ref).is_ok() {
                default_remote_name = Some(remote_name.to_string());
                break;
            }
        }

        // Find the default remote by checking the remote's HEAD reference
        let default_remote_name = default_remote_name.expect("Couldn't find default remote");

        // Set up the credentials callback. This will be called when the Git library needs to authenticate with the remote.
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key(
                username_from_url.unwrap(),
                None,
                std::path::Path::new(&format!("{}/.ssh/id_ed25519", env::var("HOME").unwrap())),
                None,
            )
        });
        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        // Fetch updates from the default remote
        let mut default_remote = repo.find_remote(&default_remote_name).unwrap();
        default_remote
            .fetch::<&str>(&[], Some(&mut fetch_options), None)
            .unwrap();

        // Get the HEAD commit of the default remote
        let default_remote_head_ref = repo
            .revparse_single(&format!("{default_remote_name}/{branch_name}"))
            .unwrap();
        let default_remote_head_commit = default_remote_head_ref.peel_to_commit().unwrap();

        // Get the OID (object ID) of the HEAD commit for the file in the local repository
        let file_oid = repo
            .revparse_single(&format!("HEAD:{}", POSTMAN_JSON_PATH))
            .unwrap()
            .id();

        // Get the OID (object ID) of the HEAD commit for the file in the default remote
        let default_remote_file_oid = default_remote_head_commit
            .tree()
            .unwrap()
            .get_path(Path::new(POSTMAN_JSON_PATH))
            .unwrap()
            .id();

        // Compare the OIDs to check if the file is up to date
        assert_eq!(file_oid, default_remote_file_oid);

        let file = File::open(&format!("{UWHPORTAL_REPO_PATH}/{POSTMAN_JSON_PATH}")).unwrap();
        let reader = BufReader::new(file);
        let postman: serde_json::Value = serde_json::from_reader(reader).unwrap();

        let updated = if let serde_json::Value::Array(ref list) = postman["item"] {
            let schedule: Vec<_> = list
                .iter()
                .filter(|val| val["name"] == "Schedule")
                .take(1)
                .collect();
            assert_eq!(schedule.len(), 1);
            let details: Vec<_> =
                if let serde_json::Value::Array(ref schedule) = schedule[0]["item"] {
                    schedule
                        .iter()
                        .filter(|val| val["name"] == "Create event schedule")
                        .take(1)
                        .collect()
                } else {
                    panic!("Schedule[\"item\"] is not an array");
                };
            assert_eq!(details.len(), 1);
            &details[0]["request"]["body"]["raw"]
        } else {
            panic!("postman[\"item\"] is not an array");
        };

        let updated = updated
            .as_str()
            .unwrap()
            .to_string()
            .replace("\\n", "\n")
            .replace("\\r", "\r");
        let updated: serde_json::Value = serde_json::from_str(&updated).unwrap();

        let file = File::open(SCHEDULE_JSON_PATH).unwrap();
        let reader = BufReader::new(file);
        let current: serde_json::Value = serde_json::from_reader(reader).unwrap();

        assert_eq!(updated, current);
    }
}
