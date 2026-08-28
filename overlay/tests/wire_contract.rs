//! The wire contract between the refbox and this overlay, asserted on DECODED VALUES.
//!
//! The overlay builds in the refbox's own workspace against `uwh-common` by path, so a
//! renamed REQUIRED field is already a loud failure. What compilation cannot catch is a
//! change that keeps building while the MEANING of the data moves -- serde ignores unknown
//! fields, so both sides compile happily through exactly that, and non-Rust consumers such
//! as the vMix bridge get no compile-time protection at all. So: assert the numbers, and
//! assert the wire KEY STRINGS, because renaming a key is what silently breaks a consumer
//! -- the type is unchanged, everything compiles, and the field simply arrives as a
//! default forever.
//!
//! KNOWN GAP: this fixture was captured before `portal_base_url` was added to
//! `GameSnapshot`, so that key is NOT pinned below even though it is an `Option`
//! field whose rename would be silent -- and it is the field the vMix bridge
//! depends on entirely. Closing it needs a FRESH CAPTURE from a refbox carrying
//! the field; do not hand-add the key to these messages, they are real captures
//! and their value is that nobody edited them.

use uwh_common::game_snapshot::{GamePeriod, GameSnapshot};

const CAPTURE: &str = include_str!("fixtures/feed-capture.jsonl");

fn snapshots() -> Vec<GameSnapshot> {
    CAPTURE
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("captured refbox line must still decode"))
        .collect()
}

#[test]
fn every_captured_line_decodes() {
    assert_eq!(snapshots().len(), 11, "fixture corpus changed size");
}

#[test]
fn decoded_values_match_the_capture() {
    // Index 5, NOT index 0. Index 0 is a between-games snapshot whose every interesting
    // value is zero or empty -- and a decoder that silently yielded GameSnapshot::default()
    // would PASS an all-zeros assertion. A contract test must assert against values that
    // are not the type's defaults, or it certifies nothing.
    let s = &snapshots()[5];
    assert_eq!(s.current_period, GamePeriod::FirstHalf);
    assert_eq!(s.secs_in_period, 23);
    assert_eq!(s.scores.black, 1);
    assert_eq!(s.scores.white, 0);
}

#[test]
fn wire_keys_are_part_of_the_contract() {
    // TWO FAILURE MODES, and only one of them needs this test.
    //
    // Renaming a REQUIRED field (current_period, secs_in_period, scores, penalties,
    // warnings, fouls, is_old_game, game_number, next_game_number) makes serde error
    // with `missing field`, which `every_captured_line_decodes` and
    // `decoded_values_match_the_capture` already catch loudly.
    //
    // Renaming an OPTION field is SILENT: serde fills in None, nothing errors, and the
    // overlay simply stops seeing that value forever with no signal anywhere. Those are
    // the load-bearing entries below. Do NOT "tidy" this list back to the
    // prominent-looking fields -- that removes the only protection this test adds.
    // Classified against `GameSnapshot` in this workspace, which the overlay uses by path.
    for key in [
        "\"timeout\"",
        "\"event_id\"",
        "\"recent_goal\"",
        "\"next_period_len_secs\"",
        "\"conf_pause_time\"",
    ] {
        assert!(
            CAPTURE.contains(key),
            "OPTION wire key {key} vanished -- a rename here defaults to None silently"
        );
    }
    // Belt and braces: two required keys as a corpus-shape check. The decode tests are
    // what really guard these, because a rename makes them hard-fail.
    for key in ["\"scores\"", "\"current_period\""] {
        assert!(
            CAPTURE.contains(key),
            "wire key {key} vanished from the contract"
        );
    }
    // The score bundle's own colour keys.
    assert!(
        CAPTURE.contains("\"black\""),
        "score bundle key \"black\" vanished"
    );
    assert!(
        CAPTURE.contains("\"white\""),
        "score bundle key \"white\" vanished"
    );
}
