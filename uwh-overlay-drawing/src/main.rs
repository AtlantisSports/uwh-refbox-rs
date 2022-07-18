#[macro_use]
extern crate glium;

use std::{
    io::{Cursor, Read},
    time,
};
mod pages;

pub struct Textures {
    atlantis_logo_graphic: glium::texture::SrgbTexture2d,
    bottom_graphic: glium::texture::SrgbTexture2d,
    team_information_graphic: glium::texture::SrgbTexture2d,
    team_black_graphic: glium::texture::SrgbTexture2d,
    team_white_graphic: glium::texture::SrgbTexture2d,
    team_bar_graphic: glium::texture::SrgbTexture2d,
    time_and_game_state_graphic: glium::texture::SrgbTexture2d,
    final_score_graphic: glium::texture::SrgbTexture2d,
}

macro_rules! load {
    ($file:literal, $display:ident, $textures: ident) => {
        let image = image::load(Cursor::new(&include_bytes!($file)), image::ImageFormat::Png)
            .unwrap()
            .to_rgba8();
        let image_dimensions = image.dimensions();
        let image =
            glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
        $textures.push(glium::texture::SrgbTexture2d::new($display, image).unwrap());
    };
}

impl Textures {
    fn init(display: &glium::Display) -> Self {
        let start = time::Instant::now();
        let mut textures = Vec::new();
        load!(
            "../../uwh-overlay-drawing/assets/1080/[PNG] 8K - Final Score Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/1080/[PNG] 8K - Time and Game State Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/1080/[PNG] 8K - Team Black Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/1080/[PNG] 8K - Team White Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/1080/[PNG] 8K - Team Bars Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/1080/[PNG] 8K - Team Information Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/1080/[PNG] 8K - Bottom Graphic.png",
            display,
            textures
        );
        load!(
            "../../uwh-overlay-drawing/assets/1080/[PNG] 8K - Atlantis Logo.png",
            display,
            textures
        );
        println!(
            "Loaded images in: {} seconds",
            start.elapsed().as_secs_f32()
        );
        Self {
            atlantis_logo_graphic: textures.pop().unwrap(),
            bottom_graphic: textures.pop().unwrap(),
            team_information_graphic: textures.pop().unwrap(),
            team_black_graphic: textures.pop().unwrap(),
            team_white_graphic: textures.pop().unwrap(),
            team_bar_graphic: textures.pop().unwrap(),
            time_and_game_state_graphic: textures.pop().unwrap(),
            final_score_graphic: textures.pop().unwrap(),
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

    let mut game_state: Option<GameSnapshot> = None;
    let (tx, rx) = std::sync::mpsc::channel::<GameSnapshot>();

    std::thread::spawn(|| networking_thread(tx).unwrap());
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

        if let Ok(state) = rx.try_recv() {
            game_state = Some(state);
        }
        //TODO fix this to depend on FPS
        let next_frame_time =
            std::time::Instant::now() + std::time::Duration::from_nanos(16_666_667);
        *control_flow = glutin::event_loop::ControlFlow::WaitUntil(next_frame_time);

        let mut target = display.draw();
        target.clear_color(1.0, 0.9, 0.9, 1.0);
        if game_state.is_some() {
            for uniform in if game_state.as_ref().unwrap().current_period
                == uwh_common::game_snapshot::GamePeriod::BetweenGames
            {
                match game_state.as_ref().unwrap().secs_in_period {
                    121..=u16::MAX => pages::next_game(&textures),
                    30..=120 => pages::roster(&textures),
                    _ => pages::pre_game_display(&textures),
                }
            } else {
                pages::final_scores(&textures)
            } {
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
        }

        target.finish().unwrap();
    });
}

use serde_json::Value;
use std::net::TcpStream;
use uwh_common::game_snapshot::GameSnapshot;

fn networking_thread(
    tx: std::sync::mpsc::Sender<GameSnapshot>,
) -> Result<(), Box<dyn std::error::Error>> {
    //let data: Value = serde_json::from_str(
    //    &reqwest::blocking::get("https://uwhscores.com/api/v1/tournaments")?.text()?,
    //)?;
    //let tournament = &data["tournaments"][0]["tid"];
    //let data: Value = serde_json::from_str(
    //    &reqwest::blocking::get(format!(
    //        "https://uwhscores.com/api/v1/tournaments/{}",
    //        tournament
    //    ))?
    //    .text()?,
    //)?;
    //let division = &data["tournament"]["divisions"][0].as_str().unwrap();
    //let data: Value = serde_json::from_str(
    //    &reqwest::blocking::get(format!(
    //        "https://uwhscores.com/api/v1/tournaments/{}/games?div={}",
    //        tournament, division
    //    ))?
    //    .text()?,
    //)?;
    //let game = &data["games"][0]["gid"];
    //let data: Value = serde_json::from_str(
    //    &reqwest::blocking::get(format!(
    //        "https://uwhscores.com/api/v1/tournaments/{}/games/{}",
    //        tournament, game
    //    ))?
    //    .text()?,
    //)?;
    let mut stream = TcpStream::connect(("localhost", 8000))
        .expect("Is the refbox running? We error'd out on the connection.");
    let mut buff = vec![0u8; 1024];
    loop {
        let read_bytes = stream.read(&mut buff).unwrap();
        let snapshot: GameSnapshot = serde_json::de::from_slice(&buff[..read_bytes]).unwrap();

        tx.send(snapshot).unwrap();
    }
}
