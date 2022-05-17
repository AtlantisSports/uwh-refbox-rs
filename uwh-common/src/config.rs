use log::*;
use serde_derive::{Deserialize, Serialize};
use std::{fs::read_to_string, path::Path, time::Duration};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hardware {
    pub screen_x: i32,
    pub screen_y: i32,
    pub has_xbee: bool,
    pub has_rs485: bool,
    pub white_on_right: bool,
}

impl Default for Hardware {
    fn default() -> Self {
        Self {
            screen_x: 1024,
            screen_y: 768,
            has_xbee: false,
            has_rs485: false,
            white_on_right: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct XBee {
    pub port: String,
    pub baud: u32,
    pub clients: Vec<String>,
    pub ch: String,
    pub id: String,
}

impl Default for XBee {
    fn default() -> Self {
        Self {
            #[cfg(target_os = "linux")]
            port: "/dev/ttyUSB0".to_string(),
            #[cfg(not(target_os = "linux"))]
            port: "/dev/tty.usbserial-DN03ZRU8".to_string(),
            baud: 9600,
            clients: vec![],
            ch: "000C".to_string(),
            id: "000D".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RS485 {
    pub port: String,
    pub baud: u32,
}

impl Default for RS485 {
    fn default() -> Self {
        Self {
            #[cfg(target_os = "linux")]
            port: "/dev/ttyUSB0".to_string(),
            #[cfg(not(target_os = "linux"))]
            port: "/dev/tty.usbserial-DN03ZRU8".to_string(),
            baud: 115_200,
        }
    }
}

// Due to requirements of the TOML language, items stored as tables in TOML (like `Duration`s) need
// to be after items that are not stored as tables (`u16`, `u32`, `bool`, `String`)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Game {
    pub team_timeouts_per_half: u16,
    pub has_overtime: bool,
    pub sudden_death_allowed: bool,
    pub timezone: String,
    pub use_wallclock: bool,
    pub pool: String,
    pub tid: u32,
    pub uwhscores_url: String,
    pub half_play_duration: Duration,
    pub half_time_duration: Duration,
    pub team_timeout_duration: Duration,
    pub ot_half_play_duration: Duration,
    pub ot_half_time_duration: Duration,
    pub pre_overtime_break: Duration,
    pub overtime_break_duration: Duration,
    pub pre_sudden_death_duration: Duration,
    pub post_game_duration: Duration,
    pub nominal_break: Duration,
    pub minimum_break: Duration,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            team_timeouts_per_half: 1,
            has_overtime: true,
            sudden_death_allowed: true,
            timezone: "mst".to_string(),
            use_wallclock: true,
            pool: "1".to_string(),
            tid: 16,
            uwhscores_url: "http://uwhscores.com/api/v1/".to_string(),
            half_play_duration: Duration::from_secs(900),
            half_time_duration: Duration::from_secs(180),
            team_timeout_duration: Duration::from_secs(60),
            ot_half_play_duration: Duration::from_secs(300),
            ot_half_time_duration: Duration::from_secs(180),
            pre_overtime_break: Duration::from_secs(180),
            overtime_break_duration: Duration::from_secs(60),
            pre_sudden_death_duration: Duration::from_secs(60),
            post_game_duration: Duration::from_secs(60),
            nominal_break: Duration::from_secs(900),
            minimum_break: Duration::from_secs(240),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub game: Game,
    pub hardware: Hardware,
    pub xbee: XBee,
    pub rs485: RS485,
}

impl Config {
    pub fn new_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let config_file = match read_to_string(path) {
            Ok(f) => f,
            Err(e) => {
                error!("Failed to read config file: {}", e);
                return Err(Box::new(e));
            }
        };

        match toml::from_str(&config_file) {
            Ok(c) => Ok(c),
            Err(e) => {
                error!("Failed to parse config file: {}", e);
                Err(Box::new(e))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use indoc::indoc;

    const HW_STRING: &str = indoc!(
        r#"screen_x = 1024
           screen_y = 768
           has_xbee = false
           has_rs485 = false
           white_on_right = false"#
    );

    #[cfg(target_os = "linux")]
    const XBEE_STRING: &str = indoc!(
        r#"port = "/dev/ttyUSB0"
           baud = 9600
           clients = []
           ch = "000C"
           id = "000D""#
    );

    #[cfg(not(target_os = "linux"))]
    const XBEE_STRING: &str = indoc!(
        r#"port = "/dev/tty.usbserial-DN03ZRU8"
           baud = 9600
           clients = []
           ch = "000C"
           id = "000D""#
    );

    #[cfg(target_os = "linux")]
    const RS485_STRING: &str = indoc!(
        r#"port = "/dev/ttyUSB0"
           baud = 115200"#
    );

    #[cfg(not(target_os = "linux"))]
    const RS485_STRING: &str = indoc!(
        r#"port = "/dev/tty.usbserial-DN03ZRU8"
           baud = 115200"#
    );

    const GAME_STRING: &str = indoc!(
        r#"half_play_duration = { secs = 900, nanos = 0 }
           half_time_duration = { secs = 180, nanos = 0 }
           team_timeout_duration = { secs = 60, nanos = 0 }
           has_overtime = true
           ot_half_play_duration = { secs = 300, nanos = 0 }
           ot_half_time_duration = { secs = 180, nanos = 0 }
           pre_overtime_break = { secs = 180, nanos = 0 }
           overtime_break_duration = { secs = 60, nanos = 0 }
           pre_sudden_death_duration = { secs = 60, nanos = 0 }
           sudden_death_allowed = true
           team_timeouts_per_half = 1
           post_game_duration = { secs = 60, nanos = 0 }
           nominal_break = { secs = 900, nanos = 0 }
           minimum_break = { secs = 240, nanos = 0 }
           timezone = "mst"
           use_wallclock = true
           pool = "1"
           tid = 16
           uwhscores_url = "http://uwhscores.com/api/v1/""#
    );

    #[test]
    fn test_deser_hardware() {
        let hw: Hardware = Default::default();
        let deser = toml::from_str(HW_STRING);
        assert_eq!(deser, Ok(hw));
    }

    #[test]
    fn test_ser_hardware() {
        let hw: Hardware = Default::default();
        toml::to_string(&hw).unwrap();
    }

    #[test]
    fn test_deser_xbee() {
        let xb: XBee = Default::default();
        let deser = toml::from_str(XBEE_STRING);
        assert_eq!(deser, Ok(xb));
    }

    #[test]
    fn test_ser_xbee() {
        let xb: XBee = Default::default();
        toml::to_string(&xb).unwrap();
    }

    #[test]
    fn test_deser_rs485() {
        let rs: RS485 = Default::default();
        let deser = toml::from_str(RS485_STRING);
        assert_eq!(deser, Ok(rs));
    }

    #[test]
    fn test_ser_rs485() {
        let rs: RS485 = Default::default();
        toml::to_string(&rs).unwrap();
    }

    #[test]
    fn test_deser_game() {
        let gm: Game = Default::default();
        let deser = toml::from_str(GAME_STRING);
        assert_eq!(deser, Ok(gm));
    }

    #[test]
    fn test_ser_game() {
        let gm: Game = Default::default();
        toml::to_string(&gm).unwrap();
    }

    #[test]
    fn test_deser_config() {
        let config: Config = Default::default();
        let deser = toml::from_str(&format!(
            "[game]\n{}\n[hardware]\n{}\n[xbee]\n{}\n[rs485]\n{}",
            GAME_STRING, HW_STRING, XBEE_STRING, RS485_STRING
        ));
        assert_eq!(deser, Ok(config));
    }

    #[test]
    fn test_ser_config() {
        let config: Config = Default::default();
        toml::to_string(&config).unwrap();
    }
}
