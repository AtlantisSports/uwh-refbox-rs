use arrayvec::ArrayString;
use core::{
    fmt::{Debug, Display, Write},
    ops::{Div, Rem},
};
use embedded_graphics::{
    geometry::{Point, Size},
    mono_font::MonoTextStyle,
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{rectangle::Rectangle, PrimitiveStyle},
    text::{Alignment, Baseline, LineHeight, Text, TextStyle, TextStyleBuilder},
};
use fonts::fonts::{FONT_10X25, FONT_14X31, FONT_20X46, FONT_28X64, FONT_5X8, FONT_7X15};
use more_asserts::*;
use uwh_common::{drawing_support::*, game_snapshot::*};

/// Draws all the details of the game onto the provided display. Assumes the dispaly is 256x64
///
/// Assumes the penalties have already been sorted
pub fn draw_panels<D: DrawTarget<Color = Rgb888>>(
    display: &mut D,
    state: GameSnapshotNoHeap,
    white_on_right: bool,
    flash: bool,
) -> Result<(), D::Error> {
    const RED: Rgb888 = Rgb888::RED;
    const YELLOW: Rgb888 = Rgb888::YELLOW;
    const GREEN: Rgb888 = Rgb888::GREEN;
    const BLUE: Rgb888 = Rgb888::new(64, 128, 255); //purple (225, 0, 255)
    const WHITE: Rgb888 = Rgb888::WHITE;
    const FLASH_COLOR: Rgb888 = Rgb888::new(0, 200, 200);

    const CENTERED: TextStyle = TextStyleBuilder::new()
        .alignment(Alignment::Center)
        .baseline(Baseline::Top)
        .line_height(LineHeight::Percent(100))
        .build();

    const LEFT_ALGN: TextStyle = TextStyleBuilder::new()
        .alignment(Alignment::Left)
        .baseline(Baseline::Top)
        .line_height(LineHeight::Percent(100))
        .build();

    const RIGHT_ALGN: TextStyle = TextStyleBuilder::new()
        .alignment(Alignment::Right)
        .baseline(Baseline::Top)
        .line_height(LineHeight::Percent(100))
        .build();

    if flash {
        Rectangle::new(Point::new(0, 0), Size::new(255, 64))
            .into_styled(PrimitiveStyle::with_fill(FLASH_COLOR))
            .draw(display)?;
        return Ok(());
    }

    let game_color = match state.timeout {
        TimeoutSnapshot::PenaltyShot(_) => RED,
        TimeoutSnapshot::Ref(_) => YELLOW,
        _ => match state.current_period {
            GamePeriod::FirstHalf
            | GamePeriod::SecondHalf
            | GamePeriod::OvertimeFirstHalf
            | GamePeriod::OvertimeSecondHalf => GREEN,
            GamePeriod::BetweenGames
            | GamePeriod::HalfTime
            | GamePeriod::PreOvertime
            | GamePeriod::OvertimeHalfTime
            | GamePeriod::PreSuddenDeath => YELLOW,
            GamePeriod::SuddenDeath => RED,
        },
    };

    let timeout_color = match state.timeout {
        TimeoutSnapshot::White(_) => WHITE,
        TimeoutSnapshot::Black(_) => BLUE,
        TimeoutSnapshot::Ref(_) => YELLOW,
        _ => RED,
    };

    // EVERYTHING TO BE DISPLAYED ON THE CENTER 2 TIME PANELS
    match state.timeout {
        TimeoutSnapshot::None => {
            Text::with_text_style(
                &secs_to_time_string(state.secs_in_period),
                Point::new(127, 18),
                MonoTextStyle::new(&FONT_20X46, game_color),
                CENTERED,
            )
            .draw(display)?;

            let text = match state.current_period {
                GamePeriod::BetweenGames => "NEXT GAME IN",
                GamePeriod::FirstHalf => "1ST HALF",
                GamePeriod::HalfTime => "HALF TIME",
                GamePeriod::SecondHalf => "2ND HALF",
                GamePeriod::PreOvertime => "PRE-OVERTIME",
                GamePeriod::OvertimeFirstHalf => "O/T 1ST HALF",
                GamePeriod::OvertimeHalfTime => "O/T HALF TIME",
                GamePeriod::OvertimeSecondHalf => "O/T 2ND HALF",
                GamePeriod::PreSuddenDeath => "PRE-SUDDEN DEATH",
                GamePeriod::SuddenDeath => "SUDDEN DEATH",
            };

            Text::with_text_style(
                text,
                Point::new(127, 2),
                MonoTextStyle::new(&FONT_7X15, game_color),
                CENTERED,
            )
            .draw(display)?;
        }

        _ => {
            // There is currently a timeout
            Text::with_text_style(
                &secs_to_time_string(state.secs_in_period),
                Point::new(152, 33),
                MonoTextStyle::new(&FONT_14X31, game_color),
                CENTERED,
            )
            .draw(display)?;

            let period_text = match state.current_period {
                GamePeriod::BetweenGames => "NEXT\nGAME",
                GamePeriod::FirstHalf => "1ST\nHALF",
                GamePeriod::HalfTime => "HALF\nTIME",
                GamePeriod::SecondHalf => "2ND\nHALF",
                GamePeriod::PreOvertime => "PRE OT\nBREAK",
                GamePeriod::OvertimeFirstHalf => "OT 1ST\nHALF",
                GamePeriod::OvertimeHalfTime => "OT HLF\nTIME",
                GamePeriod::OvertimeSecondHalf => "OT 2ND\nHALF",
                GamePeriod::PreSuddenDeath => "PRE SD\nBREAK",
                GamePeriod::SuddenDeath => "SUDDEN\nDEATH",
            };

            Text::with_text_style(
                period_text,
                Point::new(87, 33),
                MonoTextStyle::new(&FONT_7X15, game_color),
                CENTERED,
            )
            .draw(display)?;

            match state.timeout {
                TimeoutSnapshot::White(secs) => {
                    Text::with_text_style(
                        "WHITE\nTIMEOUT",
                        Point::new(99, 2),
                        MonoTextStyle::new(&FONT_7X15, timeout_color),
                        CENTERED,
                    )
                    .draw(display)?;
                    Text::with_text_style(
                        &secs_to_short_time_string(secs),
                        Point::new(138, 2),
                        MonoTextStyle::new(&FONT_14X31, timeout_color),
                        LEFT_ALGN,
                    )
                    .draw(display)?;
                }

                TimeoutSnapshot::Black(secs) => {
                    Text::with_text_style(
                        "BLACK\nTIMEOUT",
                        Point::new(99, 2),
                        MonoTextStyle::new(&FONT_7X15, timeout_color),
                        CENTERED,
                    )
                    .draw(display)?;
                    Text::with_text_style(
                        &secs_to_short_time_string(secs),
                        Point::new(138, 2),
                        MonoTextStyle::new(&FONT_14X31, timeout_color),
                        LEFT_ALGN,
                    )
                    .draw(display)?;
                }

                TimeoutSnapshot::Ref(_) => {
                    Text::with_text_style(
                        "REF TIMEOUT",
                        Point::new(127, 3),
                        MonoTextStyle::new(&FONT_10X25, timeout_color),
                        CENTERED,
                    )
                    .draw(display)?;
                }

                TimeoutSnapshot::PenaltyShot(_) => {
                    Text::with_text_style(
                        "PENALTY",
                        Point::new(64, 3),
                        MonoTextStyle::new(&FONT_10X25, timeout_color),
                        LEFT_ALGN,
                    )
                    .draw(display)?;
                    Text::with_text_style(
                        "SHOT",
                        Point::new(149, 3),
                        MonoTextStyle::new(&FONT_10X25, timeout_color),
                        LEFT_ALGN,
                    )
                    .draw(display)?;
                }

                _ => {
                    Text::with_text_style(
                        "T/O ERROR",
                        Point::new(127, 3),
                        MonoTextStyle::new(&FONT_10X25, RED),
                        CENTERED,
                    )
                    .draw(display)?;
                }
            };
        }
    };

    let left_penalties;
    let right_penalties;
    let left_score;
    let right_score;
    let left_color;
    let right_color;

    if white_on_right {
        left_penalties = state.b_penalties;
        right_penalties = state.w_penalties;
        left_score = state.b_score;
        right_score = state.w_score;
        left_color = BLUE;
        right_color = WHITE;
    } else {
        left_penalties = state.w_penalties;
        right_penalties = state.b_penalties;
        left_score = state.w_score;
        right_score = state.b_score;
        left_color = WHITE;
        right_color = BLUE;
    }

    // Score on Left Score Panel
    let mut left_score_string = ArrayString::<2>::new();
    write!(&mut left_score_string, "{}", left_score).unwrap();
    if left_penalties.is_empty() {
        Text::with_text_style(
            &left_score_string,
            Point::new(31, 2),
            MonoTextStyle::new(&FONT_28X64, left_color),
            CENTERED,
        )
        .draw(display)?;
    } else if left_score < 10 {
        // Full Size Left Score, Single Digit - Justified Right/Inside/Towards Time Panels
        Text::with_text_style(
            &left_score_string,
            Point::new(61, 2),
            MonoTextStyle::new(&FONT_28X64, left_color),
            RIGHT_ALGN,
        )
        .draw(display)?;
    } else {
        // 3/4 Size Left Score (Double Digit - Centered on Score Panel)
        Text::with_text_style(
            &left_score_string,
            Point::new(31, 2),
            MonoTextStyle::new(&FONT_20X46, left_color),
            CENTERED,
        )
        .draw(display)?;
    };

    // Score on Right Score Panel
    let mut right_score_string = ArrayString::<2>::new();
    write!(&mut right_score_string, "{}", right_score).unwrap();
    if right_penalties.is_empty() {
        Text::with_text_style(
            &right_score_string,
            Point::new(223, 2),
            MonoTextStyle::new(&FONT_28X64, right_color),
            CENTERED,
        )
        .draw(display)?;
    } else if right_score < 10 {
        // Full Size Right Score, Single Digit - Justified Left/Inside/Towards Time Panels
        Text::with_text_style(
            &right_score_string,
            Point::new(194, 2),
            MonoTextStyle::new(&FONT_28X64, right_color),
            LEFT_ALGN,
        )
        .draw(display)?;
    } else {
        // 3/4 Size Right Score (Double Digit - Centered on Score Panel)
        Text::with_text_style(
            &right_score_string,
            Point::new(223, 2),
            MonoTextStyle::new(&FONT_20X46, right_color),
            CENTERED,
        )
        .draw(display)?;
    };

    // Define layout for Penalties
    let mut draw_penalty =
        |x_pos: i32, y_pos: i32, color, penalty: &PenaltySnapshot| -> Result<(), D::Error> {
            let mut penalty_string = ArrayString::<3>::new();
            write!(&mut penalty_string, "#{}", penalty.player_number).unwrap();
            Text::with_text_style(
                &penalty_string,
                Point::new(x_pos, y_pos),
                MonoTextStyle::new(&FONT_5X8, color),
                CENTERED,
            )
            .draw(display)?;
            let time: ArrayString<4> = match penalty.time {
                PenaltyTime::Seconds(secs) => {
                    ArrayString::from(secs_to_time_string(secs).trim()).unwrap()
                }
                PenaltyTime::TotalDismissal => ArrayString::from("DSMS").unwrap(),
            };
            Text::with_text_style(
                &time,
                Point::new(x_pos, y_pos + 8),
                MonoTextStyle::new(&FONT_5X8, RED),
                CENTERED,
            )
            .draw(display)?;
            Ok(())
        };

    // Penalties on Left Score Panel
    if left_score < 10 {
        // Vertical Penalties (Up to 3) - Justified Left/Outside/Away from Time Panels
        // Penalties "Fall-Off" the Bottom as they run out
        for (i, penalty) in [0i32, 1, 2].into_iter().zip(left_penalties.iter()) {
            let x_pos = 15;
            let y_pos = 47 - i * 22;
            draw_penalty(x_pos, y_pos, left_color, penalty)?;
        }
    } else {
        // Horizontal Penalties (Up to 2) - Justified Left/Outside/Away from Time Panels
        // Penalties "Fall-Off" the Left Side as they run out
        for (i, penalty) in [0i32, 1].into_iter().zip(left_penalties.iter()) {
            let x_pos = 17 + i * 29;
            let y_pos = 47;
            draw_penalty(x_pos, y_pos, left_color, penalty)?;
        }
    }

    // Penalties on Right Score Panel
    if right_score < 10 {
        // Vertical Penalties (Up to 3) - Justified Right/Outside/Away from Time Panels
        // Penalties "Fall-Off" the Bottom as they run out
        for (i, penalty) in [0i32, 1, 2].into_iter().zip(right_penalties.iter()) {
            let x_pos = 240;
            let y_pos = 47i32 - i * 22i32;
            draw_penalty(x_pos, y_pos, right_color, penalty)?;
        }
    } else {
        // Horizontal Penalties (Up to 2) - Justified Right/Outside/Away from Time Panels
        // Penalties "Fall-Off" the Right Side as they run out
        for (i, penalty) in [0i32, 1].into_iter().zip(right_penalties.iter()) {
            let x_pos = 238 - i * 29;
            let y_pos = 47;
            draw_penalty(x_pos, y_pos, right_color, penalty)?;
        }
    }
    Ok(())
}

