#[macro_use]
extern crate glium;

use std::io::{Cursor, Read};
mod pages;

pub struct Textures {
    atlantis_logo: glium::texture::SrgbTexture2d,
    bottom_graphic: glium::texture::SrgbTexture2d,
    team_information_graphic: glium::texture::SrgbTexture2d,
    team_black_graphic: glium::texture::SrgbTexture2d,
    team_white_graphic: glium::texture::SrgbTexture2d,
    team_bar_graphic: glium::texture::SrgbTexture2d,
    time_and_game_state_graphic: glium::texture::SrgbTexture2d,
    final_score_graphic: glium::texture::SrgbTexture2d,
}

impl Textures {
    fn init(display: &glium::Display) -> Self {
        let image = image::load(
            Cursor::new(&include_bytes!(
                "../../uwh-overlay-drawing/assets/1080/[PNG] 8K - Atlantis Logo.png"
            )),
            image::ImageFormat::Png,
        )
        .unwrap()
        .to_rgba8();
        let image_dimensions = image.dimensions();
        let image =
            glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);

        let atlantis_logo = glium::texture::SrgbTexture2d::new(display, image).unwrap();
        let image = image::load(
            Cursor::new(&include_bytes!(
                "../../uwh-overlay-drawing/assets/1080/[PNG] 8K - Bottom Graphic.png"
            )),
            image::ImageFormat::Png,
        )
        .unwrap()
        .to_rgba8();
        let image_dimensions = image.dimensions();
        let image =
            glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);

        let bottom_graphic = glium::texture::SrgbTexture2d::new(display, image).unwrap();
        let image = image::load(
            Cursor::new(&include_bytes!(
                "../../uwh-overlay-drawing/assets/1080/[PNG] 8K - Team Information Graphic.png"
            )),
            image::ImageFormat::Png,
        )
        .unwrap()
        .to_rgba8();
        let image_dimensions = image.dimensions();
        let image =
            glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);

        let team_information_graphic = glium::texture::SrgbTexture2d::new(display, image).unwrap();
        let image = image::load(
            Cursor::new(&include_bytes!(
                "../../uwh-overlay-drawing/assets/1080/[PNG] 8K - Team Bars Graphic.png"
            )),
            image::ImageFormat::Png,
        )
        .unwrap()
        .to_rgba8();
        let image_dimensions = image.dimensions();
        let image =
            glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);

        let team_bar_graphic = glium::texture::SrgbTexture2d::new(display, image).unwrap();
        let image = image::load(
            Cursor::new(&include_bytes!(
                "../../uwh-overlay-drawing/assets/1080/[PNG] 8K - Team White Graphic.png"
            )),
            image::ImageFormat::Png,
        )
        .unwrap()
        .to_rgba8();
        let image_dimensions = image.dimensions();
        let image =
            glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);

        let team_white_graphic = glium::texture::SrgbTexture2d::new(display, image).unwrap();
        let image = image::load(
            Cursor::new(&include_bytes!(
                "../../uwh-overlay-drawing/assets/1080/[PNG] 8K - Team Black Graphic.png"
            )),
            image::ImageFormat::Png,
        )
        .unwrap()
        .to_rgba8();
        let image_dimensions = image.dimensions();
        let image =
            glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);

        let team_black_graphic = glium::texture::SrgbTexture2d::new(display, image).unwrap();
        let image = image::load(
            Cursor::new(&include_bytes!(
                "../../uwh-overlay-drawing/assets/1080/[PNG] 8K - Time and Game State Graphic.png"
            )),
            image::ImageFormat::Png,
        )
        .unwrap()
        .to_rgba8();
        let image_dimensions = image.dimensions();
        let image =
            glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);

        let time_and_game_state_graphic =
            glium::texture::SrgbTexture2d::new(display, image).unwrap();
        let image = image::load(
            Cursor::new(&include_bytes!(
                "../../uwh-overlay-drawing/assets/1080/[PNG] 8K - Final Score Graphic.png"
            )),
            image::ImageFormat::Png,
        )
        .unwrap()
        .to_rgba8();
        let image_dimensions = image.dimensions();
        let image =
            glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);

        let final_score_graphic = glium::texture::SrgbTexture2d::new(display, image).unwrap();
        Self {
            atlantis_logo,
            bottom_graphic,
            team_information_graphic,
            team_black_graphic,
            team_white_graphic,
            team_bar_graphic,
            time_and_game_state_graphic,
            final_score_graphic,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

fn main() {
    use glium::{glutin, Surface};

    let event_loop = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new();
    let cb = glutin::ContextBuilder::new();
    let display = glium::Display::new(wb, cb, &event_loop).unwrap();
    let textures = Textures::init(&display);

    implement_vertex!(Vertex, position, tex_coords);

    let shape = vec![
        Vertex {
            position: [-0.5, 1.0],
            tex_coords: [0.0, 1.0],
        },
        Vertex {
            position: [1.5, 1.0],
            tex_coords: [1.0, 1.0],
        },
        Vertex {
            position: [-0.5, -1.0],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [1.5, 1.0],
            tex_coords: [1.0, 1.0],
        },
        Vertex {
            position: [-0.5, -1.0],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [1.5, -1.0],
            tex_coords: [1.0, 0.0],
        },
    ];

    let vertex_buffer = glium::VertexBuffer::new(&display, &shape).unwrap();
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

    let vertex_shader_src = r#"
        #version 140

        in vec2 position;
        in vec2 tex_coords;
        out vec2 v_tex_coords;

        uniform mat4 matrix;

        void main() {
            v_tex_coords = tex_coords;
            gl_Position = matrix * vec4(position, 0.0, 1.0);
        }
    "#;

    let fragment_shader_src = r#"
        #version 140

        in vec2 v_tex_coords;
        out vec4 color;

        uniform sampler2D tex;

        void main() {
            color = texture(tex, v_tex_coords);
        }
    "#;

    let program =
        glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None)
            .unwrap();

    event_loop.run(move |event, _, control_flow| {
        match event {
            glutin::event::Event::WindowEvent { event, .. } => match event {
                glutin::event::WindowEvent::CloseRequested => {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                    return;
                }
                _ => return,
            },
            glutin::event::Event::NewEvents(cause) => match cause {
                glutin::event::StartCause::ResumeTimeReached { .. } => (),
                glutin::event::StartCause::Init => (),
                _ => return,
            },
            _ => return,
        }

        let next_frame_time =
            std::time::Instant::now() + std::time::Duration::from_nanos(16_666_667);
        *control_flow = glutin::event_loop::ControlFlow::WaitUntil(next_frame_time);

        let mut target = display.draw();
        target.clear_color(1.0, 0.9, 0.9, 1.0);
        for uniform in pages::final_scores(&textures) {
            target
                .draw(
                    &vertex_buffer,
                    indices,
                    &program,
                    &uniform,
                    &glium::draw_parameters::DrawParameters {
                        blend: glium::draw_parameters::Blend::alpha_blending(),
                        ..Default::default()
                    },
                )
                .unwrap();
        }

        target.finish().unwrap();
    });
}

