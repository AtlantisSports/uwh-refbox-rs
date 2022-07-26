use crate::load_images::TexturesAlpha;
use crate::load_images::TexturesColor;
use crate::pages;
use crate::GameSnapshot;
use glium::implement_vertex;
use std::sync::mpsc::Receiver;

#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

macro_rules! call_twice {
    ($func:expr, $arg1:expr, $arg2:expr) => {
        ($func($arg1), $func($arg2))
    };
}

pub fn rendering_thread(rx: Receiver<crate::GameSnapshot>) {
    use glium::{glutin, Surface};

    let event_loop = glutin::event_loop::EventLoop::new();

    let display1 = glium::Display::new(
        glutin::window::WindowBuilder::new(),
        glutin::ContextBuilder::new(),
        &event_loop,
    )
    .unwrap();

    let display2 = glium::Display::new(
        glutin::window::WindowBuilder::new(),
        glutin::ContextBuilder::new(),
        &event_loop,
    )
    .unwrap();

    let textures_color = TexturesColor::init(&display1);
    let textures_alpha = TexturesAlpha::init(&display2);

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

    let vertex_buffer1 = glium::VertexBuffer::new(&display1, &shape).unwrap();
    let vertex_buffer2 = glium::VertexBuffer::new(&display2, &shape).unwrap();

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

    let program1 =
        glium::Program::from_source(&display1, vertex_shader_src, fragment_shader_src, None)
            .unwrap();
    let program2 =
        glium::Program::from_source(&display2, vertex_shader_src, fragment_shader_src, None)
            .unwrap();

    let system = glium_text_rusttype::TextSystem::new(&display1);

    let font = glium_text_rusttype::FontTexture::new(
        &display1,
        &include_bytes!("../BAHNSCHRIFT.TTF")[..],
        50,
        glium_text_rusttype::FontTexture::ascii_character_list(),
    )
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
        let next_frame_time =
            std::time::Instant::now() + std::time::Duration::from_nanos(16_666_667);
        *control_flow = glutin::event_loop::ControlFlow::WaitUntil(next_frame_time);

        let mut target1 = display1.draw();
        let mut target2 = display2.draw();
        target1.clear_color(1.0, 0.9, 0.9, 1.0);
        target2.clear_color(0.0, 0.0, 0.0, 1.0);
        if game_state.is_some() {
            // let (uniforms_color, uniforms_alpha) =
            //     call_twice!(pages::next_game, &textures_color, &textures_alpha);
            let uniforms_color = pages::next_game(&textures_color);

            // if game_state.as_ref().unwrap().current_period
            //     == uwh_common::game_snapshot::GamePeriod::BetweenGames
            // {
            //     match game_state.as_ref().unwrap().secs_in_period {
            //         121..=u16::MAX => {
            //             call_twice!(pages::next_game, &textures_color, &textures_alpha)
            //         }
            //         30..=120 => call_twice!(pages::roster, &textures_color, &textures_alpha),
            //         _ => call_twice!(pages::pre_game_display, &textures_color, &textures_alpha),
            //     }
            // } else {
            //     call_twice!(pages::final_scores, &textures_color, &textures_alpha)
            // };

            //draw image overlays to color feed
            for uniform in uniforms_color.0 {
                target1
                    .draw(
                        &vertex_buffer1,
                        indices,
                        &program1,
                        &uniform,
                        &glium::draw_parameters::DrawParameters {
                            blend: glium::draw_parameters::Blend::alpha_blending(),
                            ..Default::default()
                        },
                    )
                    .unwrap();
            }

            //draw image overlays to alpha feed
            // for uniform in uniforms_alpha.0 {
            //     target2
            //         .draw(
            //             &vertex_buffer2,
            //             indices,
            //             &program2,
            //             &uniform,
            //             &glium::draw_parameters::DrawParameters {
            //                 blend: glium::draw_parameters::Blend::alpha_blending(),
            //                 ..Default::default()
            //             },
            //         )
            //         .unwrap();
            // }

            //draw text to color feed
            for mut text_params in uniforms_color.1 {
                let text = glium_text_rusttype::TextDisplay::new(&system, &font, text_params.text);
                let (w, h) = display1.get_framebuffer_dimensions();
                let text_width = text.get_width();
                text_params.matrix[0][0] /= text_width;
                text_params.matrix[1][1] =
                    text_params.matrix[1][1] * (w as f32) / (h as f32) / text_width;
                println!("xoffset: {}", text_params.matrix[3][0]);
                glium_text_rusttype::draw(
                    &text,
                    &system,
                    &mut target1,
                    text_params.matrix,
                    text_params.color,
                )
                .unwrap();
            }

            //draw text to alpha feed
            // for mut text_params in uniforms_alpha.1 {
            //     let text = glium_text_rusttype::TextDisplay::new(&system, &font, text_params.text);
            //     let (w, h) = display1.get_framebuffer_dimensions();
            //     let text_width = text.get_width();
            //     text_params.matrix[0][0] /= text_width;
            //     text_params.matrix[1][1] /= text_width * (w as f32 / h as f32);
            //     glium_text_rusttype::draw(
            //         &text,
            //         &system,
            //         &mut target1,
            //         text_params.matrix,
            //         text_params.color,
            //     )
            //     .unwrap();
            // }
        }

        target1.finish().unwrap();
        target2.finish().unwrap();
    });
}
