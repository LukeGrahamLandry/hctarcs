use crate::{ScratchProgram, World};

pub mod softbuffer;
pub mod notan;

pub trait RenderBackend<S: ScratchProgram<Self>>: Sized {
    type Handle;

    // TODO: make sure 'static is what i mean
    /// This function does not return until the program is over.
    fn run();
}