pub fn secs_to_time_string<T>(secs: T) -> ArrayString<5>
where
    T: Div<T> + Rem<T> + From<u16> + Copy + Ord + Debug,
    <T as Div>::Output: Display,
    <T as Rem>::Output: Display,
{
    assert_le!(secs, T::from(MAX_STRINGABLE_SECS));
    let min = secs / T::from(60u16);
    let sec = secs % T::from(60u16);
    let mut time_string = ArrayString::new();
    write!(&mut time_string, "{:2}:{:02}", min, sec).unwrap();
    time_string
}

pub fn secs_to_long_time_string<T>(secs: T) -> ArrayString<8>
where
    T: Div<T> + Rem<T> + From<u32> + Copy + Ord + Debug,
    <T as Div>::Output: Display,
    <T as Rem>::Output: Display,
{
    assert_le!(secs, T::from(MAX_LONG_STRINGABLE_SECS));
    let min = secs / T::from(60u32);
    let sec = secs % T::from(60u32);
    let mut time_string = ArrayString::new();
    write!(&mut time_string, "{:5}:{:02}", min, sec).unwrap();
    time_string
}

pub fn secs_to_short_time_string<T>(secs: T) -> ArrayString<3>
where
    T: From<u8> + Ord + Copy + Display + Debug,
{
    assert_le!(secs, T::from(99u8));
    let mut time_string = ArrayString::new();
    write!(&mut time_string, ":{:02}", secs).unwrap();
    time_string
}
