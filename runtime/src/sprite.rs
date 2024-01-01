use std::fmt::Debug;
use crate::backend::RenderBackend;
use crate::builtins::FrameCtx;
use crate::ScratchProgram;

#[derive(Clone, Default, Debug)]
pub struct SpriteBase {
    pub(crate) _uid: usize,  // TODO: increment on each clone for safety checking callback targets
    pub x: f64,
    pub y: f64,
    pub direction: f64,
    pub speed: f64,
    pub pen: Pen,
    // TODO: is this shared across sprites?
    // TODO: Will be async
    pub last_answer: String,
}

#[derive(Clone, Default, Debug)]
pub struct Line {
    pub start: (f64, f64),
    pub end: (f64, f64),
    pub size: f64,
    pub colour: Argb,
}

#[derive(Clone, Default, Debug)]
pub struct Pen {
    pub size: f64,
    pub active: bool,
    pub colour: Argb
}

#[derive(Copy, Clone, Default, Debug)]
pub struct Argb(pub u32);

/// Wraps the custom Msg defined by the program and adds some builtin ones that the runtime knows how to construct.
#[derive(Copy, Clone, Debug)]
pub enum Trigger<Msg> {
    FlagClicked,
    Message(Msg),
}

pub trait Sprite<S: ScratchProgram<R>, R: RenderBackend<S>>: Debug {
    fn receive(&mut self, ctx: &mut FrameCtx<S, R>, msg: Trigger<S::Msg>);

    // You can't just say Sprite extends Clone because that returns Self so its not object safe.
    // You can't just impl here and have where Self: Clone cause you can't call it on the trait object.
    fn clone_boxed(&self) -> Box<dyn Sprite<S, R>>;
}
