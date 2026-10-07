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
    /// PIN needed to use the control page and the Companion links. Empty until set on first use.
    pub pin: String,
    /// While on, the Live tab only shows what it would do; nothing is sent to YouTube or vMix.
    /// On by default so a fresh install can never touch a live channel by accident.
    pub practice_mode: bool,
    pub courts: Vec<CourtConfig>,
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
            practice_mode: true,
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

impl Config {
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
}
