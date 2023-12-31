use std::num::NonZeroU32;
use winit::dpi::{PhysicalSize, Size};
use winit::event::{Event, KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowBuilder;
use crate::backend::RenderBackend;
use crate::builtins::{HALF_SCREEN_HEIGHT, HALF_SCREEN_WIDTH};
use crate::sprite::Trigger;
use crate::World;

pub struct SoftBackend();

impl RenderBackend for SoftBackend {
    fn run<M: Copy, G>(mut world: World<M, G, Self>) {
        let mut this = SoftBackend();
        world.broadcast(&mut this, Trigger::FlagClicked);

        for sprite in &world.bases {
            println!("Drew {:?} lines.", sprite.lines.len());
        }

        // TODO: need to not be cloning. but that's easy since i'll be draining the lines after render anyway. render sprite each on separate layer?
        let lines = world.bases.iter().map(|sprite| sprite.lines.iter()).flatten().collect::<Vec<_>>();

        let event_loop = EventLoop::new().unwrap();
        let builder = WindowBuilder::new().with_title("hctarcs");
        let window = builder.build(&event_loop).unwrap();
        // window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
        let _ = window.request_inner_size(Size::Physical(PhysicalSize::new(480, 360)));
        let context = softbuffer::Context::new(&window).unwrap();
        let mut surface = softbuffer::Surface::new(&context, &window).unwrap();

        event_loop.set_control_flow(ControlFlow::Wait);  // Poll
        event_loop.run(|event, elwt| {
            match event {
                Event::AboutToWait => {
                    // window.request_redraw()
                },

                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                }  => elwt.exit(),

                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => {
                    let (width, height) = {
                        let size = window.inner_size();
                        (size.width, size.height)
                    };
                    surface
                        .resize(
                            NonZeroU32::new(width).unwrap(),
                            NonZeroU32::new(height).unwrap(),
                        )
                        .unwrap();

                    let mut buffer = surface.buffer_mut().unwrap();
                    for line in &lines {
                        // let len_sq = (line.start.0-line.end.0).powi(2) + (line.start.1-line.end.1).powi(2);
                        // if len_sq <= 1.0 {
                        let x = (line.start.0 + HALF_SCREEN_WIDTH) as usize;
                        let y = (HALF_SCREEN_HEIGHT - line.start.1) as usize;
                        let i = x + (y * width as usize);
                        if i > 0 && i < buffer.len() {
                            buffer[i] = line.colour.0;
                        }
                        // }
                    }
                    buffer.present().unwrap();
                }

                Event::WindowEvent { event:  WindowEvent::KeyboardInput {
                    event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        ..
                    },
                    ..
                }, ..
                } => elwt.exit(),
                _ => {}
            }
        }).unwrap();
    }
}
