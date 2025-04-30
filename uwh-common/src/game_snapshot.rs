use crate::{
    bundles::{BlackWhiteBundle, OptColorBundle},
    color::Color,
    drawing_support::*,
};
#[cfg(feature = "std")]
use crate::{config::Game, uwhportal::schedule::EventId};
use arrayref::array_ref;
use arrayvec::ArrayVec;
use core::cmp::{Ordering, PartialOrd};
#[cfg(feature = "std")]
use core::{cmp::min, time::Duration};
#[cfg(not(target_os = "windows"))]
use defmt::Format;
use derivative::Derivative;
use displaydoc::Display;
use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};
#[cfg(feature = "std")]
use time::Duration as SignedDuration;

const PANEL_PENALTY_COUNT: usize = 3;

/// Game snapshot information that the LED matrices need. Excludes some fields, limits to three
/// penalties (the three with the lowest remaining time), and places the penalties on a stack-based
/// `ArrayVec`, instead of the heap-based `Vec`
#[derive(Debug, PartialEq, Eq, Default, Clone, Serialize, Deserialize)]
pub struct GameSnapshotNoHeap {
    pub current_period: GamePeriod,
    pub secs_in_period: u16,
    pub timeout: Option<TimeoutSnapshot>,
    pub scores: BlackWhiteBundle<u8>,
    pub penalties: BlackWhiteBundle<ArrayVec<PenaltySnapshot, PANEL_PENALTY_COUNT>>,
    pub timeouts_available: BlackWhiteBundle<bool>,
    pub is_old_game: bool,
}

/// All the information needed by a UI to draw the current state of the game. Requires the `std`
/// feature.
#[cfg(feature = "std")]
#[derive(Debug, PartialEq, Eq, Default, Clone, Serialize, Deserialize)]
pub struct GameSnapshot {
    pub current_period: GamePeriod,
    pub secs_in_period: u32,
    pub timeout: Option<TimeoutSnapshot>,
    pub scores: BlackWhiteBundle<u8>,
    pub penalties: BlackWhiteBundle<Vec<PenaltySnapshot>>,
    pub warnings: BlackWhiteBundle<Vec<InfractionSnapshot>>,
    pub fouls: OptColorBundle<Vec<InfractionSnapshot>>,
    pub timeouts_available: BlackWhiteBundle<bool>,
    pub is_old_game: bool,
    pub game_number: u32,
    pub next_game_number: u32,
    pub event_id: Option<EventId>,
    pub recent_goal: Option<(Color, u8)>,
    pub next_period_len_secs: Option<u32>,
}

