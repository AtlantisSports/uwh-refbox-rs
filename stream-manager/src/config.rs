use directories::UserDirs;
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
};

/// Folder that holds the config file and the Google sign-in files. Kept outside the project
/// folder so secrets can never be committed by accident.
pub fn default_config_dir() -> PathBuf {
    UserDirs::new()
        .and_then(|dirs| dirs.document_dir().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("stream-manager")
}

pub fn default_config_path() -> PathBuf {
    default_config_dir().join("config.toml")
}

pub const LIVE_PORTAL_URL: &str = "https://api.uwhportal.com";
pub const DEV_PORTAL_URL: &str = "https://api.dev.uwhportal.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Portal API base URL, e.g. `https://api.dev.uwhportal.com` for testing.
    pub portal_url: String,
    /// The event's slug as it appears in its portal URL, e.g. `au-2026-henks-kings-cup`.
    pub event_slug: String,
    /// When the countdown to the next game reaches this many seconds, switch to its video.
    /// 195 s (3:15) is just before the overlay starts showing the next game's rosters.
    pub switch_lead_secs: u32,
    /// The overlay shows the next game's rosters while the countdown is between these two
    /// values (inclusive). No automatic switch happens inside this window. These must match
    /// the overlay (`overlay/src/main.rs`, `BetweenGames` page selection).
    pub roster_start_secs: u32,
    pub roster_end_secs: u32,
    /// Privacy of the videos and playlists we create: `unlisted`, `public` or `private`.
    pub privacy: String,
    /// Google sign-in file downloaded from the Cloud console. Relative paths are relative to
    /// the config file's folder.
    pub client_secret_file: PathBuf,
    /// Port of the control page (`http://<this computer>:<port>`).
    pub web_port: u16,
    /// PIN for signing in on the control page. Empty until set on first use.
    pub pin: String,
    /// Off (the default): the control page answers only this mini PC itself. On: it also
    /// answers the devices in `allowed_devices`. Either applies after a restart.
    #[serde(default)]
    pub allow_other_devices: bool,
    /// The network addresses of the other devices allowed to use the control page.
    #[serde(default)]
    pub allowed_devices: Vec<IpAddr>,
    /// Secret in the Stream Deck (Companion) links, `?key=…`: 64 hex characters, created on first
    /// start. Never logged, and only shown on this mini PC itself.
    #[serde(default)]
    pub button_key: String,
    /// While on, the Live tab only shows what it would do; nothing is sent to YouTube or vMix.
    /// On by default so a fresh install can never touch a live channel by accident.
    pub practice_mode: bool,
    /// YouTube's daily allowance for the Google Cloud project, in units (10,000 unless Google
    /// granted more). Every court's Stream Manager shares it.
    #[serde(default = "default_quota_daily_limit")]
    pub quota_daily_limit: u32,
    /// This program's share of the daily allowance, in percent: 100 with one court, 50 each with
    /// two. A program that runs more than one court uses this share for all of them.
    #[serde(default = "default_quota_share_percent")]
    pub quota_share_percent: u8,
    /// Bitfocus Companion's address (`ip:port`, e.g. `127.0.0.1:8000`), for live status on the
    /// Stream Deck buttons (ADR 026 §4). Empty turns the feature off.
    #[serde(default)]
    pub companion_address: String,
    pub courts: Vec<CourtConfig>,
}

fn default_quota_daily_limit() -> u32 {
    10_000
}

