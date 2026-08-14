//! Which site the tool talks to, and how it authenticates there.
//!
//! Mirrors the refbox's model: the operator picks the built-in portal or a
//! custom site, and a custom site is one address they type. Every decision
//! here is a pure function so it can be tested — `main.rs` keeps only the
//! prompting, which needs a terminal.

/// Which kind of site the tool is talking to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiteKind {
    /// The built-in UWH/UWR portal, chosen from the environment menu.
    Portal,
    /// An address the operator typed, authenticated with a pasted access key.
    Custom,
}

/// A resolved site: where to connect, and what is expected of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteTarget {
    pub kind: SiteKind,
    pub base_url: String,
    /// Whether a valid TLS certificate is required. Taken from the address's
    /// own scheme, so a local or on-LAN `http://` site still works.
    pub require_https: bool,
}

/// The portal address for a menu pair. The menu strings come straight from the
/// prompts in `main.rs`; anything else is a programming error, so this falls
/// back to production hockey rather than panicking on a typo.
pub fn portal_default_url(environment: &str, sport: &str) -> &'static str {
    match (environment, sport) {
        ("Local", _) => "http://localhost:9000",
        ("Development", "Underwater Rugby") => "https://api.dev.uwrportal.com",
        ("Development", _) => "https://api.dev.uwhportal.com",
        (_, "Underwater Rugby") => "https://api.uwrportal.com",
        _ => "https://api.uwhportal.com",
    }
}

/// Turn an address the operator typed into a target, or explain why it cannot
/// be used. Surrounding spaces and a trailing `/` are cleaned up silently —
/// both are ordinary copy-paste artefacts, not mistakes worth a message.
pub fn custom_target(typed_url: &str) -> Result<SiteTarget, String> {
    let url = typed_url.trim().trim_end_matches('/');
    if url.is_empty() {
        return Err("No address entered. Type the full web address of your site.".to_string());
    }
    let require_https = if url.starts_with("https://") {
        true
    } else if url.starts_with("http://") {
        false
    } else {
        return Err(format!(
            "\"{url}\" does not look like a web address. It needs to start with https:// \
             (or http:// for a site on your own network)."
        ));
    };
    Ok(SiteTarget {
        kind: SiteKind::Custom,
        base_url: url.to_string(),
        require_https,
    })
}

/// The environment variable that can replace the menu-selected address.
pub fn override_var_name(sport: &str) -> &'static str {
    match sport {
        "Underwater Rugby" => "UWR_PORTAL_URL_OVERRIDE",
        _ => "UWH_PORTAL_URL_OVERRIDE",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_menu_pair_resolves_to_its_address() {
        assert_eq!(
            portal_default_url("Production", "Underwater Hockey"),
            "https://api.uwhportal.com"
        );
        assert_eq!(
            portal_default_url("Production", "Underwater Rugby"),
            "https://api.uwrportal.com"
        );
        assert_eq!(
            portal_default_url("Development", "Underwater Hockey"),
            "https://api.dev.uwhportal.com"
        );
        assert_eq!(
            portal_default_url("Development", "Underwater Rugby"),
            "https://api.dev.uwrportal.com"
        );
        assert_eq!(
            portal_default_url("Local", "Underwater Hockey"),
            "http://localhost:9000"
        );
        assert_eq!(
            portal_default_url("Local", "Underwater Rugby"),
            "http://localhost:9000"
        );
    }

    #[test]
    fn a_typed_https_address_requires_a_certificate() {
        let t = custom_target("  https://scores.example.org  ").unwrap();
        assert_eq!(t.kind, SiteKind::Custom);
        assert_eq!(
            t.base_url, "https://scores.example.org",
            "surrounding spaces are trimmed"
        );
        assert!(t.require_https);
    }

    #[test]
    fn a_typed_http_address_does_not_require_a_certificate() {
        // A club server on the local network is a real case; refusing it
        // would make the custom option useless there.
        let t = custom_target("http://192.168.1.50:9000").unwrap();
        assert!(!t.require_https);
    }

    #[test]
    fn an_address_without_a_scheme_is_refused_in_plain_words() {
        let err = custom_target("scores.example.org").unwrap_err();
        assert!(
            err.contains("http://") && err.contains("https://"),
            "the message must tell the operator what to type; got: {err}"
        );
    }

    #[test]
    fn a_blank_address_is_refused() {
        assert!(custom_target("   ").is_err());
    }

    #[test]
    fn a_trailing_slash_is_dropped_so_paths_do_not_double_up() {
        let t = custom_target("https://scores.example.org/").unwrap();
        assert_eq!(t.base_url, "https://scores.example.org");
    }

    #[test]
    fn the_override_variable_follows_the_sport() {
        assert_eq!(
            override_var_name("Underwater Hockey"),
            "UWH_PORTAL_URL_OVERRIDE"
        );
        assert_eq!(
            override_var_name("Underwater Rugby"),
            "UWR_PORTAL_URL_OVERRIDE"
        );
    }
}
