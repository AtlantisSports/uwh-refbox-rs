//! Parsing of the single URL an operator types for a custom game source.
//!
//! The operator types one string carrying both the site and the event, e.g.
//! `https://scoreboard.local:8099/api/1234-A`. refbox needs the two halves
//! separately: the base URL for the client, and the event ID for every call
//! that names an event.
//!
//! Only those two things are asked for. The rest of each address —
//! `/api/events/{id}/teams` and the other seven calls — is refbox's own path
//! convention, which a third-party site implements to match and which the
//! client appends itself, so requiring the operator to type it would be
//! requiring them to type what the software already knows.
//!
//! The `/api/` marker is still required, and that is deliberate: with no fixed
//! segment there would be nothing separating the site from the event ID, so any
//! three-character path segment — a homepage, a login page — would parse as a
//! valid event and only fail later, on the first call.
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

/// Why a typed URL cannot be used.
///
/// **The UI does not distinguish these.** It shows one message naming the shape
/// an address must have, because five specific messages would have cost 75
/// translations for a field the operator retypes anyway. The variants are kept
/// separate so that decision can be revisited without re-deriving the rules,
/// and because the tests assert against them individually.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustomSiteError {
    /// Nothing typed.
    Empty,
    /// A scheme other than `http` or `https`. refbox derives whether TLS is
    /// required from this scheme, so no other scheme has a meaning here.
    UnsupportedScheme,
    /// A scheme but no host, e.g. `https:///api/1234-A`. Left unchecked this
    /// yields a base URL of `https:`, which no request can reach — and the SITE
    /// row would then show that as the address in use.
    MissingHost,
    /// No `/api/` anywhere in the path, so nothing marks where the site ends
    /// and the event ID begins.
    MissingApiSegment,
    /// The marker is present but no event ID follows it.
    MissingEventId,
    /// An event ID too short for `uwh-common` to accept. Catching this at
    /// entry is the whole point of this function: `uwh-common` validates event
    /// IDs when it deserialises a response, and a violation fails the *entire*
    /// response rather than the single field.
    EventIdTooShort,
}

use reqwest::Url;

const API_SEGMENT: &str = "/api/";

/// The shape the address had before it was shortened. Still accepted, so that an
/// address already typed into a config — and the examples in the integration
/// document — keep working. Tried first: splitting `/api/events/1234-A` on the
/// shorter marker alone would take `events` for the event ID.
const LEGACY_EVENTS_SEGMENT: &str = "/api/events/";

pub fn parse_custom_site(input: &str) -> Result<ParsedSite, CustomSiteError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(CustomSiteError::Empty);
    }
    let (scheme, after_scheme) = if let Some(rest) = input.strip_prefix("https://") {
        ("https://", rest)
    } else if let Some(rest) = input.strip_prefix("http://") {
        ("http://", rest)
    } else {
        return Err(CustomSiteError::UnsupportedScheme);
    };

    // Separate the host from the path *before* looking for the marker, and look
    // only in the path. Searching the whole address let the `//api/` belonging to
    // the authority match: in `https://api/1234-A` the host was taken for the
    // marker and thrown away, leaving a base URL of `https:` that no request can
    // reach — and the address was accepted rather than refused, so APPLY
    // repointed the refbox at it and the SITE row displayed it as the address in
    // use. That is the silent misdirection this whole parser exists to prevent.
    let (authority, path) = after_scheme.split_once('/').unwrap_or((after_scheme, ""));
    if authority.is_empty() {
        return Err(CustomSiteError::MissingHost);
    }
    // Restore the leading slash the split consumed, so a marker at the very
    // start of the path (`/api/1234-A`, the common case) is still found.
    let path = format!("/{path}");

    let (path_prefix, rest) = path
        .rsplit_once(LEGACY_EVENTS_SEGMENT)
        .or_else(|| path.rsplit_once(API_SEGMENT))
        .ok_or(CustomSiteError::MissingApiSegment)?;

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
        base_url: format!("{scheme}{authority}{path_prefix}")
            .trim_end_matches('/')
            .to_string(),
        event_id,
    })
}