#[cfg(feature = "std")]
impl From<GameSnapshot> for GameSnapshotNoHeap {
    fn from(snapshot: GameSnapshot) -> Self {
        let penalties = snapshot
            .penalties
            .into_iter()
            .map(|(c, mut orig)| {
                orig.retain(|pen| {
                    if let PenaltyTime::Seconds(secs) = pen.time {
                        secs != 0
                    } else {
                        true
                    }
                });
                orig.sort_by(|a, b| a.time.cmp(&b.time));
                (c, orig.into_iter().take(3).collect())
            })
            .collect();

        Self {
            current_period: snapshot.current_period,
            secs_in_period: min(
                snapshot
                    .secs_in_period
                    .try_into()
                    .unwrap_or(MAX_STRINGABLE_SECS),
                MAX_STRINGABLE_SECS,
            ),
            timeout: snapshot.timeout,
            scores: snapshot.scores,
            penalties,
            timeouts_available: snapshot.timeouts_available,
            is_old_game: snapshot.is_old_game,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct PenaltySnapshot {
    pub player_number: u8,
    pub time: PenaltyTime,
    pub infraction: Infraction,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct InfractionSnapshot {
    pub player_number: Option<u8>,
    pub infraction: Infraction,
}

#[derive(Derivative, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[derivative(Debug, Default, Clone, Copy)]
pub enum GamePeriod {
    #[derivative(Default)]
    BetweenGames,
    FirstHalf,
    HalfTime,
    SecondHalf,
    PreOvertime,
    OvertimeFirstHalf,
    OvertimeHalfTime,
    OvertimeSecondHalf,
    PreSuddenDeath,
    SuddenDeath,
}

impl GamePeriod {
    #[cfg(feature = "std")]
    pub fn penalties_run(self, config: &Game) -> bool {
        match self {
            Self::BetweenGames
            | Self::HalfTime
            | Self::PreOvertime
            | Self::OvertimeHalfTime
            | Self::PreSuddenDeath => false,
            Self::FirstHalf | Self::SecondHalf => true,
            Self::OvertimeFirstHalf | Self::OvertimeSecondHalf => config.overtime_allowed,
            Self::SuddenDeath => config.sudden_death_allowed,
        }
    }

    #[cfg(feature = "std")]
    pub fn duration(self, config: &Game) -> Option<Duration> {
        match self {
            Self::BetweenGames | Self::SuddenDeath => None,
            Self::FirstHalf | Self::SecondHalf => Some(config.half_play_duration),
            Self::HalfTime => Some(config.half_time_duration),
            Self::PreOvertime => Some(config.pre_overtime_break),
            Self::OvertimeFirstHalf | Self::OvertimeSecondHalf => {
                Some(config.ot_half_play_duration)
            }
            Self::OvertimeHalfTime => Some(config.ot_half_time_duration),
            Self::PreSuddenDeath => Some(config.pre_sudden_death_duration),
        }
    }

    #[cfg(feature = "std")]
    pub fn time_elapsed_at(self, time: Duration, config: &Game) -> Option<SignedDuration> {
        match self {
            p @ Self::BetweenGames
            | p @ Self::FirstHalf
            | p @ Self::HalfTime
            | p @ Self::SecondHalf
            | p @ Self::PreOvertime
            | p @ Self::OvertimeFirstHalf
            | p @ Self::OvertimeHalfTime
            | p @ Self::OvertimeSecondHalf
            | p @ Self::PreSuddenDeath => p
                .duration(config)
                .and_then(|d| d.try_into().ok().map(|sd: SignedDuration| sd - time)),
            Self::SuddenDeath => time.try_into().ok(),
        }
    }

    #[cfg(feature = "std")]
    pub fn time_between(self, start: SignedDuration, end: SignedDuration) -> SignedDuration {
        match self {
            Self::BetweenGames
            | Self::FirstHalf
            | Self::HalfTime
            | Self::SecondHalf
            | Self::PreOvertime
            | Self::OvertimeFirstHalf
            | Self::OvertimeHalfTime
            | Self::OvertimeSecondHalf
            | Self::PreSuddenDeath => start - end,
            Self::SuddenDeath => end - start,
        }
    }

    pub fn next_period(self) -> Option<GamePeriod> {
        match self {
            Self::BetweenGames => Some(Self::FirstHalf),
            Self::FirstHalf => Some(Self::HalfTime),
            Self::HalfTime => Some(Self::SecondHalf),
            Self::SecondHalf => Some(Self::PreOvertime),
            Self::PreOvertime => Some(Self::OvertimeFirstHalf),
            Self::OvertimeFirstHalf => Some(Self::OvertimeHalfTime),
            Self::OvertimeHalfTime => Some(Self::OvertimeSecondHalf),
            Self::OvertimeSecondHalf => Some(Self::PreSuddenDeath),
            Self::PreSuddenDeath => Some(Self::SuddenDeath),
            Self::SuddenDeath => None,
        }
    }

    #[cfg(feature = "std")]
    pub fn next_period_dur(self, config: &Game) -> Option<Duration> {
        match self.next_period()? {
            Self::BetweenGames => None,
            Self::FirstHalf => Some(config.half_play_duration),
            Self::HalfTime => Some(config.half_time_duration),
            Self::SecondHalf => Some(config.half_play_duration),
            Self::PreOvertime => {
                if config.overtime_allowed {
                    Some(config.pre_overtime_break)
                } else if config.sudden_death_allowed {
                    Some(config.pre_sudden_death_duration)
                } else {
                    None
                }
            }
            Self::OvertimeFirstHalf => {
                if config.overtime_allowed {
                    Some(config.ot_half_play_duration)
                } else {
                    None
                }
            }
            Self::OvertimeHalfTime => {
                if config.overtime_allowed {
                    Some(config.ot_half_time_duration)
                } else {
                    None
                }
            }
            Self::OvertimeSecondHalf => {
                if config.overtime_allowed {
                    Some(config.ot_half_play_duration)
                } else {
                    None
                }
            }
            Self::PreSuddenDeath => {
                if config.sudden_death_allowed {
                    Some(config.pre_sudden_death_duration)
                } else {
                    None
                }
            }
            Self::SuddenDeath => None,
        }
    }
}

impl core::fmt::Display for GamePeriod {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match *self {
            GamePeriod::BetweenGames => write!(f, "Between Games"),
            GamePeriod::FirstHalf => write!(f, "First Half"),
            GamePeriod::HalfTime => write!(f, "Half Time"),
            GamePeriod::SecondHalf => write!(f, "Second Half"),
            GamePeriod::PreOvertime => write!(f, "Pre Overtime"),
            GamePeriod::OvertimeFirstHalf => write!(f, "Overtime First Half"),
            GamePeriod::OvertimeHalfTime => write!(f, "Overtime Half Time"),
            GamePeriod::OvertimeSecondHalf => write!(f, "Overtime Second Half"),
            GamePeriod::PreSuddenDeath => write!(f, "Pre Sudden Death"),
            GamePeriod::SuddenDeath => write!(f, "Sudden Death"),
        }
    }
}

#[derive(Derivative, Serialize, Deserialize)]
#[derivative(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub enum TimeoutSnapshot {
    #[derivative(Default)]
    White(u16),
    Black(u16),
    Ref(u16),
    PenaltyShot(u16),
}

impl core::fmt::Display for TimeoutSnapshot {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match *self {
            TimeoutSnapshot::Black(_) => write!(f, "Black Timeout"),
            TimeoutSnapshot::White(_) => write!(f, "White Timeout"),
            TimeoutSnapshot::Ref(_) => write!(f, "Ref Timeout"),
            TimeoutSnapshot::PenaltyShot(_) => write!(f, "PenaltyShot"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum PenaltyTime {
    Seconds(u16),
    TotalDismissal,
}

impl Ord for PenaltyTime {
    fn cmp(&self, other: &Self) -> Ordering {
        match self {
            PenaltyTime::TotalDismissal => match other {
                PenaltyTime::TotalDismissal => Ordering::Equal,
                PenaltyTime::Seconds(_) => Ordering::Greater,
            },
            PenaltyTime::Seconds(mine) => match other {
                PenaltyTime::Seconds(theirs) => mine.cmp(theirs),
                PenaltyTime::TotalDismissal => Ordering::Less,
            },
        }
    }
}

impl PartialOrd for PenaltyTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Derivative, Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Sequence)]
#[derivative(Default)]
pub enum Infraction {
    #[derivative(Default)]
    Unknown,
    StickInfringement,
    IllegalAdvancement,
    IllegalSubstitution,
    IllegallyStoppingThePuck,
    OutOfBounds,
    GrabbingTheBarrier,
    Obstruction,
    DelayOfGame,
    UnsportsmanlikeConduct,
    FreeArm,
    FalseStart,
}

impl Infraction {
    pub fn short_name(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::StickInfringement => "Stick Foul",
            Self::IllegalAdvancement => "Illegal Advance",
            Self::IllegalSubstitution => "Sub Foul",
            Self::IllegallyStoppingThePuck => "Illegal Stoppage",
            Self::OutOfBounds => "Out Of Bounds",
            Self::GrabbingTheBarrier => "Grabbing The Wall",
            Self::Obstruction => "Obstruction",
            Self::DelayOfGame => "Delay Of Game",
            Self::UnsportsmanlikeConduct => "Unsportsmanlike",
            Self::FreeArm => "Free Arm",
            Self::FalseStart => "False Start",
        }
    }

    pub fn get_image(&self) -> &'static [u8] {
        match self {
            Self::Unknown => include_bytes!(
                "../../refbox/resources/Atlantis_infractions/Black_question_mark.png"
            ),
            Self::StickInfringement => include_bytes!(
                "../../refbox/resources/Atlantis_infractions/Stick_Infringement_smaller.png"
            ),
            Self::IllegalAdvancement => include_bytes!(
                "../../refbox/resources/Atlantis_infractions/Illegal_Advancement_smaller.png"
            ),
            Self::IllegalSubstitution => include_bytes!(
                "../../refbox/resources/Atlantis_infractions/Illegal_Substitution_smaller.png"
            ),
            Self::IllegallyStoppingThePuck => include_bytes!(
                "../../refbox/resources/Atlantis_infractions/Illegal_Knockdown_smaller.png"
            ),
            Self::OutOfBounds => include_bytes!(
                "../../refbox/resources/Atlantis_infractions/Out_of_Bounds_smaller.png"
            ),
            Self::GrabbingTheBarrier => include_bytes!(
                "../../refbox/resources/Atlantis_infractions/Grabbing_Barrier_smaller.png"
            ),
            Self::Obstruction => include_bytes!(
                "../../refbox/resources/Atlantis_infractions/Obstruction_smaller.png"
            ),
            Self::DelayOfGame => include_bytes!(
                "../../refbox/resources/Atlantis_infractions/Delay_of_Game_smaller.png"
            ),
            Self::UnsportsmanlikeConduct => {
                include_bytes!("../../refbox/resources/Atlantis_infractions/Unsporting_smaller.png")
            }
            Self::FreeArm => {
                include_bytes!("../../refbox/resources/Atlantis_infractions/Free_Arm_smaller.png")
            }
            Self::FalseStart => include_bytes!(
                "../../refbox/resources/Atlantis_infractions/False_Start_smaller.png"
            ),
        }
    }
}

#[cfg_attr(not(target_os = "windows"), derive(Format))]
#[derive(Debug, Display, PartialEq, Eq, Clone)]
pub enum EncodingError {
    /// Player number was more than two digits: {0}
    PlayerNumTooLarge(u8),
    /// Penalty time too large: {0}
    PenaltyTimeTooLarge(u16),
    /// Timeout time was too large for a u16: {0}
    TimeoutTimeTooLarge(u16),
}

#[cfg(feature = "std")]
impl std::error::Error for EncodingError {}

#[cfg_attr(not(target_os = "windows"), derive(Format))]
#[derive(Debug, Display, PartialEq, Eq, Clone)]
pub enum DecodingError {
    /// Invalid timeout type: {0:#06x}
    InvalidTimeoutType(u16),
    /// Invalid game period: {0:#04x}
    InvalidGamePeriod(u8),
}

#[cfg(feature = "std")]
impl std::error::Error for DecodingError {}

impl PenaltySnapshot {
    pub fn encode(&self) -> Result<[u8; 2], EncodingError> {
        if self.player_number >= 100 {
            return Err(EncodingError::PlayerNumTooLarge(self.player_number));
        }
        if let PenaltyTime::Seconds(time @ 511..) = self.time {
            return Err(EncodingError::PenaltyTimeTooLarge(time));
        }
        let num = (self.player_number as u16) << 9;
        let time = match self.time {
            PenaltyTime::Seconds(time) => time,
            PenaltyTime::TotalDismissal => 511,
        };
        Ok((num | time).to_be_bytes())
    }

