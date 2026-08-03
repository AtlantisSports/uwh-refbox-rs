//! The "CMAS Official" scoresheet style.
//!
//! Unlike the other styles, the whole page below the title band is a single HTML
//! table of 25 master columns, so every block aligns to shared column boundaries.
//! See `docs/superpowers/specs/2026-08-02-cmas-official-scoresheet-design.md`.

/// Render one game's CMAS Official scoresheet page.
///
/// Returns a complete standalone HTML document. The caller extracts the `<style>`
/// block and the `<div class='page'>` fragment to build the combined PDF.
#[allow(clippy::too_many_arguments)]
pub fn render_html_cmas_official() -> String {
    // Replaced with the real grid in a later task.
    String::from(
        "<!doctype html><html><head><meta charset='utf-8'/><style></style></head>\
         <body><div class='page'></div></body></html>",
    )
}