/// The address to report for a site, with any `user:password@` credentials removed, or `None`
/// when it cannot be parsed.
///
/// The refbox reports its portal address on the JSON feed (`update_sender`), which binds to every
/// interface with no authentication -- anyone on the pool LAN can read it. `parse_custom_site`
/// deliberately stores the authority exactly as the operator typed it, so an address entered with
/// embedded credentials would otherwise put the password on the wire.
///
/// Stripping happens at the point of reporting rather than the point of entry: refusing
/// credentials when they are typed would change how an existing feature behaves, and is a
/// separate decision.
///
/// Parsing is delegated to the `url` crate that `reqwest` itself uses, rather than scanning for
/// `@` by hand. A hand-rolled version ended the authority at `/` alone, so it read
/// `https://host?q=a@b` as having the host `b` and reported a different machine than the one
/// refbox actually calls -- the silent misdirection this module exists to prevent. Sharing the
/// HTTP client's own parser means the reported host cannot disagree with the host requests go to.
///
/// `None` rather than a guess when the address will not parse: `https://user@` has no host at
/// all, and an unreachable address presented as authoritative is worse than no address.
pub fn strip_credentials(base_url: &str) -> Option<String> {
    let mut url = Url::parse(base_url).ok()?;

    if url.username().is_empty() && url.password().is_none() {
        // Nothing to remove. Return the original verbatim rather than the parser's normalised
        // form, so the overwhelmingly common case is not reshaped -- `Url` would append a
        // trailing slash that `parse_custom_site` deliberately trims.
        return Some(base_url.to_string());
    }

    url.set_username("").ok()?;
    url.set_password(None).ok()?;
    Some(url.as_str().trim_end_matches('/').to_string())
}

#[cfg(test)]
mod test {
    use super::*;

    // ---- Accepted ----

    /// The shape the operator is asked for: their site, then the event ID. The
    /// `events/` segment is refbox's own convention and is not typed.
    #[test]
    fn parses_the_short_form_into_base_and_event() {
        let parsed = parse_custom_site("https://scoreboard.local:8099/api/1234-A").unwrap();
        assert_eq!(parsed.base_url, "https://scoreboard.local:8099");
        assert_eq!(parsed.event_id.partial(), "1234-A");
    }

    /// A site reached through a path prefix keeps that prefix in its base.
    #[test]
    fn parses_a_site_behind_a_path_prefix() {
        let parsed = parse_custom_site("https://club.example/scoreboard/api/1234-A").unwrap();
        assert_eq!(parsed.base_url, "https://club.example/scoreboard");
        assert_eq!(parsed.event_id.partial(), "1234-A");
    }

    /// The longer form must keep working: it is what already-configured
    /// refboxes hold and what the integration document's examples show. Split on
    /// the short marker alone it would yield `events` as the event ID.
    #[test]
    fn the_longer_events_form_is_still_accepted() {
        let parsed = parse_custom_site("https://scoreboard.local:8099/api/events/1234-A").unwrap();
        assert_eq!(parsed.base_url, "https://scoreboard.local:8099");
        assert_eq!(parsed.event_id.partial(), "1234-A");
    }

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

    /// Without the marker there is nothing separating site from event, so an
    /// ordinary page on the same site must not be mistaken for an event address.
    #[test]
    fn rejects_a_url_with_no_api_segment() {
        for url in [
            "http://scoreboard.local:8099/",
            "http://scoreboard.local:8099/1234-A",
            "http://scoreboard.local:8099/login",
            "http://scoreboard.local:8099/api",
        ] {
            assert_eq!(
                parse_custom_site(url),
                Err(CustomSiteError::MissingApiSegment),
                "{url:?} should be refused"
            );
        }
    }