    pub fn encode_none() -> [u8; 2] {
        [0xfe, 0x00]
    }

    pub fn decode(bytes: &[u8; 2]) -> Option<Self> {
        let val = u16::from_be_bytes(*bytes);
        if (val & 0xfe00) == 0xfe00 {
            return None;
        }
        Some(Self {
            player_number: ((val & 0xfe00) >> 9) as u8,
            time: match val & 0x01ff {
                0x01ff => PenaltyTime::TotalDismissal,
                time => PenaltyTime::Seconds(time),
            },
            infraction: Infraction::Unknown,
        })
    }
}

struct PeriodTimeoutAvailInfo {
    current_period: GamePeriod,
    is_old_game: bool,
    timeouts_available: BlackWhiteBundle<bool>,
}

impl PeriodTimeoutAvailInfo {
    fn encode(&self) -> u8 {
        let mut val = self.current_period.encode();
        val |= if self.is_old_game { 0x80 } else { 0x00 };
        val |= ((self.timeouts_available.black as u8) << 6)
            | ((self.timeouts_available.white as u8) << 5);
        val
    }

    fn decode(val: u8) -> Result<Self, DecodingError> {
        Ok(Self {
            current_period: GamePeriod::decode(val & 0x0f)?,
            is_old_game: (val & 0x80) != 0x00,
            timeouts_available: BlackWhiteBundle {
                black: (val & 0x40) != 0x00,
                white: (val & 0x20) != 0x00,
            },
        })
    }
}

impl TimeoutSnapshot {
    pub fn encode(&self) -> Result<[u8; 2], EncodingError> {
        match self {
            Self::Black(time) | Self::White(time) | Self::Ref(time) | Self::PenaltyShot(time) => {
                if *time > MAX_STRINGABLE_SECS {
                    Err(EncodingError::TimeoutTimeTooLarge(*time))
                } else {
                    let variant = match self {
                        Self::Black(_) => 0b0010_0000,
                        Self::White(_) => 0b0100_0000,
                        Self::Ref(_) => 0b0110_0000,
                        Self::PenaltyShot(_) => 0b1000_0000,
                    };
                    let mut arr = time.to_be_bytes();
                    arr[0] |= variant;
                    Ok(arr)
                }
            }
        }
    }

