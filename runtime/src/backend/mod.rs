use crate::World;

pub mod softbuffer;

pub trait RenderBackend: Sized {
    /// This function does not return until the program is over.
    fn run<M: Copy, G>(world: World<M, G, Self>);
}
