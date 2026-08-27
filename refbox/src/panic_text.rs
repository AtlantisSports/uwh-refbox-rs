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

#[cfg(test)]
mod tests {
    use super::panic_reason;

    #[test]
    fn reads_both_payload_shapes_and_neither() {
        let from_str: Box<dyn std::any::Any + Send> = Box::new("No snapshot");
        assert_eq!(panic_reason(&*from_str), "No snapshot");

        let from_string: Box<dyn std::any::Any + Send> = Box::new(String::from("boom"));
        assert_eq!(panic_reason(&*from_string), "boom");

        let from_other: Box<dyn std::any::Any + Send> = Box::new(7u8);
        assert_eq!(
            panic_reason(&*from_other),
            "panic with an unrecognised payload"
        );
    }
}
