#![allow(non_snake_case)]  // TODO: compiler could fix the names for me

use std::any::{Any};
use rand::{Rng, SeedableRng};
use rand::rngs::{StdRng, ThreadRng};
use std::cell::RefCell;
use std::io::{stdout, Write};
use std::time::{SystemTime};
use crate::backend::RenderBackend;
use crate::poly::Str;
use crate::{RenderHandle, ScratchProgram, Sprite};
use crate::sprite::{Line, SpriteBase};

pub const HALF_SCREEN_WIDTH: f64 = 240.0;
pub const HALF_SCREEN_HEIGHT: f64 = 180.0;


// Note: each instance is linked to a specific sprite.
pub struct FrameCtx<'msg, 'frame: 'msg, S: ScratchProgram<R>, R: RenderBackend<S>> {
    pub sprite: &'msg mut SpriteBase,
    // pub vars: &'a mut S,
    pub globals: &'msg mut S::Globals,
    pub(crate) render: &'msg mut R::Handle<'frame>,
}

// TODO: think about some macro magic to generate the prototypes in the compiler based on these functions.
impl<'msg, 'frame: 'msg, S: ScratchProgram<R>, R: RenderBackend<S>> FrameCtx<'msg, 'frame, S, R> {
    // TODO: check if using downcast_mut_unchecked is faster (it still asserts in debug builds)
    // TODO: why can you only call downcast_mut on dyn Any not dyn <some trait: Any>. https://github.com/rust-lang/rust/issues/65991
    // The existence of this is unfortunate.
    // This should ONLY be called by the compiler!
    // Being a method on the Ctx gives opportunity to add more precise satisfy checks later (ex. same instance uid as expected).
    pub fn trusted_cast<'a, O: Sprite<S, R>>(&self, sprite: &'a mut dyn Any) -> &'a mut O {
        sprite.downcast_mut().unwrap()
    }

    pub fn pen_setPenColorToColor(&mut self, colour: f64) {
        self.sprite.pen.colour.0 = colour.max(0.0) as u32;
    }

    pub fn pen_setPenSizeTo(&mut self, size: f64) {
        self.sprite.pen.size = size;
    }

    pub fn pen_penUp(&mut self) {
        self.sprite.pen.active = false;
    }

    pub fn pen_penDown(&mut self) {
        self.sprite.pen.active = true;
    }

    pub fn motion_changexby(&mut self, dx: f64) {
        let old = self.pos();
        self.sprite.x = (self.sprite.x + dx).clamp(-HALF_SCREEN_WIDTH, HALF_SCREEN_WIDTH);
        self.draw(old);
    }

    pub fn motion_changeyby(&mut self, dy: f64) {
        let old = self.pos();
        self.sprite.y = (self.sprite.y + dy).clamp(-HALF_SCREEN_HEIGHT, HALF_SCREEN_HEIGHT);
        self.draw(old);
    }

    pub fn motion_setx(&mut self, x: f64) {
        let old = self.pos();
        self.sprite.x = x.clamp(-HALF_SCREEN_WIDTH, HALF_SCREEN_WIDTH);
        self.draw(old);
    }

    pub fn motion_sety(&mut self, y: f64) {
        let old = self.pos();
        self.sprite.y = y.clamp(-HALF_SCREEN_HEIGHT, HALF_SCREEN_HEIGHT);
        self.draw(old);
    }

    pub fn motion_gotoxy(&mut self, x: f64, y: f64) {
        let old = self.pos();
        self.sprite.x = x.clamp(-HALF_SCREEN_WIDTH, HALF_SCREEN_WIDTH);
        self.sprite.y = y.clamp(-HALF_SCREEN_HEIGHT, HALF_SCREEN_HEIGHT);
        self.draw(old);
    }

    pub fn motion_xposition(&self) -> f64 {
        self.sprite.x
    }

    pub fn motion_yposition(&self) -> f64 {
        self.sprite.y
    }

    pub fn sensing_answer(&self) -> Str {
        Str::Owned(self.sprite.last_answer.clone())
    }

    pub fn pen_stamp(&mut self) {
        // TODO: make sure this uses sprite size not pen size
        self.render.pen_stamp(self.pos(), self.sprite.costume, self.sprite.size_frac);
    }

    pub fn looks_hide(&mut self) {
        self.sprite.hidden = true;
    }

    pub fn pen_clear(&mut self) {
        self.render.pen_clear();
    }

    pub fn looks_setsizeto(&mut self, size: f64) {
        // TODO: what are the limits?
        self.sprite.size_frac = size / 100.0;
    }

    // TODO: each sprite has its own set of costumes and they can have overlapping names so really it has to pass in the id and costume_by_name needs to be on the sprite trait
    pub fn looks_switchcostumeto(&mut self, costume: Str) {
        if let Some(id) = S::costume_by_name(costume.clone()) {
            self.sprite.costume = id;
        }

        match costume {
            Str::Char(c) => {
                print!("{c}");
                stdout().flush().unwrap();
            },
            Str::Const(_) | Str::Owned(_) => {},
        }
    }

    // TODO: remove
    pub fn sensing_askandwait_sync(&mut self) {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        self.sprite.last_answer = line;  // TODO: trim new-line?
    }

    pub fn looks_say(&mut self, msg: Str) {
        println!("[SAY] {msg:?}");  // TODO: dont print.
        self.render.say(msg.as_ref(), self.pos());
    }

    pub fn stop_all_sync(&mut self) {
        println!("stop all");
        self.render.save_frame(&format!("scratch_exit_{}.png", SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()));
        std::process::exit(0);
    }

    pub fn control_create_clone_of(&self) {
        println!("TODO: clone-myself")
    }

    fn pos(&self) -> (f64, f64) {
        (self.sprite.x, self.sprite.y)
    }

    fn draw(&mut self, old: (f64, f64)) {
        if self.sprite.pen.active {
            let end = self.pos();
            let is_pixel = self.sprite.pen.size <= 1.0 && 1.0 == ((old.0 - end.0).powi(2) + (old.1 - end.1).powi(2)) ;
            if is_pixel {
                self.render.pen_pixel(end, self.sprite.pen.colour);
            } else {
                self.render.pen_line(Line {
                    start: old,
                    end,
                    size: self.sprite.pen.size,
                    colour: self.sprite.pen.colour,
                })
            }
        }
    }
}

thread_local! {
    static RNG: RefCell<StdRng> = RefCell::new(StdRng::from_rng(ThreadRng::default()).unwrap());
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
