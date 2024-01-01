use std::collections::VecDeque;
use std::env;

pub mod sprite;
pub mod builtins;
pub mod callback;
pub mod poly;
pub mod backend;

pub use sprite::*;
pub use builtins::*;
pub use poly::*;
pub use backend::*;

pub trait ScratchProgram<R: RenderBackend<Self>>: Sized + 'static {
    type Msg: Copy;
    type Globals;
    fn create_initial_state() -> (Self::Globals, Vec<Box<dyn Sprite<Self, R>>>);
}

/// Types for Msg and Globals are generated for a specific scratch program by the compiler.
/// This crate needs to be generic over the user program but there will only ever be one instantiation of this generic in a given application.
pub struct World<S: ScratchProgram<R>, R: RenderBackend<S>> {
    bases: VecDeque<SpriteBase>,
    custom: VecDeque<Box<dyn Sprite<S, R>>>,
    globals: S::Globals,
    _messages: VecDeque<S::Msg>
}

impl<S: ScratchProgram<R>, R: RenderBackend<S>> World<S, R> {
    pub fn new() -> Self {
        let (globals, custom) = S::create_initial_state();
        credits();
        World {
            bases: vec![SpriteBase::default(); custom.len()].into(),
            custom: custom.into(),
            globals,
            _messages: VecDeque::new(),
        }
    }

    // TODO: the compiler knows which messages each type wants to listen to.
    //       it could generate a separate array for each and have no virtual calls or traversing everyone on each message
    pub fn broadcast<'a, 'frame: 'a>(&'a mut self, render: &'a mut R::Handle<'frame, 'frame>, msg: Trigger<S::Msg>) {
        let sprites = self.bases.iter_mut().zip(self.custom.iter_mut());
        for (sprite, c) in sprites {
            let mut ctx = FrameCtx {
                sprite,
                globals: &mut self.globals,
                render,
            };
            c.receive(&mut ctx, msg.clone());
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