    pub fn decode(bytes: &[u8; 2]) -> Result<Option<Self>, DecodingError> {
        const TYPE_MASK: u16 = 0b1110_0000_0000_0000;
        const TIME_MASK: u16 = 0b0001_1111_1111_1111;
        let val = u16::from_be_bytes(*bytes);
        if (val & TYPE_MASK) == 0 {
            Ok(None)
        } else {
            let time = val & TIME_MASK;
            Ok(Some(match (val & TYPE_MASK) >> 13 {
                0b001 => TimeoutSnapshot::Black(time),
                0b010 => TimeoutSnapshot::White(time),
                0b011 => TimeoutSnapshot::Ref(time),
                0b100 => TimeoutSnapshot::PenaltyShot(time),
                _ => return Err(DecodingError::InvalidTimeoutType(val)),
            }))
        }
    }
}

impl GamePeriod {
    pub fn encode(&self) -> u8 {
        match self {
            Self::BetweenGames => 0,
            Self::FirstHalf => 1,
            Self::HalfTime => 2,
            Self::SecondHalf => 3,
            Self::PreOvertime => 4,
            Self::OvertimeFirstHalf => 5,
            Self::OvertimeHalfTime => 6,
            Self::OvertimeSecondHalf => 7,
            Self::PreSuddenDeath => 8,
            Self::SuddenDeath => 9,
        }
    }

    pub fn decode(val: u8) -> Result<Self, DecodingError> {
        match val {
            0 => Ok(Self::BetweenGames),
            1 => Ok(Self::FirstHalf),
            2 => Ok(Self::HalfTime),
            3 => Ok(Self::SecondHalf),
            4 => Ok(Self::PreOvertime),
            5 => Ok(Self::OvertimeFirstHalf),
            6 => Ok(Self::OvertimeHalfTime),
            7 => Ok(Self::OvertimeSecondHalf),
            8 => Ok(Self::PreSuddenDeath),
            9 => Ok(Self::SuddenDeath),
            _ => Err(DecodingError::InvalidGamePeriod(val)),
        }
    }
}

impl GameSnapshotNoHeap {
    pub const ENCODED_LEN: usize = 19;

    pub fn encode(&self) -> Result<[u8; Self::ENCODED_LEN], EncodingError> {
        let per_tm_avail = PeriodTimeoutAvailInfo {
            current_period: self.current_period,
            is_old_game: self.is_old_game,
            timeouts_available: self.timeouts_available,
        };

        let mut val = [0u8; Self::ENCODED_LEN];
        val[0] = per_tm_avail.encode();
        val[1..=2].copy_from_slice(&self.secs_in_period.to_be_bytes());
        val[3..=4].copy_from_slice(
            &self
                .timeout
                .as_ref()
                .map(TimeoutSnapshot::encode)
                .unwrap_or(Ok([0; 2]))?,
        );
        val[5] = self.scores.black;
        val[6] = self.scores.white;

        let encode_pen = |pen_opt: Option<&PenaltySnapshot>| -> Result<[u8; 2], EncodingError> {
            match pen_opt {
                Some(pen) => pen.encode(),
                None => Ok(PenaltySnapshot::encode_none()),
            }
        };

        let mut pen_iter = self.penalties.black.iter();

        val[7..=8].copy_from_slice(&encode_pen(pen_iter.next())?);
        val[9..=10].copy_from_slice(&encode_pen(pen_iter.next())?);
        val[11..=12].copy_from_slice(&encode_pen(pen_iter.next())?);

        let mut pen_iter = self.penalties.white.iter();

        val[13..=14].copy_from_slice(&encode_pen(pen_iter.next())?);
        val[15..=16].copy_from_slice(&encode_pen(pen_iter.next())?);
        val[17..=18].copy_from_slice(&encode_pen(pen_iter.next())?);

        Ok(val)
    }