fn default_quota_share_percent() -> u8 {
    50
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StreamMode {
    /// Two stream keys taking turns: no gap at a switch (ADR 026 §5).
    #[default]
    TwoKeys,
    /// One stream key all day: a few seconds' gap before kickoff.
    OneKey,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CourtConfig {
    /// Court name exactly as the portal schedule uses it (e.g. `1`).
    pub name: String,
    pub refbox_ip: IpAddr,
    /// The refbox's JSON snapshot port (the same one the overlay uses).
    pub refbox_port: u16,
    /// Names of the court's two YouTube stream keys, as shown in YouTube Studio. Games take
    /// turns between them (see ADR 026, §5). Defaults: "Court <name> - A" / "Court <name> - B".
    #[serde(default)]
    pub stream_a: Option<String>,
    #[serde(default)]
    pub stream_b: Option<String>,
    /// This court's vMix Web Controller (`ip:port`). Stream key A is vMix streaming
    /// destination 1 and stream key B is destination 2.
    #[serde(default = "default_vmix_address")]
    pub vmix_address: String,
    /// Two stream keys taking turns (default), or one key all day (ADR 026, §5).
    #[serde(default)]
    pub stream_mode: StreamMode,
}

fn default_vmix_address() -> String {
    "127.0.0.1:8088".to_string()
}

/// vMix streaming destination (as numbered in vMix) for stream key A (index 0) or B (index 1).
pub fn vmix_destination(stream_index: usize) -> u8 {
    if stream_index == 0 { 1 } else { 2 }
}

impl CourtConfig {
    /// Which stream key (0 = A, 1 = B) the court's game at `position` in the day's order uses.
    pub fn stream_for_position(&self, position: usize) -> usize {
        match self.stream_mode {
            StreamMode::TwoKeys => position % 2,
            StreamMode::OneKey => 0,
        }
    }

    pub fn stream_names(&self) -> [String; 2] {
        [
            self.stream_a
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| format!("Court {} - A", self.name)),
            self.stream_b
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| format!("Court {} - B", self.name)),
        ]
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            portal_url: DEV_PORTAL_URL.to_string(),
            event_slug: String::new(),
            switch_lead_secs: 195,
            roster_start_secs: 181,
            roster_end_secs: 30,
            privacy: "unlisted".to_string(),
            client_secret_file: PathBuf::from("client_secret.json"),
            web_port: 8090,
            pin: String::new(),
            allow_other_devices: false,
            allowed_devices: Vec::new(),
            button_key: String::new(),
            practice_mode: true,
            quota_daily_limit: default_quota_daily_limit(),
            quota_share_percent: default_quota_share_percent(),
            companion_address: String::new(),
            courts: vec![CourtConfig {
                name: "1".to_string(),
                refbox_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
                refbox_port: 8000,
                stream_a: None,
                stream_b: None,
                vmix_address: default_vmix_address(),
                stream_mode: StreamMode::TwoKeys,
            }],
        }
    }
}

/// The event's slug goes into a file name (`state-<slug>.json`), so only letters, digits and
/// dashes are accepted, as in every portal event address.
pub fn check_event_slug(slug: &str) -> Result<(), String> {
    if slug.is_empty() {
        return Err("Choose an event first (Settings tab)".into());
    }
    if !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(format!(
            "The event {slug:?} isn't a portal event name: it may only use letters, digits and \
             dashes (e.g. au-2026-henks-kings-cup). Choose the event again in Settings."
        ));
    }
    Ok(())
}

impl Config {
    /// `validate`, plus the checks that only apply to settings being saved over `current` (the
    /// settings in use): these never stop an existing settings file from loading.
    pub fn validate_for_save(&self, current: &Config) -> Result<(), String> {
        self.validate()?;
        // Only the two portals are offered; anything else could send the schedule requests (and
        // the event's titles) somewhere unexpected. Checked only when the portal changes, so a
        // hand-edited settings file with another portal can still have its first PIN set.
        let portal = self
            .portal_url
            .strip_suffix('/')
            .unwrap_or(&self.portal_url);
        if self.portal_url != current.portal_url
            && portal != LIVE_PORTAL_URL
            && portal != DEV_PORTAL_URL
        {
            return Err("Choose the live or dev portal".into());
        }
        // The event may still be unset; once chosen, it must be usable in a file name.
        if !self.event_slug.is_empty() {
            check_event_slug(&self.event_slug)?;
        }
        if self.quota_daily_limit == 0 {
            return Err("YouTube's daily allowance must be at least 1 unit".into());
        }
        // Two courts sharing Companion variables would overwrite each other's buttons; this only
        // matters while the Companion address is set.
        if !self.companion_address.trim().is_empty() {
            for (i, court) in self.courts.iter().enumerate() {
                let prefix = crate::companion::variable_prefix(&court.name);
                if let Some(other) = self.courts[..i]
                    .iter()
                    .find(|c| crate::companion::variable_prefix(&c.name) == prefix)
                {
                    return Err(format!(
                        "Courts \"{}\" and \"{}\" would share the same Stream Deck status names \
                         ({prefix}_…); rename one",
                        other.name, court.name
                    ));
                }
            }
        }
        Ok(())
    }

