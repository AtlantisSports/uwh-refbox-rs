use matrix_drawing::transmitted_data::{Brightness, TransmittedData};
use uwh_common::game_snapshot::{GamePeriod, TimeoutSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Digit {
    pub a: bool,
    pub b: bool,
    pub c: bool,
    pub d: bool,
    pub e: bool,
    pub f: bool,
    pub g: bool,
}

impl Digit {
    const EMPTY: Self = Self {
        a: false,
        b: false,
        c: false,
        d: false,
        e: false,
        f: false,
        g: false,
    };

    const ERROR: Self = Self {
        a: false,
        b: false,
        c: false,
        d: false,
        e: false,
        f: false,
        g: true,
    };

    const ZERO: Self = Self {
        a: true,
        b: true,
        c: true,
        d: true,
        e: true,
        f: true,
        g: false,
    };

    const ONE: Self = Self {
        a: false,
        b: true,
        c: true,
        d: false,
        e: false,
        f: false,
        g: false,
    };

    const TWO: Self = Self {
        a: true,
        b: true,
        c: false,
        d: true,
        e: true,
        f: false,
        g: true,
    };

    const THREE: Self = Self {
        a: true,
        b: true,
        c: true,
        d: true,
        e: false,
        f: false,
        g: true,
    };

    const FOUR: Self = Self {
        a: false,
        b: true,
        c: true,
        d: false,
        e: false,
        f: true,
        g: true,
    };

    const FIVE: Self = Self {
        a: true,
        b: false,
        c: true,
        d: true,
        e: false,
        f: true,
        g: true,
    };

    const SIX: Self = Self {
        a: true,
        b: false,
        c: true,
        d: true,
        e: true,
        f: true,
        g: true,
    };

    const SEVEN: Self = Self {
        a: true,
        b: true,
        c: true,
        d: false,
        e: false,
        f: false,
        g: false,
    };

    const EIGHT: Self = Self {
        a: true,
        b: true,
        c: true,
        d: true,
        e: true,
        f: true,
        g: true,
    };

    const NINE: Self = Self {
        a: true,
        b: true,
        c: true,
        d: true,
        e: false,
        f: true,
        g: true,
    };

    pub const fn from_num(x: u8) -> Self {
        match x {
            0 => Self::ZERO,
            1 => Self::ONE,
            2 => Self::TWO,
            3 => Self::THREE,
            4 => Self::FOUR,
            5 => Self::FIVE,
            6 => Self::SIX,
            7 => Self::SEVEN,
            8 => Self::EIGHT,
            9 => Self::NINE,
            _ => Self::ERROR,
        }
    }

    /// Returns a pair of digits representing the tens and ones place of the given number.
    /// If the number is greater than 99, returns an error pair.
    /// If the number is less than 10, the tens place will be empty, unless `two_digits` is true.
    ///
    /// Return pattern: (tens, ones)
    pub const fn pair_from_num(x: u8, two_digits: bool) -> (Self, Self) {
        if x > 99 {
            return (Self::ERROR, Self::ERROR);
        }

        let tens = x / 10;
        let ones = x % 10;

        let tens = if !two_digits && tens == 0 {
            Self::EMPTY
        } else {
            Self::from_num(tens)
        };
        let ones = Self::from_num(ones);

        (tens, ones)
    }

    pub fn as_verilog(&self) -> String {
        format!(
            r#"'{{a: 1'b{}, b: 1'b{}, c: 1'b{}, d: 1'b{}, e: 1'b{}, f: 1'b{}, g: 1'b{}}}"#,
            self.a as u8,
            self.b as u8,
            self.c as u8,
            self.d as u8,
            self.e as u8,
            self.f as u8,
            self.g as u8
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayState {
    pub left_score_ones: Digit,
    pub left_score_tens: Digit,
    pub right_score_ones: Digit,
    pub right_score_tens: Digit,

    pub time_m_ones: Digit,
    pub time_m_tens: Digit,
    pub time_s_ones: Digit,
    pub time_s_tens: Digit,

    pub white_on_left: bool,
    pub white_on_right: bool,

    pub left_to_ind: bool,
    pub right_to_ind: bool,
    pub ref_to_ind: bool,

    pub one: bool,
    pub slash: bool,
    pub two: bool,
    pub overtime: bool,
    pub sdn_dth: bool,

    pub colon: bool,
}

impl DisplayState {
    const FLASH: Self = Self {
        left_score_ones: Digit::EIGHT,
        left_score_tens: Digit::EIGHT,
        right_score_ones: Digit::EIGHT,
        right_score_tens: Digit::EIGHT,
        time_m_ones: Digit::EIGHT,
        time_m_tens: Digit::EIGHT,
        time_s_ones: Digit::EIGHT,
        time_s_tens: Digit::EIGHT,
        white_on_left: true,
        white_on_right: true,
        left_to_ind: true,
        right_to_ind: true,
        ref_to_ind: true,
        one: true,
        slash: true,
        two: true,
        overtime: true,
        sdn_dth: true,
        colon: true,
    };

    pub const OFF: Self = Self {
        left_score_ones: Digit::EMPTY,
        left_score_tens: Digit::EMPTY,
        right_score_ones: Digit::EMPTY,
        right_score_tens: Digit::EMPTY,
        time_m_ones: Digit::EMPTY,
        time_m_tens: Digit::EMPTY,
        time_s_ones: Digit::EMPTY,
        time_s_tens: Digit::EMPTY,
        white_on_left: false,
        white_on_right: false,
        left_to_ind: false,
        right_to_ind: false,
        ref_to_ind: false,
        one: false,
        slash: false,
        two: false,
        overtime: false,
        sdn_dth: false,
        colon: false,
    };

    pub fn from_transmitted_data(data: &TransmittedData) -> (Self, Brightness) {
        if data.flash {
            return (Self::FLASH, data.brightness);
        }

        let white_on_left = !data.white_on_right;
        let white_on_right = data.white_on_right;

        let (left_score, right_score) = if white_on_right {
            (data.snapshot.scores.black, data.snapshot.scores.white)
        } else {
            (data.snapshot.scores.white, data.snapshot.scores.black)
        };

        let (left_score_tens, left_score_ones) = Digit::pair_from_num(left_score, false);
        let (right_score_tens, right_score_ones) = Digit::pair_from_num(right_score, false);

        let left_to_ind;
        let right_to_ind;
        if white_on_right {
            left_to_ind = matches!(data.snapshot.timeout, Some(TimeoutSnapshot::Black(_)));
            right_to_ind = matches!(data.snapshot.timeout, Some(TimeoutSnapshot::White(_)));
        } else {
            left_to_ind = matches!(data.snapshot.timeout, Some(TimeoutSnapshot::White(_)));
            right_to_ind = matches!(data.snapshot.timeout, Some(TimeoutSnapshot::Black(_)));
        }
        let ref_to_ind = matches!(
            data.snapshot.timeout,
            Some(TimeoutSnapshot::Ref(_)) | Some(TimeoutSnapshot::PenaltyShot(_))
        );

        let minutes = (data.snapshot.secs_in_period / 60)
            .try_into()
            .unwrap_or(u8::MAX);
        let seconds = if minutes > 99 {
            u8::MAX
        } else {
            (data.snapshot.secs_in_period % 60) as u8
        };

        // If we are not in a team timeout, show the game time. If we are in a team timeout, show the seconds
        // remaining in the timeout, maxing out at 99, and turning off the time digits
        let (time_m_tens, time_m_ones, time_s_tens, time_s_ones);
        match data.snapshot.timeout {
            Some(TimeoutSnapshot::Black(to_secs)) | Some(TimeoutSnapshot::White(to_secs)) => {
                time_m_tens = Digit::EMPTY;
                time_m_ones = Digit::EMPTY;
                (time_s_tens, time_s_ones) = Digit::pair_from_num(to_secs.min(99) as u8, true);
            }
            Some(TimeoutSnapshot::Ref(_)) | Some(TimeoutSnapshot::PenaltyShot(_)) | None => {
                (time_m_tens, time_m_ones) = Digit::pair_from_num(minutes, false);
                (time_s_tens, time_s_ones) = Digit::pair_from_num(seconds, true);
            }
        }

        let one = matches!(
            data.snapshot.current_period,
            GamePeriod::FirstHalf
                | GamePeriod::OvertimeFirstHalf
                | GamePeriod::HalfTime
                | GamePeriod::OvertimeHalfTime
        );
        let slash = matches!(
            data.snapshot.current_period,
            GamePeriod::HalfTime | GamePeriod::OvertimeHalfTime
        );
        let two = matches!(
            data.snapshot.current_period,
            GamePeriod::HalfTime
                | GamePeriod::OvertimeHalfTime
                | GamePeriod::SecondHalf
                | GamePeriod::OvertimeSecondHalf
        );
        let overtime = matches!(
            data.snapshot.current_period,
            GamePeriod::PreOvertime
                | GamePeriod::OvertimeFirstHalf
                | GamePeriod::OvertimeHalfTime
                | GamePeriod::OvertimeSecondHalf
                | GamePeriod::PreSuddenDeath
                | GamePeriod::SuddenDeath
        );
        let sdn_dth = matches!(
            data.snapshot.current_period,
            GamePeriod::PreSuddenDeath | GamePeriod::SuddenDeath
        );

        let colon = true;

        (
            Self {
                left_score_ones,
                left_score_tens,
                right_score_ones,
                right_score_tens,
                time_m_ones,
                time_m_tens,
                time_s_ones,
                time_s_tens,
                white_on_left,
                white_on_right,
                left_to_ind,
                right_to_ind,
                ref_to_ind,
                one,
                slash,
                two,
                overtime,
                sdn_dth,
                colon,
            },
            data.brightness,
        )
    }
}

#[cfg(test)]
mod test;