    #[test]
    fn rejects_nothing_after_the_marker() {
        assert_eq!(
            parse_custom_site("http://scoreboard.local:8099/api/"),
            Err(CustomSiteError::MissingEventId)
        );
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

    /// An address with no host at all must be refused rather than yielding a
    /// base URL of `https:`, which cannot be reached and which the SITE row
    /// would then display as the address in use.
    #[test]
    fn rejects_an_address_with_no_host() {
        assert_eq!(
            parse_custom_site("https:///api/ABC"),
            Err(CustomSiteError::MissingHost)
        );
        assert_eq!(
            parse_custom_site("http:///api/events/1234-A"),
            Err(CustomSiteError::MissingHost)
        );
    }

    /// The marker must be looked for in the path, never in the authority. This
    /// address has no `/api/` of its own — the only candidate is the `//api/`
    /// belonging to the host — so it must be refused for the missing marker.
    /// It used to be accepted, with the host silently eaten and the base URL
    /// left as `https:`.
    #[test]
    fn a_host_called_api_is_not_mistaken_for_the_marker() {
        assert_eq!(
            parse_custom_site("https://api/1234-A"),
            Err(CustomSiteError::MissingApiSegment)
        );
    }

    /// And with a real marker present, a host called `api` works normally.
    #[test]
    fn a_host_called_api_works_when_the_marker_is_there() {
        let parsed = parse_custom_site("https://api/api/1234-A").unwrap();
        assert_eq!(parsed.base_url, "https://api");
        assert_eq!(parsed.event_id.partial(), "1234-A");
    }

    // ---- Credential stripping ----
    //
    // The base URL is reported on the refbox's unauthenticated JSON feed, so anything the
    // operator typed into it reaches every client on the pool LAN.

    #[test]
    fn an_ordinary_address_is_left_exactly_as_it_was() {
        assert_eq!(
            strip_credentials("https://scoreboard.local:8099").as_deref(),
            Some("https://scoreboard.local:8099")
        );
    }

    #[test]
    fn a_path_prefix_survives_unchanged() {
        assert_eq!(
            strip_credentials("https://club.example/scoreboard").as_deref(),
            Some("https://club.example/scoreboard")
        );
    }

    #[test]
    fn a_username_and_password_are_removed_from_the_authority() {
        assert_eq!(
            strip_credentials("https://scorekeeper:hunter2@scoreboard.local:8099").as_deref(),
            Some("https://scoreboard.local:8099")
        );
    }

    #[test]
    fn a_bare_username_with_no_password_is_removed_too() {
        assert_eq!(
            strip_credentials("https://scorekeeper@scoreboard.local").as_deref(),
            Some("https://scoreboard.local")
        );
    }

    #[test]
    fn credentials_are_removed_but_a_path_prefix_is_kept() {
        assert_eq!(
            strip_credentials("https://user:pw@club.example/scoreboard").as_deref(),
            Some("https://club.example/scoreboard")
        );
    }

    #[test]
    fn http_is_handled_the_same_as_https() {
        assert_eq!(
            strip_credentials("http://user:pw@scoreboard.local:8099").as_deref(),
            Some("http://scoreboard.local:8099")
        );
    }

    // ---- The host must never change ----
    //
    // These are the cases a hand-rolled parser got wrong: it ended the authority at `/` only, so
    // an `@` after a `?`, `#` or `\` made it discard the real host and report a different one.
    // Reporting the wrong host is worse than reporting credentials -- it is the silent
    // misdirection this whole module exists to prevent.

    #[test]
    fn a_query_string_containing_an_at_sign_does_not_change_the_host() {
        assert_eq!(
            strip_credentials("https://scoreboard.local?q=a@b").as_deref(),
            Some("https://scoreboard.local?q=a@b")
        );
    }

    #[test]
    fn a_fragment_containing_an_at_sign_does_not_change_the_host() {
        assert_eq!(
            strip_credentials("https://scoreboard.local#f@g").as_deref(),
            Some("https://scoreboard.local#f@g")
        );
    }

    #[test]
    fn a_path_containing_an_at_sign_does_not_change_the_host() {
        assert_eq!(
            strip_credentials("https://scoreboard.local/team@pool").as_deref(),
            Some("https://scoreboard.local/team@pool")
        );
    }

    /// A backslash ends the authority too, per WHATWG -- which is why the host here is
    /// `scoreboard.local` and not `pool`.
    #[test]
    fn a_backslash_before_an_at_sign_does_not_change_the_host() {
        assert_eq!(
            strip_credentials(r"https://scoreboard.local\team@pool").as_deref(),
            Some(r"https://scoreboard.local\team@pool")
        );
    }

    /// A password containing an `@` splits on the last one, as URL parsing does.
    #[test]
    fn the_last_at_sign_in_the_authority_is_the_delimiter() {
        assert_eq!(
            strip_credentials("https://user:p%40ss@scoreboard.local").as_deref(),
            Some("https://scoreboard.local")
        );
    }

    // ---- Nothing is invented ----

    /// An authority that is only userinfo leaves no host to report. Reporting `https://` -- an
    /// address no request can reach -- would be a plausible-looking lie, so report nothing.
    #[test]
    fn an_address_with_credentials_but_no_host_reports_nothing() {
        assert_eq!(strip_credentials("https://user@"), None);
    }

    #[test]
    fn a_string_that_is_not_an_address_at_all_reports_nothing() {
        assert_eq!(strip_credentials("not-an-address"), None);
    }
}