    pub fn decode(bytes: &[u8; Self::ENCODED_LEN]) -> Result<Self, DecodingError> {
        let PeriodTimeoutAvailInfo {
            current_period,
            is_old_game,
            timeouts_available,
        } = PeriodTimeoutAvailInfo::decode(bytes[0])?;

        let mut penalties: BlackWhiteBundle<ArrayVec<PenaltySnapshot, PANEL_PENALTY_COUNT>> =
            Default::default();
        if let Some(pen) = PenaltySnapshot::decode(array_ref![bytes, 13, 2]) {
            penalties.white.push(pen);
        }
        if let Some(pen) = PenaltySnapshot::decode(array_ref![bytes, 15, 2]) {
            penalties.white.push(pen);
        }
        if let Some(pen) = PenaltySnapshot::decode(array_ref![bytes, 17, 2]) {
            penalties.white.push(pen);
        }
        if let Some(pen) = PenaltySnapshot::decode(array_ref![bytes, 7, 2]) {
            penalties.black.push(pen);
        }
        if let Some(pen) = PenaltySnapshot::decode(array_ref![bytes, 9, 2]) {
            penalties.black.push(pen);
        }
        if let Some(pen) = PenaltySnapshot::decode(array_ref![bytes, 11, 2]) {
            penalties.black.push(pen);
        }

        Ok(Self {
            current_period,
            secs_in_period: u16::from_be_bytes(*array_ref![bytes, 1, 2]),
            timeout: TimeoutSnapshot::decode(array_ref![bytes, 3, 2])?,
            scores: BlackWhiteBundle {
                black: bytes[5],
                white: bytes[6],
            },
            penalties,
            timeouts_available,
            is_old_game,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_penalty_time_ord() {
        assert!(PenaltyTime::Seconds(5) > PenaltyTime::Seconds(0));
        assert!(PenaltyTime::Seconds(5) < PenaltyTime::Seconds(9));
        assert!(PenaltyTime::TotalDismissal > PenaltyTime::Seconds(13));
        assert!(PenaltyTime::Seconds(10_000) < PenaltyTime::TotalDismissal);
        assert_eq!(PenaltyTime::Seconds(10), PenaltyTime::Seconds(10));
        assert_eq!(PenaltyTime::TotalDismissal, PenaltyTime::TotalDismissal);
    }

    #[test]
    fn test_period_penalties_run() {
        let all_periods_config = Game {
            overtime_allowed: true,
            sudden_death_allowed: true,
            ..Default::default()
        };
        let sd_only_config = Game {
            overtime_allowed: false,
            sudden_death_allowed: true,
            ..Default::default()
        };
        let no_sd_no_ot_config = Game {
            overtime_allowed: false,
            sudden_death_allowed: false,
            ..Default::default()
        };

        assert_eq!(
            GamePeriod::BetweenGames.penalties_run(&all_periods_config),
            false
        );
        assert_eq!(
            GamePeriod::FirstHalf.penalties_run(&all_periods_config),
            true
        );
        assert_eq!(
            GamePeriod::HalfTime.penalties_run(&all_periods_config),
            false
        );
        assert_eq!(
            GamePeriod::SecondHalf.penalties_run(&all_periods_config),
            true
        );
        assert_eq!(
            GamePeriod::PreOvertime.penalties_run(&all_periods_config),
            false
        );
        assert_eq!(
            GamePeriod::OvertimeFirstHalf.penalties_run(&all_periods_config),
            true
        );
        assert_eq!(
            GamePeriod::OvertimeHalfTime.penalties_run(&all_periods_config),
            false
        );
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.penalties_run(&all_periods_config),
            true
        );
        assert_eq!(
            GamePeriod::PreSuddenDeath.penalties_run(&all_periods_config),
            false
        );
        assert_eq!(
            GamePeriod::SuddenDeath.penalties_run(&all_periods_config),
            true
        );

        assert_eq!(
            GamePeriod::BetweenGames.penalties_run(&sd_only_config),
            false
        );
        assert_eq!(GamePeriod::FirstHalf.penalties_run(&sd_only_config), true);
        assert_eq!(GamePeriod::HalfTime.penalties_run(&sd_only_config), false);
        assert_eq!(GamePeriod::SecondHalf.penalties_run(&sd_only_config), true);
        assert_eq!(
            GamePeriod::PreOvertime.penalties_run(&sd_only_config),
            false
        );
        assert_eq!(
            GamePeriod::OvertimeFirstHalf.penalties_run(&sd_only_config),
            false
        );
        assert_eq!(
            GamePeriod::OvertimeHalfTime.penalties_run(&sd_only_config),
            false
        );
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.penalties_run(&sd_only_config),
            false
        );
        assert_eq!(
            GamePeriod::PreSuddenDeath.penalties_run(&sd_only_config),
            false
        );
        assert_eq!(GamePeriod::SuddenDeath.penalties_run(&sd_only_config), true);

        assert_eq!(
            GamePeriod::BetweenGames.penalties_run(&no_sd_no_ot_config),
            false
        );
        assert_eq!(
            GamePeriod::FirstHalf.penalties_run(&no_sd_no_ot_config),
            true
        );
        assert_eq!(
            GamePeriod::HalfTime.penalties_run(&no_sd_no_ot_config),
            false
        );
        assert_eq!(
            GamePeriod::SecondHalf.penalties_run(&no_sd_no_ot_config),
            true
        );
        assert_eq!(
            GamePeriod::PreOvertime.penalties_run(&no_sd_no_ot_config),
            false
        );
        assert_eq!(
            GamePeriod::OvertimeFirstHalf.penalties_run(&no_sd_no_ot_config),
            false
        );
        assert_eq!(
            GamePeriod::OvertimeHalfTime.penalties_run(&no_sd_no_ot_config),
            false
        );
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.penalties_run(&no_sd_no_ot_config),
            false
        );
        assert_eq!(
            GamePeriod::PreSuddenDeath.penalties_run(&no_sd_no_ot_config),
            false
        );
        assert_eq!(
            GamePeriod::SuddenDeath.penalties_run(&no_sd_no_ot_config),
            false
        );
    }

