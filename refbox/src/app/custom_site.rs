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

use reqwest::Url;

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
    /// Text where the host should be that a URL parser does not read as a host,
    /// e.g. `https://scoreboard.local\scorekeeper:hunter2@pool`. See
    /// [`is_bare_authority`] for why this is refused rather than tidied up.
    MalformedHost,
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
    // Judged on `authority` itself -- the slice this function goes on to build the base URL from.
    // Checking a differently-computed slice would let the two disagree: `https://host?x=1/api/E`
    // would pass a check that stopped at the `?` while the base URL kept `host?x=1`, and every
    // request would then be swallowed into a query string.
    if !is_bare_authority(authority) {
        return Err(CustomSiteError::MalformedHost);
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
/// when the address is not one the refbox could talk to.
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
/// **The reported address is always the parser's normalised form, never the operator's raw
/// string.** An earlier version returned the raw string whenever there were no credentials to
/// remove, which was worse in three ways, all of them silent: `https://host\team@pool` was
/// republished with the backslash, which every RFC 3986 parser (Python, Go, Java) reads as host
/// `pool` while refbox calls `host`; `https://@host` kept a credential-shaped authority the
/// function promises to remove; and merely adding credentials to a site changed the reported
/// address in unrelated ways, because only one of the two branches lower-cased the host and
/// dropped the default port. Normalising unconditionally means the address published is the
/// address requests actually go to, which is the entire reason for borrowing `reqwest`'s parser
/// rather than scanning for `@` by hand.
///
/// `None` rather than a guess in two cases. An address that will not parse at all, and one whose
/// scheme is not http(s) -- `Url` happily accepts `mailto:`, `file:`, `javascript:` and, worse,
/// reads `localhost:8080` (a natural typo for `http://localhost:8080`) as scheme `localhost`. The
/// override environment variables are not validated anywhere else, so without this check a typo
/// is published as the authoritative portal address, and a consumer rendering it as a link would
/// be handed a `javascript:` URL.
///
/// Note what is NOT removed: only userinfo. A secret an operator puts elsewhere in the address --
/// `https://host/x?token=SECRET` -- is still published. The address must not be treated as safe
/// to share on that basis.
pub fn strip_credentials(base_url: &str) -> Option<String> {
    let mut url = Url::parse(base_url).ok()?;

    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }

    // Checked here rather than at each caller so that every sink inherits it: the log line, the
    // game feed -- which calls this directly and reaches every device on the pool LAN -- and
    // anything added later.
    if !is_bare_authority(authority_of(base_url)?) {
        return None;
    }

    // These two `?`s are load-bearing, not defensive. `set_username`/`set_password` return Err
    // when the URL has no host or an empty domain host -- which is exactly how `https://user@`
    // ends up reporting nothing -- and also for the `file` scheme, which the check above already
    // excluded. Do not assume the scheme check alone makes them infallible.
    url.set_username("").ok()?;
    url.set_password(None).ok()?;

    let mut reported = url.as_str().to_string();
    // `Url` renders an empty path as a single `/`, which `parse_custom_site` and `portal_target`
    // both deliberately trim. Drop exactly that one slash, and only when nothing follows it --
    // trimming the whole string would take a slash off a query value instead
    // (`https://host/?a=1/` must not become `https://host/?a=1`).
    if url.path() == "/" && url.query().is_none() && url.fragment().is_none() {
        reported.pop();
    }
    Some(reported)
}

