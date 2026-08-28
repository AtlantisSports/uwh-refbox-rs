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
        let half_width = width / 2;

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
                let key_i = (y * width + x + half_width) * 4;
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
