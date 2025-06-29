use led_panel_sim::DisplayState;
use matrix_drawing::transmitted_data::{Brightness, TransmittedData};
use std::fmt::Write;
use std::fs::OpenOptions;
use std::io::Write as IoWrite;
use uwh_common::{
    bundles::BlackWhiteBundle,
    game_snapshot::{GamePeriod, GameSnapshotNoHeap, TimeoutSnapshot},
};

fn test_to_verilog(
    name: &str,
    bin: [u8; TransmittedData::ENCODED_LEN],
    disp: DisplayState,
    brightness: Brightness,
) -> String {
    let data_str = bin
        .iter()
        .rev()
        .map(|byte| format!("8'h{:02x}", byte))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        r#"        '{{
            name: "{}",
            data: '{{{}}},
            ls_10_ex: {},
            ls_1_ex: {},
            rs_10_ex: {},
            rs_1_ex: {},
            m_10_ex: {},
            m_1_ex: {},
            s_10_ex: {},
            s_1_ex: {},
            white_on_left_ex: 1'b{},
            white_on_right_ex: 1'b{},
            left_to_ind_ex: 1'b{},
            right_to_ind_ex: 1'b{},
            ref_to_ind_ex: 1'b{},
            one_ex: 1'b{},
            slash_ex: 1'b{},
            two_ex: 1'b{},
            overtime_ex: 1'b{},
            sdn_dth_ex: 1'b{},
            colon_ex: 1'b{},
            brightness_ex: 2'b{:02b}
        }}"#,
        name,
        data_str,
        disp.left_score_tens.as_verilog(),
        disp.left_score_ones.as_verilog(),
        disp.right_score_tens.as_verilog(),
        disp.right_score_ones.as_verilog(),
        disp.time_m_tens.as_verilog(),
        disp.time_m_ones.as_verilog(),
        disp.time_s_tens.as_verilog(),
        disp.time_s_ones.as_verilog(),
        disp.white_on_left as u8,
        disp.white_on_right as u8,
        disp.left_to_ind as u8,
        disp.right_to_ind as u8,
        disp.ref_to_ind as u8,
        disp.one as u8,
        disp.slash as u8,
        disp.two as u8,
        disp.overtime as u8,
        disp.sdn_dth as u8,
        disp.colon as u8,
        brightness.to_u8(),
    )
}

struct TestCase {
    name: String,
    transmitted_data: TransmittedData,
}

impl TestCase {
    fn as_verilog(&self) -> String {
        let (disp, brightness) = DisplayState::from_transmitted_data(&self.transmitted_data);
        let bin = self.transmitted_data.encode().unwrap();

        test_to_verilog(&self.name, bin, disp, brightness)
    }
}

