use std::collections::VecDeque;
use crate::sprite::{Sprite, SpriteBase};

pub mod sprite;
pub mod builtins;

/// Types for Msg and Globals are generated for a specific scratch program by the compiler.
/// The default form of message must be FlagClicked
// TODO: needs to be a super of some builtin enum for other events the runtime needs to send
pub struct World<Msg: Clone + Copy + Default, Globals> {
    pub bases: VecDeque<SpriteBase>,
    pub custom: VecDeque<Box<dyn Sprite<Msg, Globals>>>,
    pub globals: Globals,
    pub messages: VecDeque<Msg>
}

impl<Msg: Clone + Copy + Default, Globals> World<Msg, Globals> {
    /// This function does not return until the program is over.
    pub fn run_program(globals: Globals, custom: Vec<Box<dyn Sprite<Msg, Globals>>>) {
        let mut world = World {
            bases: vec![SpriteBase::default(); custom.len()].into(),
            custom: custom.into(),
            globals,
            messages: VecDeque::new(),
        };

        world.broadcast(Msg::default());

        for sprite in &world.bases {
            println!("Drew {:?} lines.", sprite.lines.len());
        }

    }

    // TODO: the compiler knows which messages each type wants to listen to.
    //       it could generate a separate array for each and have no virtual calls or traversing everyone on each message
    pub fn broadcast(&mut self, msg: Msg) {
        let sprites = self.bases.iter_mut().zip(self.custom.iter_mut());
        for (b, c) in sprites {
            c.receive(b, &mut self.globals, msg.clone());
        }
    }

}


