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

    pub fn motion_changexby(&mut self, dx: f64) {
        self.x += dx;
    }

    pub fn motion_changeyby(&mut self, dy: f64) {
        self.y += dy;
    }

    pub fn motion_setx(&mut self, x: f64) {
        self.x = x;
    }

    pub fn motion_sety(&mut self, y: f64) {
        self.y = y;
    }
}

// TODO: depend on rand
pub fn dyn_rand(min: f64, max: f64) -> f64 {
    if min.round() == min && max.round() == max {
        // If both sides are whole numbers, result is a whole number.
        // TODO: make my compiler constant fold this branch
        min
    } else {
        min
    }
}
