use std::marker::PhantomData;
use macroquad::miniquad::window::request_quit;
use macroquad::prelude::*;
use crate::{Argb, HALF_SCREEN_HEIGHT, HALF_SCREEN_WIDTH, Line, RenderBackend, RenderHandle, ScratchProgram, Trigger, World};

pub struct BackendImpl<S: ScratchProgram<Self>>(PhantomData<S>);

pub struct Handle();

impl<S: ScratchProgram<BackendImpl<S>>> RenderBackend<S> for BackendImpl<S> {
    type Handle<'a> = Handle;

    fn run() {
        let (window_width, window_height) = ((HALF_SCREEN_WIDTH * 2.0) as i32, (HALF_SCREEN_HEIGHT * 2.0) as i32);
        macroquad::Window::from_config(Conf {
            window_title: "Hctarcs".to_string(),
            window_width,
            window_height,
            ..Default::default()
        }, Self::inner());
    }
}

impl<S: ScratchProgram<BackendImpl<S>>> BackendImpl<S> {
    async fn inner() {
        let (width, height) = ((HALF_SCREEN_WIDTH * 2.0) as u32, (HALF_SCREEN_HEIGHT * 2.0) as u32);
        let mut world = World::<S, Self>::new();
        let pen = render_target(width, height);
        pen.texture.set_filter(FilterMode::Nearest);
        let pen_camera = Camera2D {
            render_target: Some(pen.clone()),
            zoom: vec2(1.0 / HALF_SCREEN_WIDTH as f32, 1.0 / HALF_SCREEN_HEIGHT as f32),
            ..Default::default()
        };
        let mut handle = Handle {};

        // All the draw commands during an event are to the static pen texture.
        set_camera(&pen_camera);
        world.broadcast(&mut handle, Trigger::FlagClicked);
        set_default_camera();

        loop {
            clear_background(BLACK);
            draw_texture(&pen.texture,0.0, 0.0, WHITE);

            if is_key_down(KeyCode::Escape) {
                request_quit();
            }
            next_frame().await;
        }
    }
}

impl RenderHandle for Handle {
    fn pen_pixel(&mut self, (x, y): (f64, f64), colour: Argb) {
        let x = x as f32;
        let y = -y as f32;
        // TODO: can i access the frame buffer to draw a single pixel? i guess gpus dont like that.
        draw_line(x, y, x + 1.0, y + 1.0, 1.0, colour.into());
    }

    fn pen_line(&mut self, _line: Line) {
        println!("TODO: pen_line")
    }

    fn pen_stamp(&mut self, _pos: (f64, f64), _costume: usize) {
        println!("TODO: pen_stamp")
    }
}


impl From<Argb> for Color {
    fn from(value: Argb) -> Self {
        Color::from_rgba(
            ((value.0 >> 16) & 255) as u8,
            ((value.0 >> 8) & 255) as u8,
            ((value.0 >> 0) & 255) as u8,
            255 - ((value.0 >> 24) & 255) as u8
        )
    }
}
