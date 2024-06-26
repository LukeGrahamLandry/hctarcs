use std::marker::PhantomData;
use macroquad::prelude::*;
use crate::{Argb, args, HALF_SCREEN_HEIGHT, HALF_SCREEN_WIDTH, Line, RenderBackend, RenderHandle, ScratchProgram, Trigger, World};
use std::ops::{Div, Mul};
use std::process::exit;

pub struct BackendImpl<S: ScratchProgram<Self>>(PhantomData<S>);

pub struct Handle {
    costumes: Vec<Texture2D>
}

impl<S: ScratchProgram<BackendImpl<S>>> RenderBackend<S> for BackendImpl<S> {
    type Handle<'a> = Handle;

    fn run() {
        // TODO: resizable so debugger is less painful
        // TODO: dont include padding if not inspect mode.
        let (window_width, window_height) = ((HALF_SCREEN_WIDTH * 2.0) as i32 + 400, (HALF_SCREEN_HEIGHT * 2.0) as i32 + 200);
        macroquad::Window::from_config(Conf {
            window_title: "Hctarcs: macroquad".to_string(),
            window_width,
            window_height,
            high_dpi: true,
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


        #[cfg(feature = "inspect")]
        let mut debugger = crate::ui::Debugger::<S, Self>::new();

        let costumes = S::get_costumes()
            .iter()
            .map(|a| a.get(|bytes| Texture2D::from_file_with_format(bytes, None))).collect();

        let mut handle = Handle { costumes };
        world.broadcast_toplevel_async(Trigger::FlagClicked);

        // TODO: move logic out of backend.
        // TODO: sad allocation noises. I guess you can't slice an OsStr (cstr?)?
        // TODO: sad that i check this every frame
        let take_screenshot = args().any(|arg| &arg == "--first-frame-only");

        loop {
            // println!("Frame:");

            // All the draw commands during an event are to the static pen texture.
            set_camera(&pen_camera);
            world.run_frame(&mut handle);
            set_default_camera();


            clear_background(GRAY);
            // TODO: dynamic scratch window size and scale
            draw_rectangle(0.0, 0.0, (HALF_SCREEN_WIDTH * 2.0) as f32, (HALF_SCREEN_HEIGHT * 2.0) as f32, WHITE);

            draw_texture(pen.texture,0.0, 0.0, WHITE);
            for sprite in &world.bases {
                // TODO: fix wierd coordinate space
                // println!("{:?}", sprite);
                if !sprite.hidden {
                    handle.pen_stamp((sprite.x + HALF_SCREEN_WIDTH, sprite.y - HALF_SCREEN_HEIGHT), sprite.costume, sprite.size_frac);
                }
            }

            if is_key_down(KeyCode::Escape) {
                exit(0);
            }
            if take_screenshot {
                let img = get_screen_data();
                img.export_png("frame.png");
                #[cfg(not(target_arch = "wasm32"))]
                {
                    println!("Exiting. Saved first frame at {}/frame.png", std::env::current_dir().unwrap().to_string_lossy());
                }

                exit(0);
            }

            #[cfg(feature = "inspect")]
            {
                egui_macroquad::ui(|ctx| debugger.frame(ctx, &mut world));
                egui_macroquad::draw();
            }

            next_frame().await;
        }
    }
}

// TODO: why is drawing on the different camera in a different coordinate space?
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

    fn pen_stamp(&mut self, (x, y): (f64, f64), costume: usize, size: f64) {
        let x = x as f32;
        let y = -y as f32;
        // TODO: correct starting costume. shouldn't be in the backend tho
        let wh = vec2(self.costumes[costume].width(), self.costumes[costume].height());
        let size = wh.div(2.0).mul(size as f32);
        draw_texture_ex(self.costumes[costume], x - (size.x / 2.0), y - (size.y / 2.0), WHITE, DrawTextureParams {
            dest_size: Some(size),
            source: None,
            rotation: 0.0,
            flip_x: false,
            flip_y: false,
            pivot: None,
        });
    }

    // TODO: positioning is nothing like real scratch
    // TODO: speech bubble
    fn say(&mut self, text: &str, (x, y): (f64, f64)) {
        let x = x as f32;
        let y = -y as f32;
        let font_size = 40;
        let size = measure_text(text, None, font_size, 1.0);
        draw_text(text, x - (size.width / 2.0), y - (size.height / 2.0), font_size as f32, BLACK);
    }

    // TODO: make this an IoAction instead of a context method so it can finish drawing the current frame.
    fn save_frame(&mut self, path: &str) {
        get_screen_data().export_png(path);
    }

    fn pen_clear(&mut self) {
        clear_background(WHITE);
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
