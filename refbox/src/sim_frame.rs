use serde::{Deserialize, Serialize};

/// Which front-display layout the player-facing display window renders.
/// `Default` is the existing 256x64 LED-panel mirror; the others are
/// full-screen scoreboards usable only when no physical panel is connected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FrontDisplayLayout {
    #[default]
    Default,
    Classic,
    BigTime,
    Corners,
    ScoresOnly,
}

#[allow(dead_code)]
impl FrontDisplayLayout {
    /// Cycle order for the picker (wraps).
    pub const fn next(self) -> Self {
        match self {
            Self::Default => Self::Classic,
            Self::Classic => Self::BigTime,
            Self::BigTime => Self::Corners,
            Self::Corners => Self::ScoresOnly,
            Self::ScoresOnly => Self::Default,
        }
    }

    pub const fn to_u8(self) -> u8 {
        match self {
            Self::Default => 0,
            Self::Classic => 1,
            Self::BigTime => 2,
            Self::Corners => 3,
            Self::ScoresOnly => 4,
        }
    }

    pub const fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Classic,
            2 => Self::BigTime,
            3 => Self::Corners,
            4 => Self::ScoresOnly,
            _ => Self::Default,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_order_wraps() {
        let mut l = FrontDisplayLayout::Default;
        let mut seen = vec![l];
        for _ in 0..5 {
            l = l.next();
            seen.push(l);
        }
        assert_eq!(
            seen,
            vec![
                FrontDisplayLayout::Default,
                FrontDisplayLayout::Classic,
                FrontDisplayLayout::BigTime,
                FrontDisplayLayout::Corners,
                FrontDisplayLayout::ScoresOnly,
                FrontDisplayLayout::Default,
            ]
        );
    }

    #[test]
    fn u8_round_trips() {
        for l in [
            FrontDisplayLayout::Default,
            FrontDisplayLayout::Classic,
            FrontDisplayLayout::BigTime,
            FrontDisplayLayout::Corners,
            FrontDisplayLayout::ScoresOnly,
        ] {
            assert_eq!(FrontDisplayLayout::from_u8(l.to_u8()), l);
        }
    }

    #[test]
    fn unknown_u8_falls_back_to_default() {
        assert_eq!(FrontDisplayLayout::from_u8(99), FrontDisplayLayout::Default);
    }
}
