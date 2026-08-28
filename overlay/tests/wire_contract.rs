//! The wire contract between the refbox and this overlay, asserted on DECODED VALUES.
//!
//! The overlay builds in the refbox's own workspace against `uwh-common` by path, so a
//! renamed REQUIRED field is already a loud failure. What compilation cannot catch is a
//! change that keeps building while the MEANING of the data moves -- serde ignores unknown
//! fields, so both sides compile happily through exactly that, and non-Rust consumers such
//! as the vMix bridge get no compile-time protection at all. So: assert the decoded
//! numbers, and round-trip each snapshot back to JSON to check the keys the struct emits
//! TODAY still match the capture -- renaming an `Option` is what silently breaks a
//! consumer, because everything compiles, every line still decodes, and the field simply
//! arrives as None forever.
//!
//! KNOWN GAP: this fixture was captured before `portal_base_url` was added to
//! `GameSnapshot`, so that key is NOT pinned below even though it is an `Option`
//! field whose rename would be silent -- and it is the field the vMix bridge
//! depends on entirely. Closing it needs a FRESH CAPTURE from a refbox carrying
//! the field; do not hand-add the key to these messages, they are real captures
//! and their value is that nobody edited them.
//!
//! The gap is exactly that and no wider: the round-trip check below covers every key
//! the capture DOES contain, including `event_id` and `conf_pause_time`, which are null
//! on all eleven lines and so can never be guarded by a decoded value.
//!
//! TO RECAPTURE, when a new REQUIRED field is added to `GameSnapshot` and these lines
//! stop decoding: run a refbox, connect to its snapshot feed on the port the overlay
//! uses, and save the raw lines it emits -- one JSON object per line, no edits. Replace
//! this file wholesale and update the corpus size in `every_captured_line_decodes`. A
//! hand-patched fixture is worse than a stale one: it stops being evidence of what the
//! refbox actually sends.

use uwh_common::color::Color;
use uwh_common::game_snapshot::{GamePeriod, GameSnapshot, TimeoutSnapshot};

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

    // The Option fields that carry a non-null value ANYWHERE in this corpus, asserted on
    // the one line each appears on. A rename of an Option is silent -- serde fills in
    // None and nothing errors -- so these are the only lines where a decoded value can
    // prove the field still arrives. `event_id` and `conf_pause_time` are null on all
    // eleven captured lines, so no assertion here can cover them; the round-trip check
    // below is what guards those two.
    assert_eq!(snapshots()[7].timeout, Some(TimeoutSnapshot::Black(30)));
    assert_eq!(snapshots()[2].recent_goal, Some((Color::Black, 6)));
    assert_eq!(snapshots()[0].next_period_len_secs, Some(90));
}

#[test]
fn wire_keys_are_part_of_the_contract() {
    // Re-serialize each decoded snapshot and compare it against the line it came from.
    //
    // The obvious form of this check -- `assert!(CAPTURE.contains("\"event_id\""))` --
    // is CIRCULAR, and it was live in this file until review caught it. CAPTURE is this
    // test's own input, so such an assertion passes forever no matter what GameSnapshot
    // does. Demonstrated by putting `#[serde(rename = "eventId")]` on the field: all
    // three tests stayed green while every consumer would silently have seen None for
    // good. Mutating the FIXTURE makes it fail, which is what made it look like a guard.
    //
    // Round-tripping asks the STRUCT what it emits today, so a renamed, removed or
    // retyped field is visible here even though every line still decodes happily.
    for (i, line) in CAPTURE.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        let captured: serde_json::Value =
            serde_json::from_str(line).expect("captured line must be a JSON object");
        let produced =
            serde_json::to_value(&snapshots()[i]).expect("GameSnapshot must re-serialize");

        let captured = captured.as_object().expect("captured line is an object");
        let produced = produced
            .as_object()
            .expect("GameSnapshot serializes as an object");

        for (key, want) in captured {
            let got = produced.get(key).unwrap_or_else(|| {
                panic!(
                    "line {i}: wire key {key:?} is no longer emitted by GameSnapshot -- a \
                     rename or removal keeps decoding (an Option silently becomes None) \
                     and breaks every consumer, including non-Rust ones such as the vMix \
                     bridge"
                )
            });
            assert_eq!(
                got, want,
                "line {i}: wire key {key:?} round-trips to a different value"
            );
        }
    }
}
