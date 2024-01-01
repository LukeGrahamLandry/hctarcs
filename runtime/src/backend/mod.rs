//! By default no backends are enabled.
//! The generated scratch project must use a cargo feature flag to enable one.
use crate::ScratchProgram;

#[cfg(feature = "render-softbuffer")]
pub mod softbuffer;
#[cfg(feature = "render-notan")]
pub mod notan;

pub trait RenderBackend<S: ScratchProgram<Self>>: Sized {
    type Handle: RenderHandle;

    /// This function does not return until the program is over.
    fn run();
}

// TODO: UNUSED thus far
// the idea is this will be something that can be passed into builtin ctx methods to request rendering work.
// this might be the same struct as the RenderBackend but it might not want to give a unique reference to everything.
pub trait RenderHandle {

}
