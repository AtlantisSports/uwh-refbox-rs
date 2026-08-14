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
