use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::num::NonZeroU32;
use softbuffer::{Buffer, Surface};
use winit::dpi::{PhysicalSize, Size};
use winit::event::{Event, KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowBuilder};
use crate::backend::RenderBackend;
use crate::builtins::{HALF_SCREEN_HEIGHT, HALF_SCREEN_WIDTH};
use crate::sprite::Trigger;
use crate::{Argb, Line, RenderHandle, ScratchProgram, World};

pub struct BackendImpl<S> {
    _p: PhantomData<S>,
}

pub struct Handle<'a> {
    buffer: Buffer<'a, &'static Window, &'static Window>,
}

impl<S: ScratchProgram<BackendImpl<S>>> RenderBackend<S> for BackendImpl<S> {
    type Handle<'a> = Handle<'a>;

    fn run() {
        let event_loop = EventLoop::new().unwrap();
        let builder = WindowBuilder::new().with_title("hctarcs");
        let window = &*Box::leak(Box::new(builder.build(&event_loop).unwrap()));
        window.set_resizable(false);
        let _ = window.request_inner_size(Size::Physical(PhysicalSize::new(480, 360)));
        let context = softbuffer::Context::new(window).unwrap();

        let mut world = World::<S, Self>::new();
        let mut surface = Surface::new(&context, window).unwrap();
        surface
            .resize(
                NonZeroU32::new((HALF_SCREEN_WIDTH * 2.0) as u32).unwrap(),
                NonZeroU32::new((HALF_SCREEN_HEIGHT * 2.0) as u32).unwrap(),
            )
            .unwrap();

        event_loop.set_control_flow(ControlFlow::Wait);  // Poll
        event_loop.run(move |event, elwt| {
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
                    let mut handle = Handle { buffer: surface.buffer_mut().unwrap() };
                    world.broadcast(&mut handle, Trigger::FlagClicked);
                    handle.buffer.present().unwrap();
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


impl<'a> RenderHandle for Handle<'a> {
    fn pen_pixel(&mut self, (x, y): (f64, f64), colour: Argb) {
        let x = (x + HALF_SCREEN_WIDTH) as usize;
        let y = (HALF_SCREEN_HEIGHT - y) as usize;
        let i = x + (y * (HALF_SCREEN_WIDTH * 2.0) as usize);
        if i > 0 && i < self.buffer.len() {
            self.buffer[i] = colour.0;
        }
    }

    fn pen_line(&mut self, _line: Line) {
        todo!()
    }

    fn pen_stamp(&mut self, _pos: (f64, f64), _costume: usize) {
        todo!()
    }
}
