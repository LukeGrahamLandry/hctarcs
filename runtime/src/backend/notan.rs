use notan::draw::*;
use notan::prelude::*;
use crate::backend::RenderBackend;
use crate::{Argb, RenderHandle, ScratchProgram, World};
use crate::builtins::{HALF_SCREEN_HEIGHT, HALF_SCREEN_WIDTH};
use crate::sprite::Trigger;

#[derive(AppState)]
pub struct BackendImpl<S: ScratchProgram<BackendImpl<S>>> {
    state: State,
    world: World<S, BackendImpl<S>>,
}

pub struct State {
    texture: Texture,
    bytes: Vec<u8>,
}

impl<S: ScratchProgram<Self>> RenderBackend<S> for BackendImpl<S> {
    type Handle = State;

    fn run() {
        notan::init_with(BackendImpl::<S>::init)
            .add_config(DrawConfig)
            .draw(BackendImpl::draw)
            .update(BackendImpl::update)
            .build()
            .unwrap()
    }
}

// The notan callbacks arent methods, I just want them to be in the scope of the generic
impl<S: ScratchProgram<BackendImpl<S>>> BackendImpl<S> {
    fn init(app: &mut App, gfx: &mut Graphics) -> Self {
        app.window().set_size(2 * HALF_SCREEN_WIDTH as u32, 2 * HALF_SCREEN_HEIGHT as u32);
        app.window().set_title("Hctarcs");
        let (width, height) = app.window().size();
        let len = (width * height * 4) as usize;
        let bytes = vec![0; len];

        let texture = gfx
            .create_texture()
            .from_bytes(&bytes, width, height)
            .build()
            .unwrap();

        let mut s = Self {
            state: State { texture, bytes },
            world: World::new(),
        };

        // let start = Instant::now();
        s.world.broadcast(&mut s.state, Trigger::FlagClicked);
        // println!("Handled Trigger::FlagClicked in {}ms.", (Instant::now() - start).as_millis());
        s
    }

    fn draw(gfx: &mut Graphics, state: &mut Self) {
        let lines = state.world.bases.iter_mut().map(|sprite| sprite.lines.drain(0..)).flatten();

        for line in lines {
            // let len_sq = (line.start.0-line.end.0).powi(2) + (line.start.1-line.end.1).powi(2);
            // if len_sq <= 1.0 {

            let x = (line.start.0 + HALF_SCREEN_WIDTH) as usize;
            let y = (HALF_SCREEN_HEIGHT - line.start.1) as usize;
            let i = x + (y * (HALF_SCREEN_WIDTH as u32 * 2) as usize);
            // println!("{line:?}");
            if i > 0 && i < (state.state.bytes.len() / 4) {
                let i = i * 4;
                // Aaaa i cant think about colour spaces.
                state.state.bytes[i..i + 4].copy_from_slice(&Color::from(line.colour).rgba_u8());
            }
        }

        // Update the texture with the new data
        gfx.update_texture(&mut state.state.texture)
            .with_data(&state.state.bytes)
            .update()
            .unwrap();

        // Draw the texture using the draw 2d API for convenience
        let mut draw = gfx.create_draw();
        draw.clear(Color::BLACK);
        draw.image(&state.state.texture);
        gfx.render(&draw);
    }


    fn update(app: &mut App, _state: &mut Self) {
        if app.keyboard.was_pressed(KeyCode::Escape) {
            app.exit();
        }
    }
}


impl RenderHandle for State {

}

impl From<Argb> for Color {
    fn from(value: Argb) -> Self {
        Color::from_bytes(
            ((value.0 >> 16) & 255) as u8,
            ((value.0 >> 8) & 255) as u8,
            ((value.0 >> 0) & 255) as u8,
            255 - ((value.0 >> 24) & 255) as u8
        )
    }
}