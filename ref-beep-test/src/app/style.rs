use iced::{
    application,
    widget::{self, button, container, scrollable, svg, text},
    Background, BorderRadius, Color, Vector,
};
use iced_core::text::LineHeight;
use iced_renderer::Renderer;
use paste::paste;

pub const BORDER_RADIUS: f32 = 9.0;
pub const BORDER_WIDTH: f32 = 6.0;
pub const SPACING: f32 = 8.0;
pub const PADDING: f32 = 8.0;
pub const MIN_BUTTON_SIZE: f32 = 89.0;

pub const SMALL_PLUS_TEXT: f32 = 29.0;
pub const LARGE_TEXT: f32 = 66.0;

pub const LINE_HEIGHT: LineHeight = LineHeight::Relative(1.15);

// See https://stackoverflow.com/a/727339 for color mixing math. For darkening colors with pure
// black, the math simplifies to new_r = orig_r * (1 - black_alpha), so we will multiply by the
// value of (1 - black_alpha)
macro_rules! make_color {
    ($name:ident, $r:literal, $g:literal, $b:literal) => {
        paste! {
            pub const $name: iced::Color = iced::Color::from_rgb($r, $g, $b);
            pub const [<$name _PRESSED>]: iced::Color = iced::Color::from_rgb(
                $r * 0.85,
                $g * 0.85,
                $b * 0.85);
        }
    };
}

make_color!(RED, 1.0, 0.0, 0.0);
make_color!(ORANGE, 1.0, 0.5, 0.0);
make_color!(GREEN, 0.0, 1.0, 0.0);
make_color!(GRAY, 0.5, 0.5, 0.5);

pub const BLACK: Color = Color::from_rgb(0.0, 0.0, 0.0);
pub const WHITE: Color = Color::from_rgb(1.0, 1.0, 1.0);
pub const YELLOW: Color = Color::from_rgb(1.0, 1.0, 0.0);

pub const BORDER_COLOR: Color = Color::from_rgb(0.3, 0.47, 1.0);

pub const DISABLED_COLOR: Color = GRAY;

pub const WINDOW_BACKGROUND: Color = Color::from_rgb(0.82, 0.82, 0.82);

#[derive(Clone, Copy, Debug, Default)]
pub enum ApplicationTheme {
    #[default]
    Light,
}

pub type Element<'a, Message> = iced::Element<'a, Message, Renderer<ApplicationTheme>>;
pub type Button<'a, Message> = widget::Button<'a, Message, Renderer<ApplicationTheme>>;
pub type Row<'a, Message> = widget::Row<'a, Message, Renderer<ApplicationTheme>>;
pub type Text<'a> = widget::Text<'a, Renderer<ApplicationTheme>>;

#[derive(Clone, Copy, Debug, Default)]
pub enum ButtonStyle {
    Red,
    Orange,
    Green,
    #[default]
    Gray,
}

impl button::StyleSheet for ApplicationTheme {
    type Style = ButtonStyle;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        let (background_color, text_color) = match style {
            ButtonStyle::Red => (RED, BLACK),
            ButtonStyle::Orange => (ORANGE, BLACK),
            ButtonStyle::Green => (GREEN, BLACK),

            ButtonStyle::Gray => (GRAY, BLACK),
        };

        let border_width = match style {
            ButtonStyle::Red | ButtonStyle::Orange | ButtonStyle::Green | ButtonStyle::Gray => 0.0,
        };

        let background = Some(Background::Color(background_color));

        let border_color = match style {
            ButtonStyle::Red | ButtonStyle::Orange | ButtonStyle::Green | ButtonStyle::Gray => {
                BORDER_COLOR
            }
        };

        button::Appearance {
            shadow_offset: Vector::default(),
            background,
            border_radius: BorderRadius::from(BORDER_RADIUS),
            border_width,
            border_color,
            text_color,
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        self.active(style)
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        let background_color = match style {
            ButtonStyle::Red => RED_PRESSED,
            ButtonStyle::Orange => ORANGE_PRESSED,
            ButtonStyle::Green => GREEN_PRESSED,
            ButtonStyle::Gray => GRAY_PRESSED,
        };

        button::Appearance {
            background: Some(Background::Color(background_color)),
            ..self.active(style)
        }
    }

    fn disabled(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(WINDOW_BACKGROUND)),
            border_color: DISABLED_COLOR,
            border_width: BORDER_WIDTH,
            text_color: DISABLED_COLOR,
            ..self.active(style)
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContainerStyle {
    #[default]
    Gray,
}

impl container::StyleSheet for ApplicationTheme {
    type Style = ContainerStyle;

    fn appearance(&self, style: &Self::Style) -> container::Appearance {
        match style {
            ContainerStyle::Gray => cont_style(GRAY, BLACK),
        }
    }
}

fn cont_style(bkgnd: Color, text: Color) -> container::Appearance {
    container::Appearance {
        text_color: Some(text),
        background: Some(Background::Color(bkgnd)),
        border_radius: BorderRadius::from(BORDER_RADIUS),
        border_width: 0.0,
        border_color: BORDER_COLOR,
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum TextStyle {
    #[default]
    Defualt,
    Yellow,
}

impl text::StyleSheet for ApplicationTheme {
    type Style = TextStyle;

    fn appearance(&self, style: Self::Style) -> text::Appearance {
        text::Appearance {
            color: match style {
                TextStyle::Defualt => None,
                TextStyle::Yellow => Some(YELLOW),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum SvgStyle {
    #[default]
    White,
}

impl svg::StyleSheet for ApplicationTheme {
    type Style = SvgStyle;

    fn appearance(&self, style: &Self::Style) -> svg::Appearance {
        let color = match style {
            SvgStyle::White => Some(WHITE),
        };
        svg::Appearance { color }
    }
}

impl scrollable::StyleSheet for ApplicationTheme {
    type Style = ();

    fn active(&self, _style: &Self::Style) -> scrollable::Scrollbar {
        scrollable::Scrollbar {
            background: None,
            border_radius: 2.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            scroller: scrollable::Scroller {
                color: GRAY,
                border_radius: 2.0.into(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
        }
    }

    fn hovered(
        &self,
        style: &Self::Style,
        _is_mouse_over_scrollbar: bool,
    ) -> scrollable::Scrollbar {
        self.active(style)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ApplicationStyle {}

impl application::StyleSheet for ApplicationTheme {
    type Style = ApplicationStyle;

    fn appearance(&self, _style: &Self::Style) -> application::Appearance {
        application::Appearance {
            background_color: WINDOW_BACKGROUND,
            text_color: BLACK,
        }
    }
}
