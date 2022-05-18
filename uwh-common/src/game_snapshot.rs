#[cfg(feature = "std")]
use crate::config::Game;
use arrayref::array_ref;
use arrayvec::ArrayVec;
use core::{
    cmp::{Ordering, PartialOrd},
    time::Duration,
};
#[cfg(not(target_os = "windows"))]
use defmt::Format;
use derivative::Derivative;
use displaydoc::Display;
use serde_derive::{Deserialize, Serialize};

const PANEL_PENALTY_COUNT: usize = 3;

/// Game snapshot information that the LED matrices need. Excludes some fields, limits to three
/// penalties (the three with the lowest remaining time), and places the penalties on a stack based
/// `ArrayVec`, instead of the heap based `Vec`
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct GameSnapshotNoHeap {
    pub current_period: GamePeriod,
    pub secs_in_period: u16,
    pub timeout: TimeoutSnapshot,
    pub b_score: u8,
    pub w_score: u8,
    pub b_penalties: ArrayVec<PenaltySnapshot, PANEL_PENALTY_COUNT>,
    pub w_penalties: ArrayVec<PenaltySnapshot, PANEL_PENALTY_COUNT>,
}

/// All the information needed by a UI to draw the current state of the game. Requires the `std`
/// feature.
#[cfg(feature = "std")]
#[derive(Debug, PartialEq, Eq, Default, Clone, Serialize, Deserialize)]
pub struct GameSnapshot {
    pub current_period: GamePeriod,
    pub secs_in_period: u16,
    pub timeout: TimeoutSnapshot,
    pub b_score: u8,
    pub w_score: u8,
    pub b_penalties: Vec<PenaltySnapshot>,
    pub w_penalties: Vec<PenaltySnapshot>,
    pub game_number: u16,
    pub next_game_number: u16,
    pub tournament_id: u64,
}

#[cfg(feature = "std")]
impl From<GameSnapshot> for GameSnapshotNoHeap {
    fn from(snapshot: GameSnapshot) -> Self {
        let process_penalties = |mut orig: Vec<PenaltySnapshot>| {
            orig.retain(|pen| {
                if let PenaltyTime::Seconds(secs) = pen.time {
                    secs != 0
                } else {
                    true
                }
            });
            orig.sort_by(|a, b| a.time.cmp(&b.time));
            orig.into_iter().take(3).collect()
        };

        Self {
            current_period: snapshot.current_period,
            secs_in_period: snapshot.secs_in_period,
            timeout: snapshot.timeout,
            b_score: snapshot.b_score,
            w_score: snapshot.w_score,
            b_penalties: process_penalties(snapshot.b_penalties),
            w_penalties: process_penalties(snapshot.w_penalties),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct PenaltySnapshot {
    pub player_number: u8,
    pub time: PenaltyTime,
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
            Self::OvertimeFirstHalf | Self::OvertimeSecondHalf => config.has_overtime,
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
    pub fn time_elapsed_at(self, time: Duration, config: &Game) -> Option<Duration> {
        match self {
            p @ Self::BetweenGames
            | p @ Self::FirstHalf
            | p @ Self::HalfTime
            | p @ Self::SecondHalf
            | p @ Self::PreOvertime
            | p @ Self::OvertimeFirstHalf
            | p @ Self::OvertimeHalfTime
            | p @ Self::OvertimeSecondHalf
            | p @ Self::PreSuddenDeath => p.duration(config).and_then(|d| d.checked_sub(time)),
            Self::SuddenDeath => Some(time),
        }
    }

    pub fn time_between(self, start: Duration, end: Duration) -> Option<Duration> {
        match self {
            Self::BetweenGames
            | Self::FirstHalf
            | Self::HalfTime
            | Self::SecondHalf
            | Self::PreOvertime
            | Self::OvertimeFirstHalf
            | Self::OvertimeHalfTime
            | Self::OvertimeSecondHalf
            | Self::PreSuddenDeath => start.checked_sub(end),
            Self::SuddenDeath => end.checked_sub(start),
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
    None,
    White(u16),
    Black(u16),
    Ref(u16),
    PenaltyShot(u16),
}

impl core::fmt::Display for TimeoutSnapshot {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match *self {
            TimeoutSnapshot::None => write!(f, "No Timeout"),
            TimeoutSnapshot::Black(_) => write!(f, "Black Timeout"),
            TimeoutSnapshot::White(_) => write!(f, "White Timeout"),
            TimeoutSnapshot::Ref(_) => write!(f, "Ref Timeout"),
            TimeoutSnapshot::PenaltyShot(_) => write!(f, "PenaltyShot"),
        }
    }
}

#[derive(Derivative, Serialize, Deserialize)]
#[derivative(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub enum Color {
    #[derivative(Default)]
    Black,
    White,
}

impl core::fmt::Display for Color {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match *self {
            Self::Black => write!(f, "Black"),
            Self::White => write!(f, "White"),
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
        })
    }
}

impl TimeoutSnapshot {
    pub fn encode(&self) -> Result<[u8; 2], EncodingError> {
        match self {
            Self::None => Ok([0x00, 0x00]),
            Self::Black(time) | Self::White(time) | Self::Ref(time) | Self::PenaltyShot(time) => {
                if *time > 5999 {
                    Err(EncodingError::TimeoutTimeTooLarge(*time))
                } else {
                    let variant = match self {
                        Self::None => panic!("Impossible"),
                        Self::Black(_) => 0x20,
                        Self::White(_) => 0x40,
                        Self::Ref(_) => 0x60,
                        Self::PenaltyShot(_) => 0x80,
                    };
                    let mut arr = time.to_be_bytes();
                    arr[0] |= variant;
                    Ok(arr)
                }
            }
        }
    }