/// Whether `authority` -- the text where the host goes -- is text a URL parser reads as a host.
///
/// Splitting an address on its first `/` finds *a* substring; it does not establish that the
/// substring is an authority. A URL parser reads `\` as `/` for http(s), so in
/// `https://scoreboard.local\scorekeeper:hunter2@pool` the host is only `scoreboard.local` and
/// the rest is path. Two things go wrong at once. The refbox contacts an address the operator
/// never named, and anything meant as a password sits in the path, where credential stripping
/// cannot reach it -- so it travels into the log file, on to the game feed, and into the URL of
/// every network error message.
///
/// What is actually rejected, stated plainly rather than implied:
///
/// * a `\` anywhere, which is not a legal authority character and is what re-opens the path;
/// * whitespace, which the parser either trims or silently deletes -- `https://host /api/1234-A`
///   would be committed and then fail every call, and a tab inside the host is removed outright,
///   so `scoreboard<TAB>local` resolves to `scoreboardlocal`;
/// * anything the parser will not read as a bare host -- `hunter2%40host` and the full-width
///   `hunter2＠host` both leave a port that is not a number.
///
/// It is not a test for secrets, and it is not asked to be. It asks only whether this text means
/// what it looks like; where it does not, no later code can be sure what it is handling.
///
/// **Known gap, stated so it is not mistaken for covered.** The parser also deletes the Unicode
/// "format" characters -- U+00AD SOFT HYPHEN, U+200B ZERO WIDTH SPACE, U+FEFF and the rest of
/// category Cf -- which are neither whitespace nor control, so they pass this check. That means
/// `https://scoreboard.local\u{AD}evil.example/api/1234-A` is accepted and every request goes to
/// `scoreboard.localevil.example`, a host the operator cannot see in what they typed. Catching it
/// needs the Unicode tables the `idna` crate has and `char` does not; listing the codepoints by
/// hand would cover part of the class while reading as though it covered all of it, which is worse
/// than saying plainly that it does not. No credential escapes this way -- the address is still
/// redacted wherever it is printed -- but the refbox does talk to the wrong machine.
///
/// A control character is not checked for, deliberately: every one of them is either whitespace,
/// caught above, or rejected by the host parser on its own (U+0001 and U+007F both fail). A clause
/// for them would be a condition no input could make false.
///
/// Refusing such an address costs nothing. It could not work before either -- the requests went
/// somewhere else -- so the only change is that the refbox now says so instead of failing later.
fn is_bare_authority(authority: &str) -> bool {
    if authority.contains('\\') || authority.chars().any(char::is_whitespace) {
        return false;
    }
    // Probed under a fixed scheme: only the shape is in question here, never which scheme it is.
    // That the probe scheme is a *special* one is load-bearing: `Url` will not parse `https://`
    // without a host, so a successful parse already establishes there is one, and the path of a
    // bare authority is always `/` rather than empty. Testing either again here would be scenery
    // -- a condition no input can make false, which no test could ever pin.
    Url::parse(&format!("https://{authority}")).is_ok_and(|probe| {
        probe.path() == "/" && probe.query().is_none() && probe.fragment().is_none()
    })
}

/// The authority of an address: what follows `://`, up to the first `/`, `?` or `#`.
///
/// `parse_custom_site` deliberately does *not* use this. It judges the slice it goes on to build
/// the base URL from, which stops only at `/` -- so a `?` or `#` falls inside the authority there
/// and is refused. That is the stricter reading, and the right one for an address being committed:
/// a site address has a path, so a query where the path should start is a mistake. Here, where any
/// address may need printing, the looser reading is right -- `https://host?q=1` is a perfectly
/// good thing to log.
fn authority_of(addr: &str) -> Option<&str> {
    let (_, after_scheme) = addr.split_once("://")?;
    // `split` always yields at least one item, so this never falls through.
    after_scheme.split(['/', '?', '#']).next()
}

/// A site address that never prints its own credentials.
///
/// The refbox logs where it is pointed in several places, and a custom site is stored exactly as
/// the operator typed it -- so an address entered as `https://user:password@host/...` reached the
/// log file verbatim. On a Pi those logs rotate to disk and are shared when something is being
/// diagnosed.
///
/// Redacting at each log site was rejected: there were six of them already, `SiteTarget` derives
/// `Debug` so dumping the whole struct leaked too, and the next log line anyone adds would leak
/// again. Putting the redaction in `Display` and `Debug` makes every present and future printer
/// safe by construction.
///
/// What it removes is the `user:password@` part of an address, which is the only place the refbox
/// itself ever puts a credential. It is not a general secret scrubber: a token an operator chose
/// to hide in a query string is still printed, exactly as [`strip_credentials`] says.
///
/// The real value is reachable only through [`SiteAddress::expose`].
#[derive(Clone, PartialEq, Eq)]
pub struct SiteAddress(String);

