//! By default no backends are enabled.
//! The generated scratch project must use a cargo feature flag to enable one.
use crate::{Argb, Line, ScratchProgram};

#[cfg(feature = "render-softbuffer")]
pub mod softbuffer;
#[cfg(feature = "render-notan")]
pub mod notan;
#[cfg(feature = "render-macroquad")]
pub mod macroquad;

pub trait RenderBackend<S: ScratchProgram<Self>>: Sized {
    type Handle<'a>: RenderHandle;

    /// This function does not return until the program is over.
    fn run();
}

// this might be the same struct as the RenderBackend but it might not want to give a unique reference to everything.
pub trait RenderHandle {
    fn pen_pixel(&mut self, pos: (f64, f64), colour: Argb);
    fn pen_line(&mut self, line: Line);

    fn pen_stamp(&mut self, pos: (f64, f64), costume: usize, size: f64);

    fn say(&mut self, text: &str, pos: (f64, f64));

    fn save_frame(&mut self, _path: &str) {
        todo!("save_frame not implemented on this backend");
    }
}
