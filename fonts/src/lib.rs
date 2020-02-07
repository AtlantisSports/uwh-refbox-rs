pub mod fonts {
    macro_rules! font {
        ($name:ident, $file:literal, $w:literal, $h:literal) => {
            paste::item! {
            #[derive(Debug, Copy, Clone, PartialEq, Eq)]
            pub struct [<$name Config>];

            impl embedded_graphics::fonts::font_builder::FontBuilderConf for [<$name Config>] {
                const FONT_IMAGE: &'static [u8] =
                    include_bytes!(concat!(env!("OUT_DIR"), "/", $file));
                const FONT_IMAGE_WIDTH: u32 = $w * 8;

                const CHAR_HEIGHT: u32 = $h;
                const CHAR_WIDTH: u32 = $w;

                fn char_offset(c: char) -> u32 {
                    match c {
                        ' ' => 0,
                        '#' => 1,
                        '-' => 2,
                        '/'..=':' => c as u32 - '/' as u32 + 3,
                        'A'..='Z' => c as u32 - 'A' as u32 + 15,
                        '[' => 41,
                        ']' => 42,
                        '_' => 43,
                        _ => 44,
                    }
                }
            }

            pub type $name<'a, C> = embedded_graphics::fonts::font_builder::FontBuilder<'a, C, [<$name Config>]>;
            }};
        }
    font!(Font6x8, "font_6x8.raw", 6, 8);
    font!(Font8x15, "font_8x15.raw", 8, 15);
    font!(Font11x25, "font_11x25.raw", 11, 25);
    font!(Font16x31, "font_16x31.raw", 16, 31);
    font!(Font22x46, "font_22x46.raw", 22, 46);
    font!(Font32x64, "font_32x64.raw", 32, 64);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