    pub fn decode(bytes: &[u8; 2]) -> Result<Self, DecodingError> {
        let val = u16::from_be_bytes(*bytes);
        let time = val & 0x1fff;
        match (val & 0xe000) >> 13 {
            0x0 => Ok(Self::None),
            0x1 => Ok(Self::Black(time)),
            0x2 => Ok(Self::White(time)),
            0x3 => Ok(Self::Ref(time)),
            0x4 => Ok(Self::PenaltyShot(time)),
            other => Err(DecodingError::InvalidTimeoutType(other)),
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
        let mut val = [0u8; Self::ENCODED_LEN];
        val[0] = self.current_period.encode();
        val[1..=2].copy_from_slice(&self.secs_in_period.to_be_bytes());
        val[3..=4].copy_from_slice(&self.timeout.encode()?);
        val[5] = self.b_score;
        val[6] = self.w_score;

        let encode_pen = |pen_opt: Option<&PenaltySnapshot>| -> Result<[u8; 2], EncodingError> {
            match pen_opt {
                Some(pen) => pen.encode(),
                None => Ok(PenaltySnapshot::encode_none()),
            }
        };

        let mut pen_iter = self.b_penalties.iter();

        val[7..=8].copy_from_slice(&encode_pen(pen_iter.next())?);
        val[9..=10].copy_from_slice(&encode_pen(pen_iter.next())?);
        val[11..=12].copy_from_slice(&encode_pen(pen_iter.next())?);

        let mut pen_iter = self.w_penalties.iter();

        val[13..=14].copy_from_slice(&encode_pen(pen_iter.next())?);
        val[15..=16].copy_from_slice(&encode_pen(pen_iter.next())?);
        val[17..=18].copy_from_slice(&encode_pen(pen_iter.next())?);

        Ok(val)
    }

    pub fn decode(bytes: &[u8; Self::ENCODED_LEN]) -> Result<Self, DecodingError> {
        let mut b_penalties = ArrayVec::new();
        let mut w_penalties = ArrayVec::new();
        if let Some(pen) = PenaltySnapshot::decode(array_ref![bytes, 13, 2]) {
            w_penalties.push(pen);
        }
        if let Some(pen) = PenaltySnapshot::decode(array_ref![bytes, 15, 2]) {
            w_penalties.push(pen);
        }
        if let Some(pen) = PenaltySnapshot::decode(array_ref![bytes, 17, 2]) {
            w_penalties.push(pen);
        }
        if let Some(pen) = PenaltySnapshot::decode(array_ref![bytes, 7, 2]) {
            b_penalties.push(pen);
        }
        if let Some(pen) = PenaltySnapshot::decode(array_ref![bytes, 9, 2]) {
            b_penalties.push(pen);
        }
        if let Some(pen) = PenaltySnapshot::decode(array_ref![bytes, 11, 2]) {
            b_penalties.push(pen);
        }

        Ok(Self {
            current_period: GamePeriod::decode(bytes[0])?,
            secs_in_period: u16::from_be_bytes(*array_ref![bytes, 1, 2]),
            timeout: TimeoutSnapshot::decode(array_ref![bytes, 3, 2])?,
            b_score: bytes[5],
            w_score: bytes[6],
            b_penalties,
            w_penalties,
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
            has_overtime: true,
            sudden_death_allowed: true,
            ..Default::default()
        };
        let sd_only_config = Game {
            has_overtime: false,
            sudden_death_allowed: true,
            ..Default::default()
        };
        let no_sd_no_ot_config = Game {
            has_overtime: false,
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
            Some(Duration::from_secs(2))
        );
        assert_eq!(
            GamePeriod::HalfTime.time_elapsed_at(Duration::from_secs(4), &config),
            Some(Duration::from_secs(3))
        );
        assert_eq!(
            GamePeriod::SecondHalf.time_elapsed_at(Duration::from_secs(3), &config),
            Some(Duration::from_secs(2))
        );
        assert_eq!(
            GamePeriod::PreOvertime.time_elapsed_at(Duration::from_secs(4), &config),
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            GamePeriod::OvertimeFirstHalf.time_elapsed_at(Duration::from_secs(7), &config),
            Some(Duration::from_secs(4))
        );
        assert_eq!(
            GamePeriod::OvertimeHalfTime.time_elapsed_at(Duration::from_secs(8), &config),
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.time_elapsed_at(Duration::from_secs(7), &config),
            Some(Duration::from_secs(4))
        );
        assert_eq!(
            GamePeriod::PreSuddenDeath.time_elapsed_at(Duration::from_secs(9), &config),
            Some(Duration::from_secs(6))
        );
        assert_eq!(
            GamePeriod::SuddenDeath.time_elapsed_at(Duration::from_secs(3), &config),
            Some(Duration::from_secs(3))
        );

        assert_eq!(
            GamePeriod::FirstHalf.time_elapsed_at(Duration::from_secs(9), &config),
            None
        );
        assert_eq!(
            GamePeriod::HalfTime.time_elapsed_at(Duration::from_secs(9), &config),
            None
        );
        assert_eq!(
            GamePeriod::SecondHalf.time_elapsed_at(Duration::from_secs(9), &config),
            None
        );
        assert_eq!(
            GamePeriod::PreOvertime.time_elapsed_at(Duration::from_secs(25), &config),
            None
        );
        assert_eq!(
            GamePeriod::OvertimeFirstHalf.time_elapsed_at(Duration::from_secs(25), &config),
            None
        );
        assert_eq!(
            GamePeriod::OvertimeHalfTime.time_elapsed_at(Duration::from_secs(25), &config),
            None
        );
        assert_eq!(
            GamePeriod::OvertimeSecondHalf.time_elapsed_at(Duration::from_secs(25), &config),
            None
        );
        assert_eq!(
            GamePeriod::PreSuddenDeath.time_elapsed_at(Duration::from_secs(25), &config),
            None
        );
    }

    #[test]
    fn test_period_time_between() {
        let mut period = GamePeriod::BetweenGames;
        while period != GamePeriod::SuddenDeath {
            assert_eq!(
                period.time_between(Duration::from_secs(6), Duration::from_secs(2)),
                Some(Duration::from_secs(4))
            );
            assert_eq!(
                period.time_between(Duration::from_secs(6), Duration::from_secs(10)),
                None
            );
            period = period.next_period().unwrap();
        }
        assert_eq!(
            GamePeriod::SuddenDeath.time_between(Duration::from_secs(6), Duration::from_secs(2)),
            None
        );
        assert_eq!(
            GamePeriod::SuddenDeath.time_between(Duration::from_secs(6), Duration::from_secs(10)),
            Some(Duration::from_secs(4))
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
            timeout: TimeoutSnapshot::None,
            b_score: 0,
            w_score: 0,
            b_penalties: ArrayVec::new(),
            w_penalties: ArrayVec::new(),
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
        state.timeout = TimeoutSnapshot::Black(16);
        state.b_score = 2;
        state.w_score = 5;
        state.b_penalties.push(PenaltySnapshot {
            player_number: 1,
            time: PenaltyTime::Seconds(48),
        });
        state.w_penalties.push(PenaltySnapshot {
            player_number: 12,
            time: PenaltyTime::Seconds(96),
        });

        test_state(&mut state)?;

        state.current_period = GamePeriod::HalfTime;
        state.secs_in_period = 66;
        state.timeout = TimeoutSnapshot::White(60);
        state.b_score = 12;
        state.w_score = 25;
        state.b_penalties.push(PenaltySnapshot {
            player_number: 4,
            time: PenaltyTime::Seconds(245),
        });
        state.w_penalties.push(PenaltySnapshot {
            player_number: 14,
            time: PenaltyTime::Seconds(300),
        });

        test_state(&mut state)?;

        state.current_period = GamePeriod::SecondHalf;
        state.secs_in_period = 900;
        state.timeout = TimeoutSnapshot::Ref(432);
        state.b_score = 99;
        state.w_score = 99;
        state.b_penalties.push(PenaltySnapshot {
            player_number: 7,
            time: PenaltyTime::TotalDismissal,
        });
        state.w_penalties.push(PenaltySnapshot {
            player_number: 15,
            time: PenaltyTime::TotalDismissal,
        });

        test_state(&mut state)?;

        state.current_period = GamePeriod::PreOvertime;
        state.secs_in_period = 58;
        state.timeout = TimeoutSnapshot::PenaltyShot(16);

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
