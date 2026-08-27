//! Reading the message out of a caught panic.
//!
//! A leaf utility with no dependencies of its own, so both the application layer and the
//! engine's test harness can use it without either depending on the other. It lives here
//! rather than in `app` because `tournament_manager` must not depend on `app`.

/// Read the human-readable reason out of a caught panic.
///
/// `panic!("...")` produces either a `&'static str` or a `String` depending on whether the
/// message was formatted; anything else carries no text to show.
pub(crate) fn panic_reason(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "panic with an unrecognised payload".to_string()
    }
}