fn main() {
    let test_cases = [
        TestCase {
            name: "First Half, T900, B0, W0".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 900,
                    timeout: None,
                    scores: BlackWhiteBundle { black: 0, white: 0 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "Half Time, T123, B1, W2".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::HalfTime,
                    secs_in_period: 123,
                    timeout: None,
                    scores: BlackWhiteBundle { black: 1, white: 2 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "Second Half, T0, B3, W4".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::SecondHalf,
                    secs_in_period: 0,
                    timeout: None,
                    scores: BlackWhiteBundle { black: 3, white: 4 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "Pre Overtime, T32, B5, W6".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::PreOvertime,
                    secs_in_period: 32,
                    timeout: None,
                    scores: BlackWhiteBundle { black: 5, white: 6 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "Overtime First Half, T234, B7, W8".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::OvertimeFirstHalf,
                    secs_in_period: 234,
                    timeout: None,
                    scores: BlackWhiteBundle { black: 7, white: 8 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "Overtime Half Time, T45, B9, W10".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::OvertimeHalfTime,
                    secs_in_period: 45,
                    timeout: None,
                    scores: BlackWhiteBundle {
                        black: 9,
                        white: 10,
                    },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "Overtime Second Half, T456, B11, W12".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::OvertimeSecondHalf,
                    secs_in_period: 456,
                    timeout: None,
                    scores: BlackWhiteBundle {
                        black: 11,
                        white: 12,
                    },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "PreSuddenDeath, T12, B13, W13".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::PreSuddenDeath,
                    secs_in_period: 12,
                    timeout: None,
                    scores: BlackWhiteBundle {
                        black: 13,
                        white: 13,
                    },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "SuddenDeath, T5999, B14, W14".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::SuddenDeath,
                    secs_in_period: 5999,
                    timeout: None,
                    scores: BlackWhiteBundle {
                        black: 14,
                        white: 14,
                    },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "BetweenGames, T99, B15, W14".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::BetweenGames,
                    secs_in_period: 99,
                    timeout: None,
                    scores: BlackWhiteBundle {
                        black: 15,
                        white: 14,
                    },
                    penalties: Default::default(),
                    is_old_game: true,
                },
            },
        },
        TestCase {
            name: "BetweenGames, T45, B0, W0".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::BetweenGames,
                    secs_in_period: 45,
                    timeout: None,
                    scores: BlackWhiteBundle { black: 0, white: 0 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout Black T60".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::Black(60)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout White T60".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::White(60)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout Ref".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::Ref(32)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout Penalty Shot".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::PenaltyShot(32)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout Black T46".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::Black(46)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout White T46".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::White(46)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout Black T45".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::Black(45)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout White T45".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::White(45)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout Black T31".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::Black(31)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout White T31".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::White(31)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout Black T30".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::Black(30)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout White T30".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::White(30)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout Black T16".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::Black(16)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout White T16".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::White(16)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout Black T15".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::Black(15)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout White T15".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::White(15)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout Black T1".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::Black(1)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout White T1".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::White(1)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout Black T0".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::BetweenGames,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::Black(0)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B2, W5, Timeout White T0".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::BetweenGames,
                    secs_in_period: 345,
                    timeout: Some(TimeoutSnapshot::White(0)),
                    scores: BlackWhiteBundle { black: 2, white: 5 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T345, B100, W101".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::BetweenGames,
                    secs_in_period: 345,
                    timeout: None,
                    scores: BlackWhiteBundle {
                        black: 100,
                        white: 101,
                    },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T6000, B99, W99".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::BetweenGames,
                    secs_in_period: 6000,
                    timeout: None,
                    scores: BlackWhiteBundle {
                        black: 99,
                        white: 99,
                    },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, Tmax, Bmax, Wmax".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::BetweenGames,
                    secs_in_period: u16::MAX,
                    timeout: None,
                    scores: BlackWhiteBundle {
                        black: u8::MAX,
                        white: u8::MAX,
                    },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T900, B0, W0, Brightness Med".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Medium,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 900,
                    timeout: None,
                    scores: BlackWhiteBundle { black: 0, white: 0 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T900, B0, W0, Brightness High".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::High,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 900,
                    timeout: None,
                    scores: BlackWhiteBundle { black: 0, white: 0 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "First Half, T900, B0, W0, Brightness Outdoor".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: false,
                beep_test: false,
                brightness: Brightness::Outdoor,
                snapshot: GameSnapshotNoHeap {
                    current_period: GamePeriod::FirstHalf,
                    secs_in_period: 900,
                    timeout: None,
                    scores: BlackWhiteBundle { black: 0, white: 0 },
                    penalties: Default::default(),
                    is_old_game: false,
                },
            },
        },
        TestCase {
            name: "Flash".to_string(),
            transmitted_data: TransmittedData {
                white_on_right: false,
                flash: true,
                beep_test: false,
                brightness: Brightness::Low,
                snapshot: Default::default(),
            },
        },
    ];

    let mut file_contents = String::new();
    write!(
        &mut file_contents,
        r#"// Generated by `led-panel-sim`. Do not edit by hand.
        
package test_cases;
    typedef struct {{
        string name;

        // Inputs
        logic [19:0][7:0] data;

        // Expected Outputs
        digit ls_10_ex, ls_1_ex, rs_10_ex, rs_1_ex, m_10_ex, m_1_ex, s_10_ex, s_1_ex;
        logic white_on_left_ex, white_on_right_ex;
        logic left_to_ind_ex, right_to_ind_ex, ref_to_ind_ex;
        logic one_ex, slash_ex, two_ex, overtime_ex, sdn_dth_ex;
        logic colon_ex;
        logic [1:0] brightness_ex;
    }} test_case;

    test_case all_tests [{}:0] = '{{
"#,
        test_cases.len()
    )
    .unwrap();

    writeln!(
        &mut file_contents,
        "{},",
        test_to_verilog(
            "All zeros",
            [0u8; TransmittedData::ENCODED_LEN],
            DisplayState::OFF,
            Brightness::Low,
        )
    )
    .unwrap();

    write!(
        &mut file_contents,
        "{}",
        test_cases
            .iter()
            .map(|t| t.as_verilog())
            .collect::<Vec<String>>()
            .join(",\n")
    )
    .unwrap();

    write!(
        &mut file_contents,
        r#"
    }};
endpackage
"#
    )
    .unwrap();

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("../led-panel/tb/test_cases.sv")
        .expect("Unable to open/create file");
    file.write_all(file_contents.as_bytes())
        .expect("Unable to write data");
}
