use crate::load_images::Textures;
use crate::pages;
use crate::GameSnapshot;
use glium::implement_vertex;
use std::sync::mpsc::Receiver;

#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

pub fn rendering_thread(rx: Receiver<crate::GameSnapshot>) {
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
