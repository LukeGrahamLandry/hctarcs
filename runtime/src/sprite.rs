use std::any::Any;
use std::fmt::Debug;
use crate::backend::RenderBackend;
use crate::callback::FnFut;
use crate::{Poly, ScratchProgram};

#[cfg(feature = "inspect")]
use crate::ui::{VarBorrow, VarBorrowMut};

#[derive(Clone, Debug)]
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
    pub costume: usize,
    pub size_frac: f64,
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
    UiClearPen,
    FlagClicked,
    Message(Msg),
}

pub trait Sprite<S: ScratchProgram<R>, R: RenderBackend<S>>: Debug + Any {
    // TODO: this is not an async function; it returns an async function. that's a strange choice
    //       But that makes it easier to call because you dont need a Ctx so the world doesn't need to store messages to the next frame.
    fn receive_async(&self, _msg: Trigger<S::Msg>) -> Box<FnFut<S, R>>;

    // You can't just say Sprite extends Clone because that returns Self so its not object safe.
    // You can't just impl here and have where Self: Clone cause you can't call it on the trait object.
    fn clone_boxed(&self) -> Box<dyn Sprite<S, R>>;

    #[cfg(feature = "inspect")]
    fn get_var_names(&self) -> &'static [&'static str] {
        &[]
    }

    #[cfg(feature = "inspect")]
    fn var(&self, _: usize) -> VarBorrow {
        VarBorrow::Fail
    }

    #[cfg(feature = "inspect")]  // TODO: alas we're getting to the point of wanting a console scripting language.... if only someone had a nice ast
    fn var_mut(&mut self, _: usize) -> VarBorrowMut {
        VarBorrowMut::Fail
    }
}

impl Default for SpriteBase {
    fn default() -> Self {
        Self {
            _uid: 0,
            x: 0.0,
            y: 0.0,
            direction: 0.0,
            speed: 0.0,
            pen: Default::default(),
            last_answer: "".to_string(),
            costume: 0,
            size_frac: 2.0,  // TODO: does custom json have a scale?
        }
    }
}