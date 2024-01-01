use std::time::Instant;
use notan::draw::*;
use notan::prelude::*;
use crate::backend::RenderBackend;
use crate::{ScratchProgram, World};
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
        notan::init_with(init::<S>)
            .add_config(DrawConfig)
            .draw(draw::<S>)
            .build()
            .unwrap()
    }
}

fn init<S: ScratchProgram<BackendImpl<S>>>(app: &mut App, gfx: &mut Graphics) -> BackendImpl<S> {
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

    let mut s = BackendImpl {
        state: State { texture, bytes },
        world: World::new(),
    };

    let start = Instant::now();
    s.world.broadcast(&mut s.state, Trigger::FlagClicked);
    println!("Handled Trigger::FlagClicked in {}ms.", (Instant::now() - start).as_millis());
    s
}

fn draw<S: ScratchProgram<BackendImpl<S>>>(gfx: &mut Graphics, state: &mut BackendImpl<S>) {

    // TODO: need to not be cloning. but that's easy since i'll be draining the lines after render anyway. need to do it gradually in the ctx?
    let lines = state.world.bases.iter().map(|sprite| sprite.lines.iter()).flatten().collect::<Vec<_>>();

    for line in &lines {
        // let len_sq = (line.start.0-line.end.0).powi(2) + (line.start.1-line.end.1).powi(2);
        // if len_sq <= 1.0 {
        let x = (line.start.0 + HALF_SCREEN_WIDTH) as usize;
        let y = (HALF_SCREEN_HEIGHT - line.start.1) as usize;
        let i = x + (y * (HALF_SCREEN_WIDTH as u32 * 2) as usize);
        // println!("{line:?}");
        if i > 0 && i < (state.state.bytes.len() / 4) {
            let i = i * 4;
            // Aaaa i cant think about colour spaces.
            let c = Color::from_bytes(
                ((line.colour.0 >> 16) & 255) as u8,
                ((line.colour.0 >> 8) & 255) as u8,
                ((line.colour.0 >> 0) & 255) as u8,
                255 - ((line.colour.0 >> 24) & 255) as u8
            );
            state.state.bytes[i..i + 4].copy_from_slice(&c.rgba_u8());
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
