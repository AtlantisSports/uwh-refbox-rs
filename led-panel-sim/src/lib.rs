use matrix_drawing::transmitted_data::TransmittedData;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutTime {
    pub fifteen: bool,
    pub thirty: bool,
    pub forty_five: bool,
    pub sixty: bool,
    pub int: bool,
}

impl TimeoutTime {
    const OFF: Self = Self {
        fifteen: false,
        thirty: false,
        forty_five: false,
        sixty: false,
        int: false,
    };

    const ON: Self = Self {
        fifteen: true,
        thirty: true,
        forty_five: true,
        sixty: true,
        int: true,
    };

    pub fn as_verilog(&self) -> String {
        format!(
            r#"'{{fifteen: 1'b{}, thirty: 1'b{}, forty_five: 1'b{}, sixty: 1'b{}, interstice: 1'b{}}}"#,
            self.fifteen as u8,
            self.thirty as u8,
            self.forty_five as u8,
            self.sixty as u8,
            self.int as u8
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayState {
    pub b_score_ones: Digit,
    pub b_score_tens: Digit,
    pub w_score_ones: Digit,
    pub w_score_tens: Digit,

    pub time_m_ones: Digit,
    pub time_m_tens: Digit,
    pub time_s_ones: Digit,
    pub time_s_tens: Digit,

    pub b_timeout_time: TimeoutTime,
    pub w_timeout_time: TimeoutTime,

    pub bto_ind: bool,
    pub wto_ind: bool,
    pub rto_ind: bool,

    pub fst_hlf: bool,
    pub hlf_tm: bool,
    pub snd_hlf: bool,
    pub overtime: bool,
    pub sdn_dth: bool,

    pub colon: bool,
}

impl DisplayState {
    const FLASH: Self = Self {
        b_score_ones: Digit::EIGHT,
        b_score_tens: Digit::EIGHT,
        w_score_ones: Digit::EIGHT,
        w_score_tens: Digit::EIGHT,
        time_m_ones: Digit::EIGHT,
        time_m_tens: Digit::EIGHT,
        time_s_ones: Digit::EIGHT,
        time_s_tens: Digit::EIGHT,
        b_timeout_time: TimeoutTime {
            fifteen: true,
            thirty: true,
            forty_five: true,
            sixty: true,
            int: true,
        },
        w_timeout_time: TimeoutTime {
            fifteen: true,
            thirty: true,
            forty_five: true,
            sixty: true,
            int: true,
        },
        bto_ind: true,
        wto_ind: true,
        rto_ind: true,
        fst_hlf: true,
        hlf_tm: true,
        snd_hlf: true,
        overtime: true,
        sdn_dth: true,
        colon: true,
    };

    pub const OFF: Self = Self {
        b_score_ones: Digit::EMPTY,
        b_score_tens: Digit::EMPTY,
        w_score_ones: Digit::EMPTY,
        w_score_tens: Digit::EMPTY,
        time_m_ones: Digit::EMPTY,
        time_m_tens: Digit::EMPTY,
        time_s_ones: Digit::EMPTY,
        time_s_tens: Digit::EMPTY,
        b_timeout_time: TimeoutTime {
            fifteen: false,
            thirty: false,
            forty_five: false,
            sixty: false,
            int: false,
        },
        w_timeout_time: TimeoutTime {
            fifteen: false,
            thirty: false,
            forty_five: false,
            sixty: false,
            int: false,
        },
        bto_ind: false,
        wto_ind: false,
        rto_ind: false,
        fst_hlf: false,
        hlf_tm: false,
        snd_hlf: false,
        overtime: false,
        sdn_dth: false,
        colon: false,
    };

    pub fn from_transmitted_data(data: &TransmittedData) -> Self {
        if data.flash {
            return Self::FLASH;
        }

        let (b_score_tens, b_score_ones) = Digit::pair_from_num(data.snapshot.scores.black, false);
        let (w_score_tens, w_score_ones) = Digit::pair_from_num(data.snapshot.scores.white, false);

        let minutes = (data.snapshot.secs_in_period / 60)
            .try_into()
            .unwrap_or(u8::MAX);
        let seconds = if minutes > 99 {
            u8::MAX
        } else {
            (data.snapshot.secs_in_period % 60) as u8
        };

        let (time_m_tens, time_m_ones) = Digit::pair_from_num(minutes, false);
        let (time_s_tens, time_s_ones) = Digit::pair_from_num(seconds, true);

        let b_timeout_time = if let Some(TimeoutSnapshot::Black(t)) = data.snapshot.timeout {
            TimeoutTime {
                fifteen: t > 0,
                thirty: t > 15,
                forty_five: t > 30,
                sixty: t > 45,
                int: false,
            }
        } else if data.snapshot.timeouts_available.black {
            TimeoutTime::ON
        } else {
            TimeoutTime::OFF
        };

        let w_timeout_time = if let Some(TimeoutSnapshot::White(t)) = data.snapshot.timeout {
            TimeoutTime {
                fifteen: t > 0,
                thirty: t > 15,
                forty_five: t > 30,
                sixty: t > 45,
                int: false,
            }
        } else if data.snapshot.timeouts_available.white {
            TimeoutTime::ON
        } else {
            TimeoutTime::OFF
        };

        let bto_ind = matches!(data.snapshot.timeout, Some(TimeoutSnapshot::Black(_)));
        let wto_ind = matches!(data.snapshot.timeout, Some(TimeoutSnapshot::White(_)));
        let rto_ind = matches!(
            data.snapshot.timeout,
            Some(TimeoutSnapshot::Ref(_)) | Some(TimeoutSnapshot::PenaltyShot(_))
        );

        let fst_hlf = matches!(
            data.snapshot.current_period,
            GamePeriod::FirstHalf
                | GamePeriod::OvertimeFirstHalf
                | GamePeriod::HalfTime
                | GamePeriod::OvertimeHalfTime
                | GamePeriod::SecondHalf
                | GamePeriod::OvertimeSecondHalf
        );
        let hlf_tm = matches!(
            data.snapshot.current_period,
            GamePeriod::HalfTime
                | GamePeriod::OvertimeHalfTime
                | GamePeriod::SecondHalf
                | GamePeriod::OvertimeSecondHalf
        );
        let snd_hlf = matches!(
            data.snapshot.current_period,
            GamePeriod::SecondHalf | GamePeriod::OvertimeSecondHalf
        );
        let overtime = matches!(
            data.snapshot.current_period,
            GamePeriod::PreOvertime
                | GamePeriod::OvertimeFirstHalf
                | GamePeriod::OvertimeHalfTime
                | GamePeriod::OvertimeSecondHalf
        );
        let sdn_dth = matches!(
            data.snapshot.current_period,
            GamePeriod::PreSuddenDeath | GamePeriod::SuddenDeath
        );

        let colon = true;

        Self {
            b_score_ones,
            b_score_tens,
            w_score_ones,
            w_score_tens,
            time_m_ones,
            time_m_tens,
            time_s_ones,
            time_s_tens,
            b_timeout_time,
            w_timeout_time,
            bto_ind,
            wto_ind,
            rto_ind,
            fst_hlf,
            hlf_tm,
            snd_hlf,
            overtime,
            sdn_dth,
            colon,
        }
    }
}

#[cfg(test)]
mod test;
