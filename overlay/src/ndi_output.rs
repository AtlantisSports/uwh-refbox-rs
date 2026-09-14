use grafton_ndi::{Error, NDI, PixelFormat, Sender, SenderOptions, VideoFrame};
use log::warn;
use macroquad::texture::Image;

/// Sends the overlay's rendered picture out as an NDI stream with real per-pixel
/// transparency, so vMix (or any other NDI-aware software) can use it as a clean
/// overlay layer instead of screen-capturing the window.
///
/// The rendered window is the existing left(color)/right(grayscale key) pair described
/// in `pages::mod`'s `draw_texture_both!` macro. This combines that pair into one true
/// RGBA image rather than changing how anything is drawn, so the existing rendering and
/// asset pipeline (`load_images.rs`, `alphagen`) is untouched.
pub struct NdiOutput {
    _ndi: NDI,
    sender: Sender,
}

impl NdiOutput {
    pub fn new(stream_name: &str) -> Result<Self, Error> {
        let ndi = NDI::new()?;
        let options = SenderOptions::builder(stream_name)
            .clock_video(true)
            .build();
        let sender = Sender::new(&ndi, &options)?;
        Ok(Self { _ndi: ndi, sender })
    }

    /// `screen` must be the full rendered window (color graphics in the left half, the
    /// matching grayscale alpha key in the right half, side by side, per
    /// `window_conf()`'s 3840x1080 size). Only the combined, half-width picture is sent.
    pub fn send_frame(&mut self, screen: &Image) {
        let width = screen.width as usize;
        let height = screen.height as usize;

        // Every draw call in `pages::mod` (the `draw_texture_both!`/`draw_text_both!` family)
        // puts the key half at a *fixed* logical x-offset of 1920 -- that is not the same
        // number as `width / 2` unless the window actually rendered at exactly the 3840 it
        // asked for in `window_conf()`. On a machine where the OS clamps the window to a
        // narrower size (a display that doesn't have 3840 pixels available), `width / 2` is
        // smaller than 1920, and reading the key half at that wrong split silently samples
        // alpha from the wrong pixels -- shifted left by however far short of 3840 the window
        // actually is, so a transparent gap ends up borrowing the opacity of whatever
        // neighbouring graphic happens to sit at the miscalculated offset instead. That is
        // the bug behind a decorative shape's negative space rendering as solid black in vMix
        // instead of see-through, even though the source assets' own alpha is correct.
        const KEY_OFFSET: usize = 1920;
        if width < KEY_OFFSET * 2 {
            warn!(
                "the overlay window rendered at {width}x{height}, narrower than the {} it needs \
                 for a correct color+key split -- the NDI picture is missing its rightmost {} \
                 pixels of key data this frame",
                KEY_OFFSET * 2,
                KEY_OFFSET * 2 - width
            );
        }
        // Never more than 1920 (the canvas width every page's layout assumes), and never more
        // than what the window actually captured on the key side -- reading past `width` would
        // be an out-of-bounds panic, and there is no key data at all for a column the window
        // was too narrow to render.
        let half_width = KEY_OFFSET.min(width.saturating_sub(KEY_OFFSET));

        let mut frame = match VideoFrame::builder()
            .resolution(half_width as i32, height as i32)
            .pixel_format(PixelFormat::BGRA)
            .frame_rate(60, 1)
            .build()
        {
            Ok(frame) => frame,
            Err(e) => {
                warn!("Failed to build NDI video frame: {e}");
                return;
            }
        };

        let src = &screen.bytes;
        let dst = frame.data_mut();

        for y in 0..height {
            for x in 0..half_width {
                let color_i = (y * width + x) * 4;
                let key_i = (y * width + x + KEY_OFFSET) * 4;
                let out_i = (y * half_width + x) * 4;

                let r = src[color_i];
                let g = src[color_i + 1];
                let b = src[color_i + 2];
                // The key half is white-on-black per pixel alpha (see `alphagen`), so
                // its rendered lightness already equals the original alpha value.
                let alpha = src[key_i];

                dst[out_i] = b;
                dst[out_i + 1] = g;
                dst[out_i + 2] = r;
                dst[out_i + 3] = alpha;
            }
        }

        self.sender.send_video(&frame);
    }
}
