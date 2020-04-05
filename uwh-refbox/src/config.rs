use log::*;
use serde_derive::{Deserialize, Serialize};
use std::fs::read_to_string;
use std::path::Path;

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Game {
    pub half_play_duration: u16,
    pub half_time_duration: u16,
    pub team_timeout_duration: u16,
    pub team_timeouts_per_half: u16,
    pub has_overtime: bool,
    pub ot_half_play_duration: u16,
    pub ot_half_time_duration: u16,
    pub pre_overtime_break: u16,
    pub overtime_break_duration: u16,
    pub pre_sudden_death_duration: u16,
    pub sudden_death_allowed: bool,
    pub pre_game_duration: u16,
    pub nominal_break: u16,
    pub minimum_break: u16,
    pub timezone: String,
    pub use_wallclock: bool,
    pub pool: String,
    pub tid: u32,
    pub uwhscores_url: String,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            half_play_duration: 900,
            half_time_duration: 180,
            team_timeout_duration: 60,
            team_timeouts_per_half: 1,
            has_overtime: true,
            ot_half_play_duration: 300,
            ot_half_time_duration: 180,
            pre_overtime_break: 180,
            overtime_break_duration: 60,
            pre_sudden_death_duration: 60,
            sudden_death_allowed: true,
            pre_game_duration: 180,
            nominal_break: 900,
            minimum_break: 240,
            timezone: "mst".to_string(),
            use_wallclock: true,
            pool: "1".to_string(),
            tid: 16,
            uwhscores_url: "http://uwhscores.com/api/v1/".to_string(),
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
        r#"half_play_duration = 900
           half_time_duration = 180
           team_timeout_duration = 60
           has_overtime = true
           ot_half_play_duration = 300
           ot_half_time_duration = 180
           pre_overtime_break = 180
           overtime_break_duration = 60
           pre_sudden_death_duration = 60
           sudden_death_allowed = true
           team_timeouts_per_half = 1
           pre_game_duration = 180
           nominal_break = 900
           minimum_break = 240
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
    fn test_deser_xbee() {
        let hw: XBee = Default::default();
        let deser = toml::from_str(XBEE_STRING);
        assert_eq!(deser, Ok(hw));
    }

    #[test]
    fn test_deser_rs485() {
        let hw: RS485 = Default::default();
        let deser = toml::from_str(RS485_STRING);
        assert_eq!(deser, Ok(hw));
    }

    #[test]
    fn test_deser_game() {
        let hw: Game = Default::default();
        let deser = toml::from_str(GAME_STRING);
        assert_eq!(deser, Ok(hw));
    }

    #[test]
    fn test_deser_config() {
        let hw: Config = Default::default();
        let deser = toml::from_str(&format!(
            "[game]\n{}\n[hardware]\n{}\n[xbee]\n{}\n[rs485]\n{}",
            GAME_STRING, HW_STRING, XBEE_STRING, RS485_STRING
        ));
        assert_eq!(deser, Ok(hw));
    }
}
