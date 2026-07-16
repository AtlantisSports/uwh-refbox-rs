//! Raspberry Pi detection and (Task 2) power commands for the operator power page.

/// True when the device-tree model string identifies a Raspberry Pi. Factored
/// out from `detect_raspberry_pi` so it is unit-testable without touching `/proc`.
pub fn model_is_pi(model: &str) -> bool {
    model.contains("Raspberry Pi")
}

/// True when running on a Raspberry Pi. Reads the device-tree model file, which
/// exists and names "Raspberry Pi" only on real Pi hardware (absent on WSL /
/// Windows / macOS, which therefore return false).
pub fn detect_raspberry_pi() -> bool {
    std::fs::read_to_string("/proc/device-tree/model")
        .map(|m| model_is_pi(&m))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_is_pi_matches_real_pi_strings() {
        assert!(model_is_pi("Raspberry Pi 4 Model B Rev 1.4"));
        // Device-tree strings are often NUL-terminated; must still match.
        assert!(model_is_pi("Raspberry Pi 5 Model B Rev 1.0\u{0}"));
    }

    #[test]
    fn model_is_pi_rejects_non_pi() {
        assert!(!model_is_pi(""));
        assert!(!model_is_pi("Some Generic x86 Board"));
        assert!(!model_is_pi("Microsoft WSL2"));
    }
}
