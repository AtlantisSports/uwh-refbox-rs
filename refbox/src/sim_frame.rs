use matrix_drawing::transmitted_data::TransmittedData;
use serde::{Deserialize, Serialize};
use uwh_common::game_snapshot::{DecodingError, EncodingError};

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

/// The layout actually shown during beep test: forced to `Default` whenever a
/// real LED panel is connected (the panel only renders Default), else the
/// operator's in-memory session choice. Mirrors the game-mode Display page's
/// `effective_layout` rule so the picker label, the preview, and the layout
/// pushed to the display always agree.
pub fn effective_beep_layout(
    has_led_panel: bool,
    session_layout: FrontDisplayLayout,
) -> FrontDisplayLayout {
    if has_led_panel {
        FrontDisplayLayout::Default
    } else {
        session_layout
    }
}

/// A display-feed frame: the existing panel payload plus a one-byte layout
/// selector. Used ONLY on the binary/TCP path to the display window. The
/// serial/hardware path keeps sending bare `TransmittedData`, so the panel
/// wire format is unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimFrame {
    pub layout: FrontDisplayLayout,
    pub data: TransmittedData,
}

impl SimFrame {
    pub const ENCODED_LEN: usize = TransmittedData::ENCODED_LEN + 1;

    pub fn encode(&self) -> Result<[u8; Self::ENCODED_LEN], EncodingError> {
        let mut out = [0u8; Self::ENCODED_LEN];
        out[0] = self.layout.to_u8();
        out[1..].copy_from_slice(&self.data.encode()?);
        Ok(out)
    }

    pub fn decode(bytes: &[u8; Self::ENCODED_LEN]) -> Result<Self, DecodingError> {
        let mut buf = [0u8; TransmittedData::ENCODED_LEN];
        buf.copy_from_slice(&bytes[1..]);
        Ok(Self {
            layout: FrontDisplayLayout::from_u8(bytes[0]),
            data: TransmittedData::decode(&buf)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_drawing::transmitted_data::Brightness;
    use uwh_common::game_snapshot::GameSnapshotNoHeap;

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

    #[test]
    fn effective_beep_layout_forces_default_with_panel() {
        // With a panel, any session choice collapses to Default.
        assert_eq!(
            effective_beep_layout(true, FrontDisplayLayout::Corners),
            FrontDisplayLayout::Default,
        );
        // Without a panel, the session choice is respected.
        assert_eq!(
            effective_beep_layout(false, FrontDisplayLayout::Corners),
            FrontDisplayLayout::Corners,
        );
        assert_eq!(
            effective_beep_layout(false, FrontDisplayLayout::Default),
            FrontDisplayLayout::Default,
        );
    }

    #[test]
    fn sim_frame_round_trips_and_is_one_byte_longer() {
        // Proves the serial/hardware format is untouched: SimFrame is exactly
        // one byte longer than TransmittedData, and that extra byte is layout.
        assert_eq!(SimFrame::ENCODED_LEN, TransmittedData::ENCODED_LEN + 1);

        let frame = SimFrame {
            layout: FrontDisplayLayout::Corners,
            data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap::default(),
            },
        };
        let bytes = frame.encode().unwrap();
        assert_eq!(bytes[0], FrontDisplayLayout::Corners.to_u8());
        assert_eq!(SimFrame::decode(&bytes).unwrap(), frame);
    }
}