impl SiteAddress {
    /// The address as typed, credentials included.
    ///
    /// Outside tests this has three callers, none of which prints it: building the HTTP client
    /// (`build_site_client`), testing the scheme (`https_policy_conflict`), and deciding what
    /// to publish on the game feed. Anything that *logs* an address must go through `Display`.
    ///
    /// That last caller is not a duplicate of this type's own redaction, and the difference is
    /// load bearing. `Display` must always produce *something*, because a log line with nothing in
    /// it is useless, so an address it cannot clean becomes a placeholder. The feed must instead
    /// publish *nothing* at all rather than a placeholder, because a consumer would take any
    /// string it was handed as a real address. Same redaction, opposite fallbacks.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

/// Text the operator is part-way through typing into the SITE row.
///
/// `Message` derives `Debug` so that every message can be traced, and that trace runs on every
/// keystroke -- so a plain `String` here put the address, password and all, into the log file
/// whenever the refbox was run with `-vv`. Which is exactly what someone is asked to run when a
/// problem is being diagnosed, and therefore exactly when the log gets shared.
///
/// Redacting it the way [`SiteAddress`] does would not work: half-typed text does not parse, so
/// there would be nothing left to show. The trace is there to say *which* message was handled, so
/// the content is simply left out of it.
#[derive(Clone, PartialEq, Eq)]
pub struct TypedSiteUrl(String);

impl TypedSiteUrl {
    /// The text itself, for the one caller that stores it.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<String> for TypedSiteUrl {
    fn from(raw: String) -> Self {
        Self(raw)
    }
}

impl std::fmt::Debug for TypedSiteUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<what the operator is typing: not logged>")
    }
}

/// The text logged in place of an address that could not be proved credential-free.
pub const WITHHELD_ADDRESS: &str = "<address withheld: it may contain a password>";

/// An address with any `user:password@` removed, ready to be logged.
///
/// An address is shown only when [`strip_credentials`] could actually clean it -- that is, it
/// parsed as an `http(s)` URL and the credentials were removed from a value the parser understood.
/// Anything else is withheld. Nothing here ever echoes back the string it was handed.
///
/// That last rule is the whole design, and it was learned three times over. Echoing an address
/// that failed to parse cannot be made safe: whatever test decides it looks harmless, the parser
/// reads some string differently from the way it looks and the password rides out on the
/// difference. Searching for an `@` missed `hunter2%40host`, the full-width `hunter2\u{FF20}host`
/// a CJK IME produces, and `user:pw@host` typed without a scheme, which parses as scheme `user`.
/// Re-parsing to prove there was no credential then missed `https://host\user:pw@pool`, where a
/// backslash is read as a slash, and `foo:/user:pw@host`, where prefixing a scheme demotes the
/// authority to a path -- each proved something about a string other than the one being printed.
/// A raw newline slipped through both, splitting one log line into two.
///
/// So the address is not the thing worth printing. A caller that needs to say an address is
/// unusable names the reason -- [`CustomSiteError`] -- which says what to fix rather than leaving
/// the operator to spot it, and cannot carry a secret at all.
///
/// What this removes is the `user:password@` part, the only place the refbox itself ever puts a
/// credential. It is not a general secret scrubber: a token an operator chose to hide in a query
/// string is still printed, exactly as [`strip_credentials`] says.
fn redacted(addr: &str) -> String {
    strip_credentials(addr).unwrap_or_else(|| WITHHELD_ADDRESS.to_string())
}

impl std::fmt::Display for SiteAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&redacted(&self.0))
    }
}

impl std::fmt::Debug for SiteAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `{:?}` must not be a way around the redaction. Quote a real address -- this field used to
        // be a `String`, and the quotes are what make a space-padded one visible -- but never the
        // placeholder, which is a sentence about the address and would read as one that was typed.
        match strip_credentials(&self.0) {
            Some(clean) => write!(f, "{clean:?}"),
            None => f.write_str(WITHHELD_ADDRESS),
        }
    }
}