    /// Checks everything that doesn't need the network. The event may still be unset.
    pub fn validate(&self) -> Result<(), String> {
        if self.switch_lead_secs <= self.roster_start_secs {
            return Err("The switch time must be earlier than the start of the rosters".into());
        }
        if self.roster_end_secs > self.roster_start_secs {
            return Err("The rosters must end after they start".into());
        }
        // A typo here must never quietly publish something; only accept YouTube's exact values.
        if !["unlisted", "private", "public"].contains(&self.privacy.as_str()) {
            return Err(format!(
                "Privacy must be \"unlisted\", \"private\" or \"public\", not {:?}",
                self.privacy
            ));
        }
        if !(1..=100).contains(&self.quota_share_percent) {
            return Err("This program's share of the YouTube allowance must be 1 to 100%".into());
        }
        if self.courts.is_empty() {
            return Err("Add at least one court".into());
        }
        for (i, court) in self.courts.iter().enumerate() {
            if court.name.trim().is_empty() {
                return Err("Every court needs a name".into());
            }
            if court.vmix_address.trim().is_empty() {
                return Err(format!("Court \"{}\" needs a vMix address", court.name));
            }
            if self.courts[..i].iter().any(|c| c.name == court.name) {
                return Err(format!("Court \"{}\" is listed twice", court.name));
            }
        }
        if !self.pin.is_empty()
            && (self.pin.len() < 4 || !self.pin.chars().all(|c| c.is_ascii_digit()))
        {
            return Err("The PIN must be at least 4 digits".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_valid_and_typos_are_rejected() {
        assert_eq!(Config::default().validate(), Ok(()));

        let c = Config {
            privacy: "Unlisted ".into(),
            ..Default::default()
        };
        assert!(c.validate().is_err());

        let mut c = Config::default();
        c.courts.push(c.courts[0].clone());
        assert!(c.validate().unwrap_err().contains("twice"));

        let mut c = Config {
            pin: "12a4".into(),
            ..Default::default()
        };
        assert!(c.validate().is_err());
        c.pin = "1234".into();
        assert_eq!(c.validate(), Ok(()));
    }

    #[test]
    fn config_without_allowance_settings_loads_with_10000_and_half() {
        let config: Config = serde_json::from_str(r#"{ "event_slug": "cup" }"#).unwrap();
        assert_eq!(config.quota_daily_limit, 10_000);
        assert_eq!(config.quota_share_percent, 50);
        assert_eq!(config.event_slug, "cup");
    }

    #[test]
    fn allowance_share_must_be_1_to_100_percent() {
        for (percent, ok) in [(0, false), (1, true), (50, true), (100, true), (101, false)] {
            let c = Config {
                quota_share_percent: percent,
                ..Default::default()
            };
            assert_eq!(c.validate().is_ok(), ok, "{percent}%");
        }
    }

    #[test]
    fn courts_sharing_stream_deck_names_are_rejected_on_save_only_with_companion_on() {
        let mut c = Config::default();
        let mut second = c.courts[0].clone();
        second.name = "Court 1".into();
        c.courts.push(second);
        // A settings file like this still loads, and saves while Companion is off.
        assert_eq!(c.validate(), Ok(()));
        assert_eq!(c.validate_for_save(&Config::default()), Ok(()));

        c.companion_address = "127.0.0.1:8000".into();
        assert_eq!(c.validate(), Ok(()), "loading must never fail on this");
        let error = c.validate_for_save(&Config::default()).unwrap_err();
        assert!(
            error.contains("\"1\"") && error.contains("\"Court 1\""),
            "{error}"
        );
        assert!(error.contains("sm_court_1"), "{error}");
        c.courts[1].name = "2".into();
        assert_eq!(c.validate_for_save(&Config::default()), Ok(()));
    }

    #[test]
    fn config_without_companion_address_loads_with_it_off() {
        let config: Config = serde_json::from_str(r#"{ "event_slug": "cup" }"#).unwrap();
        assert_eq!(config.companion_address, "");
    }

    #[test]
    fn court_without_stream_mode_loads_as_two_keys() {
        let court: CourtConfig = serde_json::from_str(
            r#"{ "name": "1", "refbox_ip": "127.0.0.1", "refbox_port": 8000 }"#,
        )
        .unwrap();
        assert_eq!(court.stream_mode, StreamMode::TwoKeys);
    }

    #[test]
    fn one_key_mode_round_trips() {
        let mut court = Config::default().courts.remove(0);
        court.stream_mode = StreamMode::OneKey;
        let text = serde_json::to_string(&court).unwrap();
        assert!(text.contains(r#""stream_mode":"one-key""#), "{text}");
        let back: CourtConfig = serde_json::from_str(&text).unwrap();
        assert_eq!(back, court);
    }

    #[test]
    fn one_key_mode_puts_every_game_on_stream_a() {
        let mut court = Config::default().courts.remove(0);
        let two_keys: Vec<usize> = (0..5).map(|i| court.stream_for_position(i)).collect();
        assert_eq!(two_keys, [0, 1, 0, 1, 0]);
        court.stream_mode = StreamMode::OneKey;
        let one_key: Vec<usize> = (0..5).map(|i| court.stream_for_position(i)).collect();
        assert_eq!(one_key, [0, 0, 0, 0, 0]);
    }

    #[test]
    fn blank_stream_names_fall_back_to_defaults() {
        let mut court = Config::default().courts.remove(0);
        court.stream_a = Some("  ".into());
        court.stream_b = Some("Main B".into());
        assert_eq!(
            court.stream_names(),
            ["Court 1 - A".to_string(), "Main B".to_string()]
        );
    }

    #[test]
    fn an_event_that_isnt_a_plain_portal_name_is_refused_on_save() {
        let mut c = Config::default();
        // Not chosen yet: fine.
        assert_eq!(c.validate_for_save(&Config::default()), Ok(()));
        for good in ["au-2026-henks-kings-cup", "Event2"] {
            c.event_slug = good.into();
            assert_eq!(c.validate_for_save(&Config::default()), Ok(()), "{good}");
        }
        for bad in ["../evil", "a/b", "a\\b", "cup 2026", "cup.json", "café"] {
            c.event_slug = bad.into();
            let error = c.validate_for_save(&Config::default()).unwrap_err();
            assert!(
                error.contains("letters, digits and dashes"),
                "{bad}: {error}"
            );
        }
        assert!(check_event_slug("").is_err());
    }

    #[test]
    fn only_the_live_or_dev_portal_is_saved() {
        let mut c = Config::default();
        for good in [
            LIVE_PORTAL_URL.to_string(),
            DEV_PORTAL_URL.to_string(),
            format!("{LIVE_PORTAL_URL}/"),
            format!("{DEV_PORTAL_URL}/"),
        ] {
            c.portal_url = good.clone();
            assert_eq!(c.validate_for_save(&Config::default()), Ok(()), "{good}");
        }
        for bad in [
            "https://evil.example".to_string(),
            "http://api.uwhportal.com".to_string(),
            format!("{LIVE_PORTAL_URL}//"),
            format!("{LIVE_PORTAL_URL}.evil.example"),
            format!(" {DEV_PORTAL_URL}"),
            String::new(),
        ] {
            c.portal_url = bad.clone();
            assert_eq!(
                c.validate_for_save(&Config::default()),
                Err("Choose the live or dev portal".to_string()),
                "{bad}"
            );
        }
    }

    #[test]
    fn a_hand_edited_portal_is_kept_on_save_but_cannot_be_chosen() {
        let current = Config {
            portal_url: "http://localhost:5000".into(),
            ..Default::default()
        };
        // Setting the first PIN saves everything else unchanged.
        let with_pin = Config {
            pin: "1234".into(),
            ..current.clone()
        };
        assert_eq!(with_pin.validate_for_save(&current), Ok(()));
        let other = Config {
            portal_url: "https://evil.example".into(),
            ..current.clone()
        };
        assert_eq!(
            other.validate_for_save(&current),
            Err("Choose the live or dev portal".to_string())
        );
        let live = Config {
            portal_url: LIVE_PORTAL_URL.into(),
            ..current.clone()
        };
        assert_eq!(live.validate_for_save(&current), Ok(()));
    }

    #[test]
    fn a_daily_allowance_of_zero_is_refused_on_save() {
        let mut c = Config {
            quota_daily_limit: 0,
            ..Default::default()
        };
        assert!(
            c.validate_for_save(&Config::default())
                .unwrap_err()
                .contains("at least 1")
        );
        c.quota_daily_limit = 1;
        assert_eq!(c.validate_for_save(&Config::default()), Ok(()));
    }

    #[test]
    fn config_without_device_settings_loads_as_this_pc_only_with_no_key() {
        let config: Config = serde_json::from_str(r#"{ "event_slug": "cup" }"#).unwrap();
        assert!(!config.allow_other_devices);
        assert!(config.allowed_devices.is_empty());
        assert_eq!(config.button_key, "");
    }
}
