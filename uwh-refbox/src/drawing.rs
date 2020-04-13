use crate::config::Config;
use crate::game_snapshot::*;
use embedded_graphics::prelude::*;
use embedded_graphics::{fonts::Font, pixelcolor};
use embedded_graphics_simulator::RgbDisplay;
use fonts::fonts::{Font11x25, Font16x31, Font22x46, Font32x64, Font6x8, Font8x15};
use std::{
    convert::TryInto,
    fmt::Display,
    ops::{Div, Rem},
};

pub fn draw_panels(display: &mut RgbDisplay, mut state: GameSnapshot, config: &Config) {
    let red = pixelcolor::Rgb888::new(255, 0, 0);
    let yellow = pixelcolor::Rgb888::new(255, 255, 0);
    let green = pixelcolor::Rgb888::new(0, 255, 0);
    let blue = pixelcolor::Rgb888::new(64, 128, 255); //purple (225, 0, 255)
    let white = pixelcolor::Rgb888::new(255, 255, 255);

    let game_color = match state.timeout {
        TimeoutSnapshot::PenaltyShot(_) => red,
        TimeoutSnapshot::Ref(_) => yellow,
        _ => match state.current_period {
            GamePeriod::FirstHalf
            | GamePeriod::SecondHalf
            | GamePeriod::OvertimeFirstHalf
            | GamePeriod::OvertimeSecondHalf => green,
            GamePeriod::BetweenGames
            | GamePeriod::HalfTime
            | GamePeriod::PreOvertime
            | GamePeriod::OvertimeHalfTime
            | GamePeriod::PreSuddenDeath => yellow,
            GamePeriod::SuddenDeath => red,
        },
    };

    let timeout_color = match state.timeout {
        TimeoutSnapshot::White(_) => white,
        TimeoutSnapshot::Black(_) => blue,
        TimeoutSnapshot::Ref(_) => yellow,
        _ => red,
    };

    // EVERYTHING TO BE DISPLAYED ON THE CENTER 2 TIME PANELS
    match state.timeout {
        TimeoutSnapshot::None => {
            //No timeout currently
            display.draw(
                Font22x46::render_str(&secs_to_time_string(state.secs_in_period))
                    .stroke(Some(game_color))
                    .translate(Point::new(74, 18)),
            );

            let (text, x, y) = match state.current_period {
                GamePeriod::BetweenGames => ("NEXT GAME", 92, 2),
                GamePeriod::FirstHalf => ("1ST HALF", 100, 2),
                GamePeriod::HalfTime => ("HALF-TIME", 92, 2),
                GamePeriod::SecondHalf => ("2ND HALF", 100, 2),
                GamePeriod::PreOvertime => ("PRE-OVERTIME", 80, 2),
                GamePeriod::OvertimeFirstHalf => ("O/T 1ST HALF", 80, 2),
                GamePeriod::OvertimeHalfTime => ("O/T HALF TIME", 76, 2),
                GamePeriod::OvertimeSecondHalf => ("O/T 2ND HALF", 80, 2),
                GamePeriod::PreSuddenDeath => ("PRE-SUDDEN DEATH", 64, 2),
                GamePeriod::SuddenDeath => ("SUDDEN DEATH", 80, 2),
            };

            display.draw(
                Font8x15::render_str(text)
                    .stroke(Some(game_color))
                    .translate(Point::new(x, y)),
            );
        }

        _ => {
            //Some timeout currently
            display.draw(
                Font16x31::render_str(&secs_to_time_string(state.secs_in_period))
                    .stroke(Some(game_color))
                    .translate(Point::new(108, 33)),
            );

            let (text1, x1, y1, text2, x2, y2, text3, x3, y3) = match state.current_period {
                GamePeriod::FirstHalf => ("1ST", 72, 33, "", 0, 0, "HALF", 68, 48),
                GamePeriod::SecondHalf => ("2ND", 72, 33, "", 0, 0, "HALF", 68, 48),
                GamePeriod::OvertimeFirstHalf => ("OT 1", 64, 33, "ST", 96, 33, "HALF", 68, 48),
                GamePeriod::OvertimeSecondHalf => ("OT 2", 64, 33, "ND", 96, 33, "HALF", 68, 48),
                GamePeriod::SuddenDeath => ("", 0, 0, "SUDDEN", 70, 39, "DEATH", 68, 48),
                _ => ("PERIOD ERROR", 72, 33, "", 0, 0, "", 0, 0),
            };

            display.draw(
                Font8x15::render_str(text1)
                    .stroke(Some(game_color))
                    .translate(Point::new(x1, y1)),
            );
            display.draw(
                Font6x8::render_str(text2)
                    .stroke(Some(game_color))
                    .translate(Point::new(x2, y2)),
            );
            display.draw(
                Font8x15::render_str(text3)
                    .stroke(Some(game_color))
                    .translate(Point::new(x3, y3)),
            );

            match state.timeout {
                TimeoutSnapshot::White(secs) => {
                    display.draw(
                        Font8x15::render_str("WHITE")
                            .stroke(Some(timeout_color))
                            .translate(Point::new(76, 2)),
                    );
                    display.draw(
                        Font8x15::render_str("TIMEOUT")
                            .stroke(Some(timeout_color))
                            .translate(Point::new(68, 17)),
                    );
                    display.draw(
                        Font16x31::render_str(&format!(":{}", secs))
                            .stroke(Some(timeout_color))
                            .translate(Point::new(132, 2)),
                    );
                }

                TimeoutSnapshot::Black(secs) => {
                    display.draw(
                        Font8x15::render_str("BLACK")
                            .stroke(Some(timeout_color))
                            .translate(Point::new(76, 2)),
                    );
                    display.draw(
                        Font8x15::render_str("TIMEOUT")
                            .stroke(Some(timeout_color))
                            .translate(Point::new(68, 17)),
                    );
                    display.draw(
                        Font16x31::render_str(&format!(":{}", secs))
                            .stroke(Some(timeout_color))
                            .translate(Point::new(132, 2)),
                    );
                }

                TimeoutSnapshot::Ref(_) => display.draw(
                    Font11x25::render_str("REF TIMEOUT")
                        .stroke(Some(timeout_color))
                        .translate(Point::new(68, 3)),
                ),

                TimeoutSnapshot::PenaltyShot(_) => {
                    display.draw(
                        Font11x25::render_str("PENALTY")
                            .stroke(Some(timeout_color))
                            .translate(Point::new(64, 3)),
                    );
                    display.draw(
                        Font11x25::render_str("SHOT")
                            .stroke(Some(timeout_color))
                            .translate(Point::new(149, 3)),
                    );
                }

                _ => display.draw(
                    Font8x15::render_str("T/O ERROR")
                        .stroke(Some(red))
                        .translate(Point::new(64, 133)),
                ),
            };
        }
    };

    // Create Vectors for the Black and White Penalty lists
    let mut black_penalties = vec![];
    let mut white_penalties = vec![];

    // Sorting Penalties by Time and then by Color

    state.penalties.sort_by(|a, b| a.time.cmp(&b.time));

    for penalty in &state.penalties {
        if penalty.color == Color::Black {
            black_penalties.push(penalty);
        } else {
            white_penalties.push(penalty);
        }
    }

    let left_penalties;
    let right_penalties;
    let left_score;
    let right_score;
    let left_color;
    let right_color;

    if config.hardware.white_on_right {
        left_penalties = black_penalties;
        right_penalties = white_penalties;
        left_score = state.b_score;
        right_score = state.w_score;
        left_color = blue;
        right_color = white;
    } else {
        left_penalties = white_penalties;
        right_penalties = black_penalties;
        left_score = state.w_score;
        right_score = state.b_score;
        left_color = white;
        right_color = blue;
    }

    // Score on Left Score Panel
    let left_score_string = format!("{:<2}", left_score);
    if left_penalties.is_empty() {
        if left_score < 10 {
            // Full Size Left Score (Single Digit Centered)
            display.draw(
                Font32x64::render_str(&left_score_string)
                    .stroke(Some(left_color))
                    .translate(Point::new(18, 2)),
            );
        } else {
            // Full Size Left Score (Double Digit Centered)
            display.draw(
                Font32x64::render_str(&left_score_string)
                    .stroke(Some(left_color))
                    .translate(Point::new(2, 2)),
            );
        }
    } else if left_score < 10 {
        // Full Size Left Score, Single Digit - Justified Right/Inside/Towards Time Panels
        display.draw(
            Font32x64::render_str(&left_score_string)
                .stroke(Some(left_color))
                .translate(Point::new(34, 2)),
        );
    } else {
        // 3/4 Size Left Score (Double Digit - Centered on Score Panel)
        display.draw(
            Font22x46::render_str(&left_score_string)
                .stroke(Some(left_color))
                .translate(Point::new(11, 2)),
        );
    };

    // Score on Right Score Panel
    let right_score_string = format!("{:<2}", right_score);
    if right_penalties.is_empty() {
        if right_score < 10 {
            // Full Size Right Score (Single Digit Centered)
            display.draw(
                Font32x64::render_str(&right_score_string)
                    .stroke(Some(right_color))
                    .translate(Point::new(210, 2)),
            );
        } else {
            // Full Size Right Score (Double Digit Centered)
            display.draw(
                Font32x64::render_str(&right_score_string)
                    .stroke(Some(right_color))
                    .translate(Point::new(194, 2)),
            );
        }
    } else if right_score < 10 {
        // Full Size Right Score, Single Digit - Justified Right/Inside/Towards Time Panels
        display.draw(
            Font32x64::render_str(&right_score_string)
                .stroke(Some(right_color))
                .translate(Point::new(194, 2)),
        );
    } else {
        // 3/4 Size Right Score (Double Digit - Centered on Score Panel)
        display.draw(
            Font22x46::render_str(&right_score_string)
                .stroke(Some(right_color))
                .translate(Point::new(203, 2)),
        );
    };

    // Define layout for Penalties
    let mut draw_penalty = |x_pos: i32, y_pos: i32, color, penalty: &PenaltySnapshot| {
        display.draw(
            Font6x8::render_str(&format!("#{}", penalty.player_number))
                .stroke(Some(color))
                .translate(Point::new(
                    if penalty.player_number > 9 { 3 } else { 6 } + x_pos,
                    y_pos,
                )),
        );
        match penalty.time {
            PenaltyTime::Seconds(secs) => {
                display.draw(
                    Font6x8::render_str(&secs_to_time_string(secs))
                        .stroke(Some(color))
                        .translate(Point::new(x_pos - 6, y_pos + 8)),
                );
            }
            PenaltyTime::TotalDismissal => {
                display.draw(
                    Font6x8::render_str(&"DSMS".to_string())
                        .stroke(Some(red))
                        .translate(Point::new(x_pos, y_pos + 8)),
                );
            }
        }
    };

    // Penalties on Left Score Panel
    if left_score < 10 {
        // Vertical Penalties (Up to 3) - Justified Left/Outside/Away from Time Panels
        // Penalties "Fall-Off" the Bottom as they run out
        for (i, penalty) in left_penalties.iter().take(3).enumerate() {
            let i: i32 = i.try_into().unwrap();
            let x_pos = 5;
            let y_pos: i32 = 47 - i * 22;
            draw_penalty(x_pos, y_pos, left_color, penalty);
        }
    } else {
        // Horizontal Penalties (Up to 2) - Justified Left/Outside/Away from Time Panels
        // Penalties "Fall-Off" the Left Side as they run out
        for (i, penalty) in left_penalties.iter().take(2).enumerate() {
            let i: i32 = i.try_into().unwrap();
            let x_pos: i32 = 5 + i * 29;
            let y_pos = 47;
            draw_penalty(x_pos, y_pos, left_color, penalty);
        }
    }

    // Penalties on Right Score Panel
    if right_score < 10 {
        // Vertical Penalties (Up to 3) - Justified Right/Outside/Away from Time Panels
        // Penalties "Fall-Off" the Bottom as they run out
        for (i, penalty) in right_penalties.iter().take(3).enumerate() {
            let i: i32 = i.try_into().unwrap();
            let x_pos = 228;
            let y_pos: i32 = 47i32 - i * 22i32;
            draw_penalty(x_pos, y_pos, right_color, penalty);
        }
    } else {
        // Horizontal Penalties (Up to 2) - Justified Right/Outside/Away from Time Panels
        // Penalties "Fall-Off" the Right Side as they run out
        for (i, penalty) in right_penalties.iter().take(2).enumerate() {
            let i: i32 = i.try_into().unwrap();
            let x_pos: i32 = 228i32 - i * 29i32;
            let y_pos = 47;
            draw_penalty(x_pos, y_pos, right_color, penalty);
        }
    }
}

pub fn secs_to_time_string<T>(secs: T) -> String
where
    T: Div<T> + Rem<T> + From<u8> + Copy,
    <T as Div>::Output: Display,
    <T as Rem>::Output: Display,
{
    let min = secs / T::from(60u8);
    let sec = secs % T::from(60u8);
    format!("{:2}:{:02}", min, sec)
}
