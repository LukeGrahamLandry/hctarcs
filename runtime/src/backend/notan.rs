use notan::draw::*;
use notan::prelude::*;
use crate::backend::RenderBackend;
use crate::{Argb, RenderHandle, ScratchProgram, World};
use crate::builtins::{HALF_SCREEN_HEIGHT, HALF_SCREEN_WIDTH};
use crate::sprite::Trigger;
use std::borrow::Borrow;

#[derive(AppState)]
pub struct BackendImpl<S: ScratchProgram<BackendImpl<S>>> {
    state: State,
    world: World<S, BackendImpl<S>>,
}

pub struct State {
    texture: Texture,
    costumes: Vec<Texture>,
    bytes: Vec<u8>,
    stamps: Vec<(f32, f32, usize)>
}

pub struct Handle<'frame> {
    state: &'frame mut State,
    gfx: &'frame mut Graphics
}

impl<S: ScratchProgram<Self>> RenderBackend<S> for BackendImpl<S> {
    type Handle<'a> = Handle<'a>;

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

        let costumes = S::get_costumes().iter().map(|bytes| gfx.create_texture().from_image(bytes.borrow()).build().unwrap()).collect();

        let mut s = Self {
            state: State { texture, costumes, bytes, stamps: vec![] },
            world: World::new(),
        };

        let mut handle = Handle {
            state: &mut s.state,
            gfx,
        };

        // let start = Instant::now();
        s.world.broadcast(&mut handle, Trigger::FlagClicked);
        // println!("Handled Trigger::FlagClicked in {}ms.", (Instant::now() - start).as_millis());
        s
    }

    fn draw(gfx: &mut Graphics, state: &mut Self) {
        let mut _handle = Handle {
            state: &mut state.state,
            gfx,
        };

        // Update the texture with the new data
        gfx.update_texture(&mut state.state.texture)
            .with_data(&state.state.bytes)
            .update()
            .unwrap();

        let mut draw = gfx.create_draw();
        draw.clear(Color::BLACK);
        for (x, y, costume) in &state.state.stamps {
            let img = &state.state.costumes[1];
            let scale = 0.5; // 100.0 / img.width();
            draw
                .image(img)
                .position(*x + (img.size().0 * scale), *y + (img.size().1 * scale))
                // .position(*x, *y)
                .scale(scale, scale);

        }
        // TODO: the stamps need to be drawn onto this texture gradually instead.
        // There's an impl CreateDraw for RenderTexture
        // can i do like this but no shaders https://github.com/Nazariglez/notan/blob/main/examples/renderer_render_texture.rs
        draw.image(&state.state.texture);
        gfx.render(&draw);
    }


    fn update(app: &mut App, _state: &mut Self) {
        if app.keyboard.was_pressed(KeyCode::Escape) {
            app.exit();
        }
    }
}

impl<'a> RenderHandle for Handle<'a> {
    fn pen_pixel(&mut self, (x, y): (f64, f64), colour: Argb) {
        // TODO: underflow check
        let x = (x + HALF_SCREEN_WIDTH) as usize;
        let y = (HALF_SCREEN_HEIGHT - y) as usize;
        let i = (x + (y * (HALF_SCREEN_WIDTH as u32 * 2) as usize)) * 4;
        if i < self.state.bytes.len() {
            self.state.bytes[i..i + 4].copy_from_slice(&Color::from(colour).rgba_u8());
        }
    }

    fn pen_line(&mut self, line: crate::Line) {
        todo!()
    }

    fn pen_stamp(&mut self, (x, y): (f64, f64), costume: usize) {
        let x = (x + HALF_SCREEN_WIDTH) as f32;
        let y = (HALF_SCREEN_HEIGHT - y) as f32;
        assert!(costume < self.state.costumes.len());
        self.state.stamps.push((x, y, costume));
        println!("stamp {x}, {y} {costume}")
    }
}

// Aaaa i cant think about colour spaces.
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
