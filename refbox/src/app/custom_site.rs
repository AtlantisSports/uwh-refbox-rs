//! Parsing of the single URL an operator types for a custom game source.
//!
//! The operator types one string carrying both the site and the event, e.g.
//! `http://scoreboard.local:8099/api/events/1234-A`. refbox needs the two
//! halves separately: the base URL for the client, and the event ID for every
//! call that names an event.
//!
//! Pure by design — no I/O and no app state — so the rules are unit-testable.

use uwh_common::uwhportal::schedule::EventId;

/// The site half and the event half of what the operator typed, once
/// separated and validated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedSite {
    pub base_url: String,
    pub event_id: EventId,
}

/// Why a typed URL cannot be used. Each variant is a separate thing to tell
/// the operator, so the UI can name what is actually wrong rather than
/// reporting a generic failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustomSiteError {
    /// Nothing typed.
    Empty,
    /// A scheme other than `http` or `https`. refbox derives whether TLS is
    /// required from this scheme, so no other scheme has a meaning here.
    UnsupportedScheme,
    /// No `/api/events/` anywhere in the path, so there is no event to find.
    MissingEventsSegment,
    /// `/api/events/` is present but nothing follows it.
    MissingEventId,
    /// An event ID too short for `uwh-common` to accept. Catching this at
    /// entry is the whole point of this function: `uwh-common` validates event
    /// IDs when it deserialises a response, and a violation fails the *entire*
    /// response rather than the single field.
    EventIdTooShort,
}

const EVENTS_SEGMENT: &str = "/api/events/";

pub fn parse_custom_site(input: &str) -> Result<ParsedSite, CustomSiteError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(CustomSiteError::Empty);
    }
    if !input.starts_with("http://") && !input.starts_with("https://") {
        return Err(CustomSiteError::UnsupportedScheme);
    }

    let (base, rest) = input
        .rsplit_once(EVENTS_SEGMENT)
        .ok_or(CustomSiteError::MissingEventsSegment)?;

    let partial = rest.split_once('/').map_or(rest, |(id, _)| id);
    if partial.is_empty() {
        return Err(CustomSiteError::MissingEventId);
    }

    // Built through the public constructor rather than re-checking the rules
    // here, so this cannot drift from uwh-common's own validation.
    let event_id = EventId::from_full(format!("events/{partial}"))
        .map_err(|_| CustomSiteError::EventIdTooShort)?;

    // The client trims a trailing slash too, but the SITE row displays this
    // string, so normalise it once, here.
    Ok(ParsedSite {
        base_url: base.trim_end_matches('/').to_string(),
        event_id,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    // ---- Accepted ----

    #[test]
    fn parses_http_url_into_base_and_event() {
        let parsed = parse_custom_site("http://scoreboard.local:8099/api/events/1234-A").unwrap();
        assert_eq!(parsed.base_url, "http://scoreboard.local:8099");
        assert_eq!(parsed.event_id.partial(), "1234-A");
    }

    #[test]
    fn parses_https_url_into_base_and_event() {
        let parsed = parse_custom_site("https://scoreboard.example.com/api/events/1234-A").unwrap();
        assert_eq!(parsed.base_url, "https://scoreboard.example.com");
        assert_eq!(parsed.event_id.partial(), "1234-A");
    }

    #[test]
    fn accepts_a_trailing_slash_after_the_event_id() {
        let parsed = parse_custom_site("http://scoreboard.local:8099/api/events/1234-A/").unwrap();
        assert_eq!(parsed.base_url, "http://scoreboard.local:8099");
        assert_eq!(parsed.event_id.partial(), "1234-A");
    }

    #[test]
    fn accepts_the_shortest_legal_event_id() {
        // Three characters after the prefix is the minimum uwh-common allows.
        let parsed = parse_custom_site("http://scoreboard.local:8099/api/events/ABC").unwrap();
        assert_eq!(parsed.event_id.partial(), "ABC");
    }

    #[test]
    fn accepts_a_long_hyphenated_event_id() {
        // Event IDs are opaque beyond the length rule — no splitting on hyphens.
        let parsed =
            parse_custom_site("http://scoreboard.local:8099/api/events/1234-A-extremely-long")
                .unwrap();
        assert_eq!(parsed.event_id.partial(), "1234-A-extremely-long");
    }

    // ---- Rejected, each with its own error ----

    #[test]
    fn rejects_a_url_with_no_events_segment() {
        assert_eq!(
            parse_custom_site("http://scoreboard.local:8099/"),
            Err(CustomSiteError::MissingEventsSegment)
        );
    }

    #[test]
    fn rejects_nothing_after_the_events_segment() {
        assert_eq!(
            parse_custom_site("http://scoreboard.local:8099/api/events/"),
            Err(CustomSiteError::MissingEventId)
        );
    }

    #[test]
    fn rejects_an_event_id_shorter_than_three_characters() {
        assert_eq!(
            parse_custom_site("http://scoreboard.local:8099/api/events/AB"),
            Err(CustomSiteError::EventIdTooShort)
        );
    }

    #[test]
    fn rejects_a_scheme_that_is_not_http_or_https() {
        assert_eq!(
            parse_custom_site("ftp://scoreboard.local:8099/api/events/1234-A"),
            Err(CustomSiteError::UnsupportedScheme)
        );
    }

    #[test]
    fn rejects_empty_input() {
        assert_eq!(parse_custom_site(""), Err(CustomSiteError::Empty));
        assert_eq!(parse_custom_site("   "), Err(CustomSiteError::Empty));
    }
}
