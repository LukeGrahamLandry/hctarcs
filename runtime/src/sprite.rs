use std::collections::VecDeque;

pub struct SpriteBase {
    pub x: f64,
    pub y: f64,
    pub direction: f64,
    pub speed: f64,
    pub pen: Pen,
}

pub struct Pen {
    pub size: f64,
    pub down: bool,
    pub colour: Colour
}

pub struct Colour {
    pub hue: f64,
    pub saturation: f64,
    pub brightness: f64,
}

pub trait Sprite<Msg, Globals> {
    fn receive(&mut self, sprite: &mut SpriteBase, globals: &mut Globals, msg: Msg);
}

/// Types for Msg and Globals are generated for a specific scratch program by the compiler.
pub struct World<Msg: Clone + Copy, Globals> {
    pub bases: VecDeque<SpriteBase>,
    pub custom: VecDeque<Box<dyn Sprite<Msg, Globals>>>,
    pub globals: Globals,
    pub messages: VecDeque<Msg>
}

impl<Msg: Clone + Copy, Globals> World<Msg, Globals> {
    // TODO: the compiler knows which messages each type wants to listen to.
    //       it could generate a separate array for each and have no virtual calls or traversing everyone on each message
    pub fn broadcast(&mut self, msg: Msg) {
        let sprites = self.bases.iter_mut().zip(self.custom.iter_mut());
        for (b, c) in sprites {
            c.receive(b, &mut self.globals, msg.clone());
        }
    }
}