impl From<String> for SiteAddress {
    fn from(raw: String) -> Self {
        Self(raw)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // ---- An address must never print its own credentials ----
    //
    // A custom site address is stored exactly as the operator typed it, so
    // `https://user:password@host/...` reached the log file verbatim -- and on a Pi those logs
    // rotate to disk and get shared when something is being diagnosed.

    const LEAKY: &str = "https://scorekeeper:hunter2@scoreboard.local:8099/api/1234-A";

    /// Every address that cannot be cleaned, from three rounds of review. Each one carries a
    /// password past some rule that was supposed to be enough, which is why the rule is now
    /// "clean it or say nothing" rather than any test of how the address looks.
    const UNCLEANABLE: &[&str] = &[
        // Withheld because it will not parse at all.
        "https://user:hunter2@",
        // Withheld because refbox will not speak the scheme.
        "ftp://user:hunter2@scoreboard.local",
        // Found looking for an `@`: a full-width one, the ordinary output of a CJK IME.
        "https://scorekeeper:hunter2\u{FF20}scoreboard.local:8099/api/1234-A",
        // ...and a percent-encoded one.
        "https://scorekeeper:hunter2%40scoreboard.local",
        // ...and no scheme at all, which parses as scheme `scorekeeper` and reports no credentials.
        "scorekeeper:hunter2@scoreboard.local:8099/api/1234-A",
        // Found re-parsing to prove there was nothing to hide: a backslash reads as a slash, so
        // the credentials land in what looks like a path.
        r"https://scoreboard.local\scorekeeper:hunter2@pool",
        // ...and one slash instead of two demotes the authority to a path when a scheme is added.
        "foo:/scorekeeper:hunter2@host",
        // Found checking the *result* for an `@` instead: a backslash moves the credentials into
        // the path, and once there the parser percent-encodes the separator, so no `@` is left to
        // find. The check is on the authority now, so how the separator was written stops mattering.
        r"https://scoreboard.local\scorekeeper:hunter2%40pool",
        "https://scoreboard.local\\scorekeeper:hunter2\u{FF20}pool",
    ];

    /// Each condition in the authority check, given a case of its own -- otherwise a clause could
    /// be deleted, or break, with the whole suite still green.
    #[test]
    fn only_a_bare_authority_passes_the_authority_check() {
        for ok in [
            "scoreboard.local",
            "scoreboard.local:8099",
            "user:pw@host",
            "[::1]:8099",
        ] {
            assert!(is_bare_authority(ok), "should accept {ok}");
        }
        for bad in [
            "",                     // nothing where the host goes
            "scoreboard.local/x",   // a path
            "scoreboard.local?x=1", // a query
            "scoreboard.local#f",   // a fragment
            r"scoreboard.local\x",  // a backslash, read as a path separator
            "scoreboard.local ",    // trailing space, trimmed away by the parser
            // A tab inside the host: the parser deletes it rather than refusing, so without the
            // whitespace clause this resolves to `scoreboardlocal` -- a host the operator never
            // typed. A control character would not pin anything here; the parser rejects those
            // on its own.
            "scoreboard\tlocal",
            "hunter2%40host:x", // not a port
        ] {
            assert!(!is_bare_authority(bad), "should refuse {bad:?}");
        }
    }

    /// An address whose authority the parser does not read as typed is refused at entry, so no
    /// client is built for it, nothing is published for it, and no log line has to redact it.
    /// It could not have worked anyway -- refbox would have called a machine nobody named.
    #[test]
    fn an_address_whose_host_is_not_a_host_is_refused() {
        for bad in [
            r"https://scoreboard.local\scorekeeper:hunter2@pool/api/1234-A",
            r"https://scoreboard.local\scorekeeper:hunter2%40pool/api/1234-A",
            // A `?` or `#` where the path should start: the base URL would keep it and every
            // request would be swallowed into a query string.
            "https://scoreboard.local?x=1/api/1234-A",
            "https://scoreboard.local#f/api/1234-A",
            // A trailing backslash: the parser tidies it away, then every request gains a doubled
            // path segment against a site that was committed as working.
            r"https://scoreboard.local\/api/1234-A",
            // A stray space, the classic paste artifact. Trimmed by the parser, so the address is
            // committed and then fails every call.
            "https://scoreboard.local /api/1234-A",
            "https://scorekeeper:hunter2\u{FF20}scoreboard.local:8099/api/1234-A",
            "https://scorekeeper:hunter2%40scoreboard.local/api/1234-A",
        ] {
            assert_eq!(
                parse_custom_site(bad),
                Err(CustomSiteError::MalformedHost),
                "should refuse {bad}"
            );
        }
    }

    /// The addresses that gave rise to that check must still be accepted when they are simply
    /// ordinary -- credentials included, since an operator may genuinely need them.
    #[test]
    fn an_ordinary_address_is_still_accepted() {
        for good in [
            "https://scoreboard.local:8099/api/1234-A",
            "https://scorekeeper:hunter2@scoreboard.local:8099/api/1234-A",
            "http://192.168.1.5/api/1234-A",
            "https://scoreboard.local/prefix/api/1234-A",
            // Shapes the check must not refuse by accident.
            "https://scoreboard.local:443/api/1234-A",
            "https://[::1]:8099/api/1234-A",
            "https://Scoreboard.Local/api/1234-A",
            "https://scoreboard.local./api/1234-A",
            // An internationalised host, its punycode form, and the older `/api/events/` shape
            // the integration doc still documents. Tightening the check later must not strand
            // any of these mid-tournament.
            "https://m\u{FC}nchen.example/api/1234-A",
            "https://xn--mnchen-3ya.example/api/1234-A",
            "https://scoreboard.local/api/events/1234-A",
        ] {
            assert!(parse_custom_site(good).is_ok(), "should accept {good}");
        }
    }

    /// A path segment that merely looks like credentials is a path segment, and is left alone.
    ///
    /// `https://host/scorekeeper:hunter2@pool/api/1234-A` is accepted and printed in full. Every
    /// URL parser agrees this is a path -- there is no disagreement to protect anyone from, and
    /// the operator asked for that address. It is the same case as a token in a query string,
    /// which `strip_credentials` also leaves alone and says so. The backslash spelling is refused
    /// not because of how it looks but because the readings *differ*: an RFC 3986 parser calls
    /// that same text a credential and resolves a different host.
    #[test]
    fn a_path_that_looks_like_credentials_is_still_a_path() {
        let addr = "https://scoreboard.local/scorekeeper:hunter2@pool/api/1234-A";
        assert!(parse_custom_site(addr).is_ok());
        assert_eq!(redacted(addr), addr);
    }

    /// An `@` after the host is not a credential and must not be treated as one. Checking the
    /// redacted *result* for an `@` used to withhold this perfectly good address.
    #[test]
    fn an_at_sign_after_the_host_is_not_treated_as_a_secret() {
        for fine in [
            "https://scoreboard.local/api/1234-A?who=a@b.com",
            "https://scoreboard.local?q=a@b",
            "https://scoreboard.local#f@g",
        ] {
            assert_ne!(redacted(fine), WITHHELD_ADDRESS, "wrongly withheld {fine}");
        }
    }

    #[test]
    fn an_address_that_cannot_be_cleaned_is_never_echoed() {
        for addr in UNCLEANABLE {
            let shown = redacted(addr);
            assert_eq!(shown, WITHHELD_ADDRESS, "for {addr}");
            assert!(!shown.contains("hunter2"), "leaked: {shown}");
        }
    }

    /// The same values through the type, since that is what the log sites actually format.
    #[test]
    fn a_site_address_that_cannot_be_cleaned_is_never_echoed() {
        for addr in UNCLEANABLE {
            let addr = SiteAddress::from((*addr).to_string());
            assert!(!format!("{addr}").contains("hunter2"), "leaked via Display");
            assert!(!format!("{addr:?}").contains("hunter2"), "leaked via Debug");
        }
    }

    #[test]
    fn a_site_address_never_displays_its_credentials() {
        let addr = SiteAddress::from(LEAKY.to_string());
        assert_eq!(
            format!("{addr}"),
            "https://scoreboard.local:8099/api/1234-A"
        );
    }

    /// `{:?}` must not be a way around the redaction. It keeps the quotes that made a
    /// space-padded address visible back when this field was a `String`...
    #[test]
    fn a_site_address_never_debugs_its_credentials() {
        let addr = SiteAddress::from(LEAKY.to_string());
        assert_eq!(
            format!("{addr:?}"),
            "\"https://scoreboard.local:8099/api/1234-A\""
        );
    }

    /// ...but not around the placeholder, which is a sentence about the address and would read as
    /// one that was typed.
    #[test]
    fn the_withheld_placeholder_is_not_dressed_up_as_an_address() {
        let addr = SiteAddress::from("https://user:hunter2@".to_string());
        assert_eq!(format!("{addr:?}"), WITHHELD_ADDRESS);
    }

    /// A log line is one line. An address that parses can still hold a raw newline, which would
    /// split the entry in two and let the rest be chosen by whoever typed the address.
    #[test]
    fn a_redacted_address_can_never_break_a_log_line() {
        let addr = "https://scoreboard.local/a\nb\rc\td/api/1234-A";
        // Pinned exactly: asserting only the absence of the control characters would stay green
        // if the address were withheld instead, which proves nothing about a shown one.
        assert_eq!(redacted(addr), "https://scoreboard.local/abcd/api/1234-A");
    }

    /// The real value stays reachable for the callers that consume it rather than print it.
    #[test]
    fn the_raw_address_is_still_available_to_expose() {
        let addr = SiteAddress::from(LEAKY.to_string());
        assert_eq!(addr.expose(), LEAKY);
    }

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
    // operator typed into it reaches every client on the pool LAN. Every expectation below is the
    // parser's normalised form: that is what gets published, deliberately.

    #[test]
    fn an_ordinary_address_survives_normalisation_unchanged() {
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
    fn http_is_handled_the_same_as_https() {
        assert_eq!(
            strip_credentials("http://user:pw@scoreboard.local:8099").as_deref(),
            Some("http://scoreboard.local:8099")
        );
    }

    /// A password containing a raw `@` splits on the last one, as URL parsing does. Testing the
    /// percent-encoded form instead would not exercise the split at all.
    #[test]
    fn the_last_at_sign_in_the_authority_is_the_delimiter() {
        assert_eq!(
            strip_credentials("https://user:p@ss@scoreboard.local").as_deref(),
            Some("https://scoreboard.local")
        );
    }

    /// The load-bearing case: credentials to remove AND a legal `@` in the path to keep.
    #[test]
    fn credentials_go_but_a_later_at_sign_in_the_path_stays() {
        assert_eq!(
            strip_credentials("https://user:pw@scoreboard.local/team@pool").as_deref(),
            Some("https://scoreboard.local/team@pool")
        );
    }

    // ---- The reported host must equal the host requests go to ----

    #[test]
    fn a_path_containing_an_at_sign_does_not_change_the_host() {
        assert_eq!(
            strip_credentials("https://scoreboard.local/team@pool").as_deref(),
            Some("https://scoreboard.local/team@pool")
        );
    }

    #[test]
    fn a_query_string_containing_an_at_sign_does_not_change_the_host() {
        assert_eq!(
            strip_credentials("https://scoreboard.local?q=a@b").as_deref(),
            Some("https://scoreboard.local/?q=a@b")
        );
    }

    #[test]
    fn a_fragment_containing_an_at_sign_does_not_change_the_host() {
        assert_eq!(
            strip_credentials("https://scoreboard.local#f@g").as_deref(),
            Some("https://scoreboard.local/#f@g")
        );
    }

    /// A backslash ends the authority under WHATWG, which is how reqwest reads it, but RFC 3986
    /// parsers (Python, Go, Java) read `host\team` as userinfo and `pool` as the host. Publishing
    /// the raw string would therefore send consumers to a different machine than refbox calls.
    ///
    /// This used to report the normalised form for that reason. It now reports nothing, because
    /// normalising answers only half of it: the two readings also disagree about whether
    /// `team@pool` is a *credential*, so under the RFC reading the published address carries the
    /// operator's password to every device on the pool LAN. Where the readings disagree the feed
    /// does what it does for any address it cannot vouch for and publishes nothing -- a consumer
    /// takes any string it is handed as a real address, and no string here is right under both
    /// readings. `parse_custom_site` now refuses such an address outright, so an operator is told
    /// rather than left with a site that quietly talks to the wrong machine.
    #[test]
    fn a_backslash_address_is_reported_as_nothing_at_all() {
        assert_eq!(
            strip_credentials(r"https://scoreboard.local\team@pool"),
            None
        );
    }

    /// Adding credentials to a site must not change anything else about the reported address.
    ///
    /// Both sides are asserted against a literal, not against each other: comparing
    /// `strip_credentials(a)` with `strip_credentials(b)` passes on `None == None`, so it would
    /// stay green if the function returned nothing at all.
    #[test]
    fn adding_credentials_does_not_change_the_rest_of_the_address() {
        assert_eq!(
            strip_credentials("https://Scoreboard.Local:443/x").as_deref(),
            Some("https://scoreboard.local/x")
        );
        assert_eq!(
            strip_credentials("https://user:pw@Scoreboard.Local:443/x").as_deref(),
            Some("https://scoreboard.local/x")
        );
    }

    /// The three normalisations the public field doc promises consumers. Asserted explicitly,
    /// because a change that stopped doing any of them would alter every consumer's resolved
    /// address while leaving the rest of this suite green.
    #[test]
    fn the_documented_normalisations_are_applied() {
        // Host lower-cased.
        assert_eq!(
            strip_credentials("https://Scoreboard.LOCAL/x").as_deref(),
            Some("https://scoreboard.local/x")
        );
        // Default port dropped.
        assert_eq!(
            strip_credentials("https://scoreboard.local:443/x").as_deref(),
            Some("https://scoreboard.local/x")
        );
        // International name punycoded.
        assert_eq!(
            strip_credentials("https://schöne.example/api").as_deref(),
            Some("https://xn--schne-lua.example/api")
        );
    }

    /// An empty userinfo is still a credential-shaped authority and must not survive.
    #[test]
    fn an_empty_userinfo_is_removed() {
        assert_eq!(
            strip_credentials("https://@scoreboard.local").as_deref(),
            Some("https://scoreboard.local")
        );
        assert_eq!(
            strip_credentials("https://:@scoreboard.local").as_deref(),
            Some("https://scoreboard.local")
        );
    }

    /// The trailing slash `Url` adds to an empty path is dropped, but a slash anywhere else --
    /// here inside a query value -- must survive.
    #[test]
    fn a_trailing_slash_inside_a_query_value_is_not_eaten() {
        assert_eq!(
            strip_credentials("https://user:pw@scoreboard.local/?a=1/").as_deref(),
            Some("https://scoreboard.local/?a=1/")
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

    /// `Url` parses far more than http(s). None of these is an address the refbox could talk to,
    /// and publishing one as the authoritative portal address would be worse than saying nothing.
    #[test]
    fn a_scheme_the_refbox_cannot_talk_to_reports_nothing() {
        assert_eq!(strip_credentials("mailto:a@b.com"), None);
        assert_eq!(strip_credentials("file:///tmp/x"), None);
        assert_eq!(strip_credentials("javascript:alert(1)@x"), None);
    }

    /// The override environment variables are validated nowhere else, and `localhost:8080` -- a
    /// natural typo for `http://localhost:8080` -- parses as scheme `localhost`, path `8080`.
    #[test]
    fn an_address_missing_its_scheme_reports_nothing() {
        assert_eq!(strip_credentials("localhost:8080"), None);
    }

    /// The cases above all happen to be "cannot-be-a-base" URLs, which fail when the credentials
    /// are cleared even with no scheme check at all -- so on their own they do not prove the
    /// scheme check does anything. `ftp` and `ws` are the shapes that would otherwise slip
    /// through: real base URLs, in WHATWG's special-scheme list, which accept credentials and
    /// would be stripped and published as the refbox's portal address.
    #[test]
    fn a_non_http_scheme_that_would_otherwise_be_accepted_reports_nothing() {
        assert_eq!(strip_credentials("ftp://user:pw@scoreboard.local/x"), None);
        assert_eq!(strip_credentials("ws://scoreboard.local/x"), None);
    }
}
