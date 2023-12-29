#![allow(non_snake_case)]  // TODO: compiler could fix the names for me

use std::borrow::Cow;
use rand::{Rng, SeedableRng};
use rand::rngs::{StdRng, ThreadRng};
use std::cell::RefCell;
use crate::sprite::{Line, SpriteBase};

pub const HALF_SCREEN_WIDTH: f64 = 240.0;
pub const HALF_SCREEN_HEIGHT: f64 = 180.0;

// TODO: think about some macro magic to generate the prototypes in the compiler based on these functions.
impl SpriteBase {
    pub fn pen_setPenColorToColor(&mut self, colour: f64) {
        self.pen.colour.0 = colour.max(0.0) as u32;
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
        let old = self.pos();
        self.x = (self.x + dx).clamp(-HALF_SCREEN_WIDTH, HALF_SCREEN_WIDTH);
        self.draw(old);
    }

    pub fn motion_changeyby(&mut self, dy: f64) {
        let old = self.pos();
        self.y = (self.y + dy).clamp(-HALF_SCREEN_HEIGHT, HALF_SCREEN_HEIGHT);
        self.draw(old);
    }

    pub fn motion_setx(&mut self, x: f64) {
        let old = self.pos();
        self.x = x.clamp(-HALF_SCREEN_WIDTH, HALF_SCREEN_WIDTH);
        self.draw(old);
    }

    pub fn motion_sety(&mut self, y: f64) {
        let old = self.pos();
        self.y = y.clamp(-HALF_SCREEN_HEIGHT, HALF_SCREEN_HEIGHT);
        self.draw(old);
    }

    pub fn motion_gotoxy(&mut self, x: f64, y: f64) {
        let old = self.pos();
        self.x = x.clamp(-HALF_SCREEN_WIDTH, HALF_SCREEN_WIDTH);
        self.y = y.clamp(-HALF_SCREEN_HEIGHT, HALF_SCREEN_HEIGHT);
        self.draw(old);
    }

    pub fn motion_xposition(&self) -> f64 {
        self.x
    }

    pub fn motion_yposition(&self) -> f64 {
        self.y
    }

    fn pos(&self) -> (f64, f64) {
        (self.x, self.y)
    }

    fn draw(&mut self, old: (f64, f64)) {
        if self.pen.down {
            self.lines.push(Line {
                start: old,
                end: self.pos(),
                size: self.pen.size,
                colour: self.pen.colour,
            })
        }
    }
}

thread_local! {
    pub static RNG: RefCell<StdRng> = RefCell::new(StdRng::from_rng(ThreadRng::default()).unwrap());
}

pub fn dyn_rand(min: f64, max: f64) -> f64 {
    if min.round() == min && max.round() == max {
        // If both sides are whole numbers, result is a whole number.
        // TODO: make my compiler constant fold this branch
        RNG.with(|rng| rng.borrow_mut().gen_range((min as isize)..(max as isize)) as f64)
    } else {
        RNG.with(|rng| rng.borrow_mut().gen_range(min..max))
    }
}

// TODO: avoid this whenever possible
#[derive(Clone, PartialEq, Debug)]
pub enum NumOrStr {
    Num(f64),
    Str(Cow<'static, str>),
}

impl From<f64> for NumOrStr {
    fn from(value: f64) -> Self {
        NumOrStr::Num(value)
    }
}

impl From<Cow<'static, str>> for NumOrStr {
    fn from(value: Cow<'static, str>) -> Self {
        NumOrStr::Str(value)
    }
}

impl From<NumOrStr> for f64 {
    fn from(value: NumOrStr) -> Self {
        match value {
            NumOrStr::Num(n) => n,
            NumOrStr::Str(s) => todo!("Tried to convert {:?} to number.", s)
        }
    }
}

impl From<NumOrStr> for Cow<'static, str> {
    fn from(value: NumOrStr) -> Self {
        match value {
            NumOrStr::Num(n) => todo!("Tried to convert {:?} to string.", n),
            NumOrStr::Str(s) => s,
        }
    }
}
