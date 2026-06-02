use super::{
    BORDER_RADIUS, BORDER_WIDTH, black, black_pressed, blue, blue_pressed, border_color,
    disabled_color, gray, gray_pressed, green, green_pressed, light_gray, light_gray_pressed,
    orange, orange_pressed, red, red_pressed, white, white_pressed, window_background, yellow,
    yellow_pressed,
};
use iced::{
    Background, Border, Shadow, Theme,
    widget::button::{Status, Style},
};

pub fn gray_button(_theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(window_background())),
        Status::Pressed => Some(Background::Color(gray_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(gray())),
    };

    let text_color = if matches!(status, Status::Disabled) {
        disabled_color()
    } else {
        black()
    };

    let border_color_val = match status {
        Status::Disabled => disabled_color(),
        Status::Pressed | Status::Active | Status::Hovered => border_color(),
    };

    let border_width = match status {
        Status::Disabled => BORDER_WIDTH,
        Status::Pressed | Status::Active | Status::Hovered => 0.0,
    };

    let border = Border {
        width: border_width,
        color: border_color_val,
        radius: BORDER_RADIUS,
    };

    Style {
        background,
        text_color,
        border,
        shadow: Shadow::default(),
    }
}

pub fn light_gray_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(window_background())),
        Status::Pressed => Some(Background::Color(light_gray_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(light_gray())),
    };

    Style {
        background,
        ..gray_button(theme, status)
    }
}

pub fn light_gray_selected_button(theme: &Theme, status: Status) -> Style {
    let mut style = light_gray_button(theme, status);
    style.border.width = BORDER_WIDTH;
    style
}

pub fn white_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(window_background())),
        Status::Pressed => Some(Background::Color(white_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(white())),
    };

    Style {
        background,
        ..gray_button(theme, status)
    }
}

pub fn white_selected_button(theme: &Theme, status: Status) -> Style {
    let mut style = white_button(theme, status);
    style.border.width = BORDER_WIDTH;
    style
}

pub fn black_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(window_background())),
        Status::Pressed => Some(Background::Color(black_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(black())),
    };

    let text_color = if matches!(status, Status::Disabled) {
        disabled_color()
    } else {
        white()
    };

    Style {
        background,
        text_color,
        ..gray_button(theme, status)
    }
}

pub fn black_selected_button(theme: &Theme, status: Status) -> Style {
    let mut style = black_button(theme, status);
    style.border.width = BORDER_WIDTH;
    style
}

pub fn red_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(window_background())),
        Status::Pressed => Some(Background::Color(red_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(red())),
    };

    Style {
        background,
        ..gray_button(theme, status)
    }
}

pub fn red_selected_button(theme: &Theme, status: Status) -> Style {
    let mut style = red_button(theme, status);
    style.border.width = BORDER_WIDTH;
    style
}

pub fn orange_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(window_background())),
        Status::Pressed => Some(Background::Color(orange_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(orange())),
    };

    Style {
        background,
        ..gray_button(theme, status)
    }
}

pub fn orange_selected_button(theme: &Theme, status: Status) -> Style {
    let mut style = orange_button(theme, status);
    style.border.width = BORDER_WIDTH;
    style
}

pub fn yellow_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(window_background())),
        Status::Pressed => Some(Background::Color(yellow_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(yellow())),
    };

    Style {
        background,
        ..gray_button(theme, status)
    }
}

pub fn yellow_selected_button(theme: &Theme, status: Status) -> Style {
    let mut style = yellow_button(theme, status);
    style.border.width = BORDER_WIDTH;
    style
}

pub fn green_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(window_background())),
        Status::Pressed => Some(Background::Color(green_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(green())),
    };

    Style {
        background,
        ..gray_button(theme, status)
    }
}

pub fn green_selected_button(theme: &Theme, status: Status) -> Style {
    let mut style = green_button(theme, status);
    style.border.width = BORDER_WIDTH;
    style
}

pub fn blue_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(window_background())),
        Status::Pressed => Some(Background::Color(blue_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(blue())),
    };

    let text_color = if matches!(status, Status::Disabled) {
        disabled_color()
    } else {
        white()
    };

    Style {
        background,
        text_color,
        ..gray_button(theme, status)
    }
}

pub fn blue_selected_button(theme: &Theme, status: Status) -> Style {
    let mut style = blue_button(theme, status);
    style.border.width = BORDER_WIDTH;
    style
}

pub fn blue_with_border_button(theme: &Theme, status: Status) -> Style {
    let mut style = blue_button(theme, status);
    style.border.width = BORDER_WIDTH;
    style.border.color = gray();
    style
}

// impl button::Catalog for Theme {
//     type Class<'a> = ButtonClass;