    #[test]
    fn test_period_duration() {
        let config = Game {
            half_play_duration: Duration::from_secs(5),
            half_time_duration: Duration::from_secs(7),
            pre_overtime_break: Duration::from_secs(9),
            ot_half_play_duration: Duration::from_secs(11),
            ot_half_time_duration: Duration::from_secs(13),
            pre_sudden_death_duration: Duration::from_secs(15),
            ..Default::default()
        };

        assert_eq!(GamePeriod::BetweenGames.duration(&config), None);
        assert_eq!(
            GamePeriod::FirstHalf.duration(&config),
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            GamePeriod::HalfTime.duration(&config),
            Some(Duration::from_secs(7))
        );
        assert_eq!(
            GamePeriod::SecondHalf.duration(&config),
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            GamePeriod::PreOvertime.duration(&config),
            Some(Duration::from_secs(9))
        );
        assert_eq!(
            GamePeriod::OvertimeFirstHalf.duration(&config),
            Some(Duration::from_secs(11))
        );
        assert_eq!(
            GamePeriod::OvertimeHalfTime.duration(&config),
            Some(Duration::from_secs(13))
        );
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.duration(&config),
            Some(Duration::from_secs(11))
        );
        assert_eq!(
            GamePeriod::PreSuddenDeath.duration(&config),
            Some(Duration::from_secs(15))
        );
        assert_eq!(GamePeriod::SuddenDeath.duration(&config), None);
    }

    #[test]
    fn test_next_period_duration() {
        let mut config = Game {
            half_play_duration: Duration::from_secs(5),
            half_time_duration: Duration::from_secs(7),
            pre_overtime_break: Duration::from_secs(9),
            ot_half_play_duration: Duration::from_secs(11),
            ot_half_time_duration: Duration::from_secs(13),
            pre_sudden_death_duration: Duration::from_secs(15),
            overtime_allowed: true,
            sudden_death_allowed: true,
            ..Default::default()
        };

        // Test once with all the game periods enabled
        assert_eq!(
            GamePeriod::BetweenGames.next_period_dur(&config),
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            GamePeriod::FirstHalf.next_period_dur(&config),
            Some(Duration::from_secs(7))
        );
        assert_eq!(
            GamePeriod::HalfTime.next_period_dur(&config),
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            GamePeriod::SecondHalf.next_period_dur(&config),
            Some(Duration::from_secs(9))
        );
        assert_eq!(
            GamePeriod::PreOvertime.next_period_dur(&config),
            Some(Duration::from_secs(11))
        );
        assert_eq!(
            GamePeriod::OvertimeFirstHalf.next_period_dur(&config),
            Some(Duration::from_secs(13))
        );
        assert_eq!(
            GamePeriod::OvertimeHalfTime.next_period_dur(&config),
            Some(Duration::from_secs(11))
        );
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.next_period_dur(&config),
            Some(Duration::from_secs(15))
        );
        assert_eq!(GamePeriod::PreSuddenDeath.next_period_dur(&config), None);
        assert_eq!(GamePeriod::SuddenDeath.next_period_dur(&config), None);

        config.overtime_allowed = false;

