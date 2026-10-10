use grafton_ndi::{Error, NDI, PixelFormat, Sender, SenderOptions, VideoFrame};
use log::warn;
use macroquad::{
    camera::{Camera2D, set_camera},
    color::WHITE,
    material::{Material, MaterialParams, gl_use_default_material, gl_use_material, load_material},
    math::{Rect, vec2},
    miniquad::ShaderSource,
    texture::{
        DrawTextureParams, FilterMode, Image, RenderTarget, Texture2D, draw_texture_ex,
        render_target,
    },
    window::get_internal_gl,
};

/// Size of the picture sent over NDI: one half of the 3840x1080 canvas.
const OUT_WIDTH: u32 = 1920;
const OUT_HEIGHT: u32 = 1080;

/// Sends the overlay's rendered picture out as an NDI stream with real per-pixel
/// transparency, so vMix (or any other NDI-aware software) can use it as a clean
/// overlay layer instead of screen-capturing the window.
///
/// The picture it sends is made by [`KeyCombiner`] from the existing left(color)/right(grayscale
/// key) canvas described in `pages::mod`'s `draw_texture_both!` macro, so the existing rendering
/// and asset pipeline (`load_images.rs`, `alphagen`) is untouched.
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

    /// `picture` must be the finished RGBA picture from [`KeyCombiner::combine`].
    pub fn send_frame(&mut self, picture: &Image) {
        let (width, height) = (usize::from(picture.width), usize::from(picture.height));

        let mut frame = match VideoFrame::builder()
            .resolution(width as i32, height as i32)
            .pixel_format(PixelFormat::RGBA)
            .frame_rate(60, 1)
            .build()
        {
            Ok(frame) => frame,
            Err(e) => {
                warn!("Failed to build NDI video frame: {e}");
                return;
            }
        };

        let dst = frame.data_mut();
        let Some(src) = picture.bytes.get(..dst.len()) else {
            warn!(
                "NDI picture is {} bytes, expected {}; frame skipped",
                picture.bytes.len(),
                dst.len()
            );
            return;
        };
        dst.copy_from_slice(src);

        self.sender.send_video(&frame);
    }
}

/// Turns the 3840x1080 canvas (color graphics in the left half, the matching grayscale alpha
/// key in the right half) into one 1920x1080 RGBA picture with real transparency, on the
/// graphics card.
///
/// Doing this per pixel on the processor, after copying the whole canvas back from the
/// graphics card, kept a CPU core almost fully busy at 60 frames a second. On the graphics
/// card it costs next to nothing, and only the finished picture (half the size) is copied back.
pub struct KeyCombiner {
    target: RenderTarget,
    camera: Camera2D,
    material: Material,
}

impl KeyCombiner {
    pub fn new() -> Result<Self, macroquad::Error> {
        let material = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX_SHADER,
                fragment: FRAGMENT_SHADER,
            },
            // The default pipeline doesn't blend, so the alpha written is exactly the key's
            // value instead of being mixed with what was in the target before.
            MaterialParams::default(),
        )?;
        let target = render_target(OUT_WIDTH, OUT_HEIGHT);
        target.texture.set_filter(FilterMode::Nearest);
        // The same y-flip as the canvas camera in `main.rs`, for the same reason.
        let mut camera = Camera2D::from_display_rect(Rect::new(
            0.,
            OUT_HEIGHT as f32,
            OUT_WIDTH as f32,
            -(OUT_HEIGHT as f32),
        ));
        camera.render_target = Some(target.clone());
        Ok(Self {
            target,
            camera,
            material,
        })
    }

    /// The finished picture for NDI. Leaves the camera and material changed; the caller sets
    /// its own camera afterwards.
    pub fn combine(&self, canvas: &Texture2D) -> Image {
        set_camera(&self.camera);
        gl_use_material(&self.material);
        draw_texture_ex(
            canvas,
            0.,
            0.,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(OUT_WIDTH as f32, OUT_HEIGHT as f32)),
                ..Default::default()
            },
        );
        gl_use_default_material();
        // Without this, `get_texture_data` below can read the target before this frame's
        // batched draw calls have actually been submitted to the GPU -- the same reason
        // macroquad's own `get_screen_data()` flushes before its own read.
        unsafe {
            get_internal_gl().flush();
        }
        self.target.texture.get_texture_data()
    }
}

/// macroquad's standard vertex shader, with full-precision texture coordinates: the canvas is
/// 3840 pixels wide, more than low precision can address exactly.
const VERTEX_SHADER: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
attribute vec4 normal;

varying highp vec2 uv;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    uv = texcoord;
}"#;

/// Color from the left half of the canvas, alpha from the same spot in the right half. The key
/// half is white-on-black per pixel alpha (see `alphagen`), so its red value is the alpha.
const FRAGMENT_SHADER: &str = r#"#version 100
precision highp float;
varying highp vec2 uv;

uniform sampler2D Texture;

void main() {
    vec3 color = texture2D(Texture, vec2(uv.x * 0.5, uv.y)).rgb;
    float alpha = texture2D(Texture, vec2(uv.x * 0.5 + 0.5, uv.y)).r;
    gl_FragColor = vec4(color, alpha);
}"#;
