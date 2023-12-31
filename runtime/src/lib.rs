use std::collections::VecDeque;
use std::env;
use crate::backend::RenderBackend;
use crate::builtins::FrameCtx;
use crate::sprite::{Sprite, SpriteBase, Trigger};

pub mod sprite;
pub mod builtins;
pub mod callback;
pub mod poly;
pub mod backend;

/// Types for Msg and Globals are generated for a specific scratch program by the compiler.
/// This crate needs to be generic over the user program but there will only ever be one instantiation of this generic in a given application.
pub struct World<Msg: Copy, Globals, R: RenderBackend> {
    pub bases: VecDeque<SpriteBase>,
    pub custom: VecDeque<Box<dyn Sprite<Msg, Globals, R>>>,
    pub globals: Globals,
    pub messages: VecDeque<Msg>
}

impl<Msg: Copy, Globals, R: RenderBackend> World<Msg, Globals, R> {
    pub fn new(globals: Globals, custom: Vec<Box<dyn Sprite<Msg, Globals, R>>>) -> Self {
        credits();
        World {
            bases: vec![SpriteBase::default(); custom.len()].into(),
            custom: custom.into(),
            globals,
            messages: VecDeque::new(),
        }
    }

    // TODO: the compiler knows which messages each type wants to listen to.
    //       it could generate a separate array for each and have no virtual calls or traversing everyone on each message
    pub fn broadcast(&mut self, render: &mut R, msg: Trigger<Msg>) {
        let sprites = self.bases.iter_mut().zip(self.custom.iter_mut());
        for (sprite, c) in sprites {
            let mut ctx = FrameCtx {
                sprite,
                globals: &mut self.globals,
                render,
                _p: Default::default(),
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
