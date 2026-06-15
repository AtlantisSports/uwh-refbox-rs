#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// Parse `X.Y.Z` (a leading `v` is tolerated). Returns None on anything else.
    pub fn parse(s: &str) -> Option<Version> {
        let s = s.strip_prefix('v').unwrap_or(s);
        let mut it = s.split('.');
        let major = it.next()?.parse().ok()?;
        let minor = it.next()?.parse().ok()?;
        let patch = it.next()?.parse().ok()?;
        if it.next().is_some() {
            return None;
        }
        Some(Version {
            major,
            minor,
            patch,
        })
    }

    pub fn cmp_to(&self, other: &Version) -> std::cmp::Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_orders() {
        use std::cmp::Ordering::*;
        assert_eq!(
            Version::parse("0.4.2").unwrap(),
            Version {
                major: 0,
                minor: 4,
                patch: 2
            }
        );
        assert_eq!(
            Version::parse("v0.4.3")
                .unwrap()
                .cmp_to(&Version::parse("0.4.2").unwrap()),
            Greater
        );
        assert_eq!(
            Version::parse("0.4.2")
                .unwrap()
                .cmp_to(&Version::parse("0.4.2").unwrap()),
            Equal
        );
        assert_eq!(
            Version::parse("0.4.2")
                .unwrap()
                .cmp_to(&Version::parse("0.4.10").unwrap()),
            Less
        );
        assert!(Version::parse("garbage").is_none());
        assert!(Version::parse("0.4").is_none());
    }
}
