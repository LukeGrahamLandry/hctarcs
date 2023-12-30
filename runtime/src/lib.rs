use std::collections::VecDeque;
use std::env;
use std::num::NonZeroU32;
use winit::dpi::{PhysicalSize, Size};
use winit::event::{Event, KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowBuilder;
use crate::builtins::{HALF_SCREEN_HEIGHT, HALF_SCREEN_WIDTH};
use crate::sprite::{Sprite, SpriteBase, Trigger};

pub mod sprite;
pub mod builtins;
pub mod callback;
pub mod poly;

/// Types for Msg and Globals are generated for a specific scratch program by the compiler.
/// The default form of message must be FlagClicked
// TODO: needs to be a super of some builtin enum for other events the runtime needs to send
pub struct World<Msg: Clone + Copy, Globals> {
    pub bases: VecDeque<SpriteBase>,
    pub custom: VecDeque<Box<dyn Sprite<Msg, Globals>>>,
    pub globals: Globals,
    pub messages: VecDeque<Msg>
}

impl<Msg: Clone + Copy, Globals> World<Msg, Globals> {
    /// This function does not return until the program is over.
    pub fn run_program(globals: Globals, custom: Vec<Box<dyn Sprite<Msg, Globals>>>) {
        credits();
        let mut world = World {
            bases: vec![SpriteBase::default(); custom.len()].into(),
            custom: custom.into(),
            globals,
            messages: VecDeque::new(),
        };

        world.broadcast(Trigger::FlagClicked);

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

    // TODO: the compiler knows which messages each type wants to listen to.
    //       it could generate a separate array for each and have no virtual calls or traversing everyone on each message
    pub fn broadcast(&mut self, msg: Trigger<Msg>) {
        let sprites = self.bases.iter_mut().zip(self.custom.iter_mut());
        for (b, c) in sprites {
            c.receive(b, &mut self.globals, msg.clone());
        }
    }
}


fn credits() {
    if env::args().len() > 1 {
        println!("{}", CREDITS)
    }
}

const CREDITS: &str = r#"This program is compiled from a Scratch project using github.com/LukeGrahamLandry/hctarcs
All projects shared on the Scratch website are covered by the Creative Commons Attribution Share-Alike license.
Scratch is a project of the Scratch Foundation, in collaboration with the Lifelong Kindergarten Group at the MIT Media Lab. It is available for free at https://scratch.mit.edu
"#;