        // Test once with sudden death enabled and overtime disabled
        assert_eq!(
            GamePeriod::BetweenGames.next_period_dur(&config),
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            GamePeriod::FirstHalf.next_period_dur(&config),
            Some(Duration::from_secs(7))
        );
        assert_eq!(
            GamePeriod::HalfTime.next_period_dur(&config),
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            GamePeriod::SecondHalf.next_period_dur(&config),
            Some(Duration::from_secs(15))
        );
        assert_eq!(GamePeriod::PreOvertime.next_period_dur(&config), None);
        assert_eq!(GamePeriod::OvertimeFirstHalf.next_period_dur(&config), None);
        assert_eq!(GamePeriod::OvertimeHalfTime.next_period_dur(&config), None);
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.next_period_dur(&config),
            Some(Duration::from_secs(15))
        );
        assert_eq!(GamePeriod::PreSuddenDeath.next_period_dur(&config), None);
        assert_eq!(GamePeriod::SuddenDeath.next_period_dur(&config), None);

        config.sudden_death_allowed = false;

        // Test again with only the minimal periods enabled
        assert_eq!(
            GamePeriod::BetweenGames.next_period_dur(&config),
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            GamePeriod::FirstHalf.next_period_dur(&config),
            Some(Duration::from_secs(7))
        );
        assert_eq!(
            GamePeriod::HalfTime.next_period_dur(&config),
            Some(Duration::from_secs(5))
        );
        assert_eq!(GamePeriod::SecondHalf.next_period_dur(&config), None);
        assert_eq!(GamePeriod::PreOvertime.next_period_dur(&config), None);
        assert_eq!(GamePeriod::OvertimeFirstHalf.next_period_dur(&config), None);
        assert_eq!(GamePeriod::OvertimeHalfTime.next_period_dur(&config), None);
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.next_period_dur(&config),
            None
        );
        assert_eq!(GamePeriod::PreSuddenDeath.next_period_dur(&config), None);
        assert_eq!(GamePeriod::SuddenDeath.next_period_dur(&config), None);
    }

    #[test]
    fn test_period_time_elapsed_at() {
        let config = Game {
            half_play_duration: Duration::from_secs(5),
            half_time_duration: Duration::from_secs(7),
            pre_overtime_break: Duration::from_secs(9),
            ot_half_play_duration: Duration::from_secs(11),
            ot_half_time_duration: Duration::from_secs(13),
            pre_sudden_death_duration: Duration::from_secs(15),
            ..Default::default()
        };

        assert_eq!(
            GamePeriod::BetweenGames.time_elapsed_at(Duration::from_secs(5), &config),
            None
        );
        assert_eq!(
            GamePeriod::FirstHalf.time_elapsed_at(Duration::from_secs(3), &config),
            Some(SignedDuration::seconds(2))
        );
        assert_eq!(
            GamePeriod::HalfTime.time_elapsed_at(Duration::from_secs(4), &config),
            Some(SignedDuration::seconds(3))
        );
        assert_eq!(
            GamePeriod::SecondHalf.time_elapsed_at(Duration::from_secs(3), &config),
            Some(SignedDuration::seconds(2))
        );
        assert_eq!(
            GamePeriod::PreOvertime.time_elapsed_at(Duration::from_secs(4), &config),
            Some(SignedDuration::seconds(5))
        );
        assert_eq!(
            GamePeriod::OvertimeFirstHalf.time_elapsed_at(Duration::from_secs(7), &config),
            Some(SignedDuration::seconds(4))
        );
        assert_eq!(
            GamePeriod::OvertimeHalfTime.time_elapsed_at(Duration::from_secs(8), &config),
            Some(SignedDuration::seconds(5))
        );
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.time_elapsed_at(Duration::from_secs(7), &config),
            Some(SignedDuration::seconds(4))
        );
        assert_eq!(
            GamePeriod::PreSuddenDeath.time_elapsed_at(Duration::from_secs(9), &config),
            Some(SignedDuration::seconds(6))
        );
        assert_eq!(
            GamePeriod::SuddenDeath.time_elapsed_at(Duration::from_secs(3), &config),
            Some(SignedDuration::seconds(3))
        );

        assert_eq!(
            GamePeriod::FirstHalf.time_elapsed_at(Duration::from_secs(9), &config),
            Some(SignedDuration::seconds(-4))
        );
        assert_eq!(
            GamePeriod::HalfTime.time_elapsed_at(Duration::from_secs(9), &config),
            Some(SignedDuration::seconds(-2))
        );
        assert_eq!(
            GamePeriod::SecondHalf.time_elapsed_at(Duration::from_secs(9), &config),
            Some(SignedDuration::seconds(-4))
        );
        assert_eq!(
            GamePeriod::PreOvertime.time_elapsed_at(Duration::from_secs(25), &config),
            Some(SignedDuration::seconds(-16))
        );
        assert_eq!(
            GamePeriod::OvertimeFirstHalf.time_elapsed_at(Duration::from_secs(25), &config),
            Some(SignedDuration::seconds(-14))
        );
        assert_eq!(
            GamePeriod::OvertimeHalfTime.time_elapsed_at(Duration::from_secs(25), &config),
            Some(SignedDuration::seconds(-12))
        );
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.time_elapsed_at(Duration::from_secs(25), &config),
            Some(SignedDuration::seconds(-14))
        );
        assert_eq!(
            GamePeriod::PreSuddenDeath.time_elapsed_at(Duration::from_secs(25), &config),
            Some(SignedDuration::seconds(-10))
        );
    }

    #[test]
    fn test_period_time_between() {
        let mut period = GamePeriod::BetweenGames;
        while period != GamePeriod::SuddenDeath {
            assert_eq!(
                period.time_between(SignedDuration::seconds(6), SignedDuration::seconds(2)),
                SignedDuration::seconds(4)
            );
            assert_eq!(
                period.time_between(SignedDuration::seconds(6), SignedDuration::seconds(10)),
                SignedDuration::seconds(-4)
            );
            period = period.next_period().unwrap();
        }
        assert_eq!(
            GamePeriod::SuddenDeath
                .time_between(SignedDuration::seconds(6), SignedDuration::seconds(2)),
            SignedDuration::seconds(-4)
        );
        assert_eq!(
            GamePeriod::SuddenDeath
                .time_between(SignedDuration::seconds(6), SignedDuration::seconds(10)),
            SignedDuration::seconds(4)
        );
    }

    #[test]
    fn test_next_period() {
        assert_eq!(
            GamePeriod::BetweenGames.next_period(),
            Some(GamePeriod::FirstHalf)
        );
        assert_eq!(
            GamePeriod::FirstHalf.next_period(),
            Some(GamePeriod::HalfTime)
        );
        assert_eq!(
            GamePeriod::HalfTime.next_period(),
            Some(GamePeriod::SecondHalf)
        );
        assert_eq!(
            GamePeriod::SecondHalf.next_period(),
            Some(GamePeriod::PreOvertime)
        );
        assert_eq!(
            GamePeriod::PreOvertime.next_period(),
            Some(GamePeriod::OvertimeFirstHalf)
        );
        assert_eq!(
            GamePeriod::OvertimeFirstHalf.next_period(),
            Some(GamePeriod::OvertimeHalfTime)
        );
        assert_eq!(
            GamePeriod::OvertimeHalfTime.next_period(),
            Some(GamePeriod::OvertimeSecondHalf)
        );
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.next_period(),
            Some(GamePeriod::PreSuddenDeath)
        );
        assert_eq!(
            GamePeriod::PreSuddenDeath.next_period(),
            Some(GamePeriod::SuddenDeath)
        );
        assert_eq!(GamePeriod::SuddenDeath.next_period(), None);
    }

    #[test]
    fn test_custom_serialize_and_desereialize() -> Result<(), Box<dyn std::error::Error>> {
        let mut state = GameSnapshotNoHeap {
            current_period: GamePeriod::BetweenGames,
            secs_in_period: 0,
            timeout: None,
            scores: BlackWhiteBundle { black: 0, white: 0 },
            penalties: Default::default(),
            timeouts_available: BlackWhiteBundle {
                black: true,
                white: false,
            },
            is_old_game: false,
        };

        let test_state =
            |state: &mut GameSnapshotNoHeap| -> Result<(), Box<dyn std::error::Error>> {
                let serialization = state.encode()?;
                let mut recreated = GameSnapshotNoHeap::decode(array_ref![
                    serialization,
                    0,
                    GameSnapshotNoHeap::ENCODED_LEN
                ])?;
                assert_eq!(state, &mut recreated);
                Ok(())
            };

        test_state(&mut state)?;

        state.current_period = GamePeriod::FirstHalf;
        state.secs_in_period = 345;
        state.timeout = Some(TimeoutSnapshot::Black(16));
        state.timeouts_available.black = false;
        state.scores.black = 2;
        state.scores.white = 5;
        state.penalties.black.push(PenaltySnapshot {
            player_number: 1,
            time: PenaltyTime::Seconds(48),
            infraction: Infraction::Unknown,
        });
        state.penalties.white.push(PenaltySnapshot {
            player_number: 12,
            time: PenaltyTime::Seconds(96),
            infraction: Infraction::Unknown,
        });

        test_state(&mut state)?;

        state.current_period = GamePeriod::HalfTime;
        state.secs_in_period = 66;
        state.timeout = Some(TimeoutSnapshot::White(60));
        state.scores.black = 12;
        state.scores.white = 25;
        state.penalties.black.push(PenaltySnapshot {
            player_number: 4,
            time: PenaltyTime::Seconds(245),
            infraction: Infraction::Unknown,
        });
        state.penalties.white.push(PenaltySnapshot {
            player_number: 14,
            time: PenaltyTime::Seconds(300),
            infraction: Infraction::Unknown,
        });

        test_state(&mut state)?;

        state.current_period = GamePeriod::SecondHalf;
        state.secs_in_period = 900;
        state.timeout = Some(TimeoutSnapshot::Ref(432));
        state.scores.black = 99;
        state.scores.white = 99;
        state.penalties.black.push(PenaltySnapshot {
            player_number: 7,
            time: PenaltyTime::TotalDismissal,
            infraction: Infraction::Unknown,
        });
        state.penalties.white.push(PenaltySnapshot {
            player_number: 15,
            time: PenaltyTime::TotalDismissal,
            infraction: Infraction::Unknown,
        });

        test_state(&mut state)?;

        state.current_period = GamePeriod::PreOvertime;
        state.secs_in_period = 58;
        state.timeout = Some(TimeoutSnapshot::PenaltyShot(16));

        test_state(&mut state)?;

        state.current_period = GamePeriod::OvertimeFirstHalf;
        state.secs_in_period = 300;

        test_state(&mut state)?;

        state.current_period = GamePeriod::OvertimeHalfTime;
        state.secs_in_period = 53;

        test_state(&mut state)?;

        state.current_period = GamePeriod::OvertimeSecondHalf;

        test_state(&mut state)?;

        state.current_period = GamePeriod::PreSuddenDeath;

        test_state(&mut state)?;

        state.current_period = GamePeriod::SuddenDeath;

        test_state(&mut state)?;

        Ok(())
    }
}
