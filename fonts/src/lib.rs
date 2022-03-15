#![no_std]

pub mod fonts {
    macro_rules! font {
        ($name:ident, $file:literal, $w:literal, $h:literal, $spacing:literal, $base:literal) => {
            pub const $name: embedded_graphics::mono_font::MonoFont =
                embedded_graphics::mono_font::MonoFont {
                    image: embedded_graphics::image::ImageRaw::new_binary(
                        core::include_bytes!(core::concat!(core::env!("OUT_DIR"), "/", $file)),
                        $w * 8,
                    ),
                    glyph_mapping: &embedded_graphics::mono_font::mapping::StrGlyphMapping::new(
                        " #-/0123456789:ABCDEFGHIJKLMNOPQRSTUVWXYZ[]_",
                        44,
                    ),
                    character_size: embedded_graphics::geometry::Size::new($w, $h),
                    character_spacing: $spacing,
                    baseline: $base,
                    underline:
                        embedded_graphics::mono_font::DecorationDimensions::default_underline($h),
                    strikethrough:
                        embedded_graphics::mono_font::DecorationDimensions::default_strikethrough(
                            $h,
                        ),
                };
        };
    }

    font!(FONT_5X8, "font_5x8.raw", 5, 8, 1, 7);
    font!(FONT_7X15, "font_7x15.raw", 7, 15, 1, 14);
    font!(FONT_10X25, "font_10x25.raw", 10, 25, 1, 24);
    font!(FONT_14X31, "font_14x31.raw", 14, 31, 2, 29);
    font!(FONT_20X46, "font_20x46.raw", 20, 46, 2, 44);
    font!(FONT_28X64, "font_28x64.raw", 28, 64, 4, 60);
}
