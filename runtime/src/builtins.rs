#![allow(non_snake_case)]  // TODO: compiler could fix the names for me
use crate::sprite::{Colour, SpriteBase};

// TODO: think about some macro magic to generate the prototypes in the compiler based on these functions.
impl SpriteBase {
    pub fn pen_setPenColorToColor(&mut self, colour: Colour) {
        self.pen.colour = colour;
    }

    pub fn pen_setPenSizeTo(&mut self, size: f64) {
        self.pen.size = size;
    }

    pub fn pen_penUp(&mut self) {
        self.pen.down = false;
    }

    pub fn pen_penDown(&mut self) {
        self.pen.down = true;
    }
}
