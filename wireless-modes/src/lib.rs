#![no_std]

use lora_modulation::{Bandwidth, CodingRate, SpreadingFactor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WirelessMode {
    UnitedStates,
    Europe,
    Australia,
}

impl WirelessMode {
    /// Gray code values should be false for open pin on the rotary switch
    /// and true for closed pin
    pub fn from_gray_code(one: bool, two: bool, four: bool, eight: bool) -> Option<Self> {
        // | Rotary Switch | Pin 1 (SW1) | Pin 2 (SW2) | Pin 4 (SW3) | Pin 8 (SW4) |
        // |:-------------:|:-----------:|:-----------:|:-----------:|:-----------:|
        // |       0       |    false    |    false    |    false    |    false    |
        // |       1       |     true    |    false    |    false    |    false    |
        // |       2       |     true    |     true    |    false    |    false    |
        // |       3       |    false    |     true    |    false    |    false    |
        // |       4       |    false    |     true    |     true    |    false    |
        // |       5       |     true    |     true    |     true    |    false    |
        // |       6       |     true    |    false    |     true    |    false    |
        // |       7       |    false    |    false    |     true    |    false    |
        // |       8       |    false    |    false    |     true    |     true    |
        // |       9       |     true    |    false    |     true    |     true    |
        // |       A       |     true    |     true    |     true    |     true    |
        // |       B       |    false    |     true    |     true    |     true    |
        // |       C       |    false    |     true    |    false    |     true    |
        // |       D       |     true    |     true    |    false    |     true    |
        // |       E       |     true    |    false    |    false    |     true    |
        // |       F       |    false    |    false    |    false    |     true    |
        match (one, two, four, eight) {
            (false, false, false, false) => Some(WirelessMode::UnitedStates), // 0
            (true, false, false, false) => Some(WirelessMode::Europe),        // 1
            (true, true, false, false) => Some(WirelessMode::Australia),      // 2
            _ => None,
        }
    }

    /// Returns the frequency in Hz
    pub fn frequency(&self) -> u32 {
        match self {
            WirelessMode::UnitedStates => 914_900_000,
            WirelessMode::Europe => 868_300_000,
            WirelessMode::Australia => 918_000_000,
        }
    }

    pub fn tx_power(&self) -> i32 {
        // Unit is dBm. The radio max is 20dBm
        match self {
            WirelessMode::UnitedStates => 20,
            WirelessMode::Europe => 14,
            WirelessMode::Australia => 20,
        }
    }

    pub fn bandwidth(&self) -> Bandwidth {
        Bandwidth::_250KHz
    }

    pub fn spreading_factor(&self) -> SpreadingFactor {
        SpreadingFactor::_8
    }

    pub fn coding_rate(&self) -> CodingRate {
        CodingRate::_4_8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_gray_code() {
        assert_eq!(
            WirelessMode::from_gray_code(false, false, false, false),
            Some(WirelessMode::UnitedStates)
        );
        assert_eq!(
            WirelessMode::from_gray_code(true, false, false, false),
            Some(WirelessMode::Europe)
        );
        assert_eq!(
            WirelessMode::from_gray_code(true, true, false, false),
            Some(WirelessMode::Australia)
        );
        assert_eq!(WirelessMode::from_gray_code(false, true, true, true), None);
    }
}