use serde_json::Value;
use std::net::TcpStream;
use uwh_common::game_snapshot::GameSnapshot;

fn _unused() -> Result<(), Box<dyn std::error::Error>> {
    let data: Value = serde_json::from_str(
        &reqwest::blocking::get("https://uwhscores.com/api/v1/tournaments")?.text()?,
    )?;
    let tournament = &data["tournaments"][0]["tid"];
    let data: Value = serde_json::from_str(
        &reqwest::blocking::get(format!(
            "https://uwhscores.com/api/v1/tournaments/{}",
            tournament
        ))?
        .text()?,
    )?;
    let division = &data["tournament"]["divisions"][0].as_str().unwrap();
    let data: Value = serde_json::from_str(
        &reqwest::blocking::get(format!(
            "https://uwhscores.com/api/v1/tournaments/{}/games?div={}",
            tournament, division
        ))?
        .text()?,
    )?;
    let game = &data["games"][0]["gid"];
    let data: Value = serde_json::from_str(
        &reqwest::blocking::get(format!(
            "https://uwhscores.com/api/v1/tournaments/{}/games/{}",
            tournament, game
        ))?
        .text()?,
    )?;
    println!("{}", serde_json::ser::to_string_pretty(&data)?);
    let mut stream = TcpStream::connect(("localhost", 8000))?;
    println!("connected!");
    let mut buff = vec![0u8; 1024];
    loop {
        let read_bytes = stream.read(&mut buff).unwrap();
        let snapshot: GameSnapshot = serde_json::de::from_slice(&buff[..read_bytes]).unwrap();

        print!("{:?}", snapshot);
    }
}