//     fn default<'a>() -> Self::Class<'a> {
//         Default::default()
//     }

//     fn style(&self, class: &Self::Class<'_>, status: button::Status) -> button::Style {
//         let background = match status {
//             button::Status::Disabled => Some(Background::Color(WINDOW_BACKGROUND)),
//             button::Status::Pressed => match class {
//                 LWhite_button | LWhiteSelected_button => {
//                     Some(Background::Color(WHITE_PRESSED))
//                 }
//                 LBlack_button | LBlackSelected_button => {
//                     Some(Background::Color(BLACK_PRESSED))
//                 }
//                 LRed_button | LRedSelected_button => Some(Background::Color(RED_PRESSED)),
//                 LOrange_button | LOrangeSelected_button => {
//                     Some(Background::Color(ORANGE_PRESSED))
//                 }
//                 LYellow_button | LYellowSelected_button => {
//                     Some(Background::Color(YELLOW_PRESSED))
//                 }
//                 LGreen_button | LGreenSelected_button => {
//                     Some(Background::Color(GREEN_PRESSED))
//                 }
//                 LBlue_button | LBlueSelected_button | LBlueWithBorder_button => {
//                     Some(Background::Color(BLUE_PRESSED))
//                 }
//                 LGray_button => Some(Background::Color(GRAY_PRESSED)),
//                 LLightGray_button | LLightGraySelected_button => {
//                     Some(Background::Color(LIGHT_GRAY_PRESSED))
//                 }
//             },
//             button::Status::Active | button::Status::Hovered => match class {
//                 LWhite_button | LWhiteSelected_button => Some(Background::Color(WHITE)),
//                 LBlack_button | LBlackSelected_button => Some(Background::Color(BLACK)),
//                 LRed_button | LRedSelected_button => Some(Background::Color(RED)),
//                 LOrange_button | LOrangeSelected_button => {
//                     Some(Background::Color(ORANGE))
//                 }
//                 LYellow_button | LYellowSelected_button => {
//                     Some(Background::Color(YELLOW))
//                 }
//                 LGreen_button | LGreenSelected_button => Some(Background::Color(GREEN)),
//                 LBlue_button | LBlueSelected_button | LBlueWithBorder_button => {
//                     Some(Background::Color(BLUE))
//                 }
//                 LGray_button => Some(Background::Color(GRAY)),
//                 LLightGray_button | LLightGraySelected_button => {
//                     Some(Background::Color(LIGHT_GRAY))
//                 }
//             },
//         };

//         let text_color = if matches!(status, button::Status::Disabled) {
//             DISABLED_COLOR
//         } else {
//             match class {
//                 LWhite_button
//                 | LWhiteSelected_button
//                 | LRed_button
//                 | LRedSelected_button
//                 | LOrange_button
//                 | LOrangeSelected_button
//                 | LYellow_button
//                 | LYellowSelected_button
//                 | LGreen_button
//                 | LGreenSelected_button
//                 | LGray_button
//                 | LLightGray_button
//                 | LLightGraySelected_button => BLACK,
//                 LBlack_button
//                 | LBlackSelected_button
//                 | LBlue_button
//                 | LBlueSelected_button
//                 | LBlueWithBorder_button => WHITE,
//             }
//         };

//         let border_color = match status {
//             button::Status::Disabled => DISABLED_COLOR,
//             button::Status::Pressed | button::Status::Active | button::Status::Hovered => {
//                 match class {
//                     LBlueWithBorder_button => GRAY,
//                     _ => BORDER_COLOR,
//                 }
//             }
//         };

//         let border_width = match status {
//             button::Status::Disabled => BORDER_WIDTH,
//             button::Status::Pressed | button::Status::Active | button::Status::Hovered => {
//                 match class {
//                     LBlack_button
//                     | LBlue_button
//                     | LGray_button
//                     | LLightGray_button
//                     | LWhite_button
//                     | LRed_button
//                     | LOrange_button
//                     | LYellow_button
//                     | LGreen_button => 0.0,
//                     LBlackSelected_button
//                     | LBlueSelected_button
//                     | LLightGraySelected_button
//                     | LWhiteSelected_button
//                     | LRedSelected_button
//                     | LOrangeSelected_button
//                     | LYellowSelected_button
//                     | LGreenSelected_button
//                     | LBlueWithBorder_button => BORDER_WIDTH,
//                 }
//             }
//         };

//         let border = Border {
//             width: border_width,
//             color: border_color,
//             radius: BORDER_RADIUS,
//         };

//         button::Style {
//             background,
//             text_color,
//             border,
//             shadow: Shadow::default(),
//         }
//     }
// }
