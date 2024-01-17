//! DIY shitty async runtime.
//! The idea is each stack of blocks will be split into many functions, each ending in an await point.
#![allow(unused)]
use std::any::Any;
use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::hint::black_box;
use std::mem::{size_of, size_of_val};
use std::num::NonZeroU16;
use std::ops::{Add, ControlFlow, FromResidual, Try};
use std::pin::{Pin, pin};
use std::time::{Duration};
use crate::{FrameCtx, RenderBackend, ScratchProgram, Trigger, World};
use crate::sprite::{Sprite, SpriteBase};
use crate::Instant;

// TODO: try to clean up the concurrency model. Relationship between IoAction, FnFut, and FutOut feels a bit over complicated.
pub enum IoAction<S: ScratchProgram<R>, R: RenderBackend<S>> {
    SleepSecs(f64),  // Resolves to WaitUntil(_)
    WaitUntil(Instant),
    /// https://en.scratch-wiki.info/wiki/Ask_()_and_Wait_(block)
    Ask(String),  // Tell the event loop to request user input. Resolves to WaitForAsk.
    // Don't need to remember which ask we're waiting on because the world can only have one active at a time.
    WaitForAsk(usize), // sprite id, TODO: dont need id cause its on the script
    BroadcastWait(S::Msg),
    CloneMyself,  // TODO: impl
    LoopYield,
    StopAllScripts,  // TODO: is this just on one sprite?
    /// Pop back up the closest CallMarker
    StopCurrentScript,
    None,
    // TODO: This should be a Vec<Script> instead since you might want to wait on other sprites.
    //       then receive should return a vec![ioaction] and the runtime turns it into one of these or flattens it.
    //       then the top level of the runtime can also be one of these and everything's consistant.
    Concurrent(Vec<IoAction<S, R>>),
    ConcurrentScripts(Vec<Script<S, R>>),
    // TODO: this biggest variant (5 words), can i cut it down? name can be an index.
    //       im sure u16 is enough for both but rust might word align things anyway
    /// An iterator yielding IoActions.
    FutMachine(Box<FutMachine<S, R>>, &'static str, NonZeroU16),
}

// TODO: this can go on FutOut for breakpoints and is zero sized in not(inspect)
#[cfg(feature = "inspect")]
pub type DbgId = usize;
#[cfg(not(feature = "inspect"))]
pub type DbgId = ();

pub type FutMachine<S, R> = dyn FnMut(&mut FrameCtx<S, R>, &mut dyn Any, NonZeroU16) -> FutRes<S, R>;
// Storing the state outside in the runtime means functions with no args (and no loops) don't need to allocate.
// And that the debugger can show where you are in the function.
pub struct FutRes<S: ScratchProgram<R>, R: RenderBackend<S>>(pub IoAction<S, R>, pub Option<NonZeroU16>);

#[derive(Debug)]
pub struct Script<S: ScratchProgram<R>, R: RenderBackend<S>>  {
    pub next: Vec<IoAction<S, R>>,
    pub owner: usize,  // Which instance requested this action
    pub trigger: Trigger<S::Msg> // Shown in debugger
}

// TODO: this is unnecessary but makes me feel better
pub const fn assert_zero_sized<T>(){
    if size_of::<T>() != 0 {  // assert_eq isn't const
        panic!("Expected zero-sized.");
    }
}

pub const fn assert_nonzero_sized<T>(){
    if size_of::<T>() == 0 {  // assert_ne isn't const
        panic!("Expected non-zero-sized.");
    }
}

impl<S: ScratchProgram<R>, R: RenderBackend<S>> Debug for IoAction<S, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IoAction::WaitUntil(time) => write!(f, "WaitUntil({time:?})"),
            IoAction::Ask(q) => write!(f, "Ask({q:?})"),
            IoAction::WaitForAsk(id) => write!(f, "WaitForAsk({id})"),
            IoAction::BroadcastWait(msg) => write!(f, "BroadcastWait({msg:?})"),
            IoAction::CloneMyself => write!(f, "CloneMyself"),
            IoAction::LoopYield => write!(f, "LoopYield"),
            IoAction::None => write!(f, "None"),
            IoAction::Concurrent(c) => write!(f, "Concurrent({c:?})"),
            IoAction::SleepSecs(s) => write!(f, "StartSleep({s})"),
            IoAction::StopAllScripts => write!(f, "StopAllScripts"),
            IoAction::StopCurrentScript => write!(f, "StopCurrentScript"),
            IoAction::ConcurrentScripts(s) => write!(f, "ConcurrentScripts(...)"),
            IoAction::FutMachine(_, name, state) => write!(f, "{name} [{state}]"),
        }
    }
}

/// This should only be called by the compiler.
#[macro_export]
macro_rules! state {
    ($n:expr) => {{
        debug_assert_ne!($n, 0, "ICE");
        unsafe { std::num::NonZeroU16::new_unchecked($n) }
    }};
}

pub fn fut<S, R, F>(name: &'static str, f: F) -> IoAction<S, R>
    where S: ScratchProgram<R>,
          R: RenderBackend<S>,
          F: FnMut(&mut FrameCtx<S, R>, &mut dyn Any, NonZeroU16) -> FutRes<S, R> + 'static
{
    assert_zero_sized::<F>();
    IoAction::FutMachine(Box::new(f), name, state!(1))
}

pub fn fut_a<S, R, F>(name: &'static str, f: F) -> IoAction<S, R>
    where S: ScratchProgram<R>,
          R: RenderBackend<S>,
          F: FnMut(&mut FrameCtx<S, R>, &mut dyn Any, NonZeroU16) -> FutRes<S, R> + 'static
{
    IoAction::FutMachine(Box::new(f), name, state!(1))
}

impl<S: ScratchProgram<R>, R: RenderBackend<S>> Try for FutRes<S, R> {
    type Output = ();
    type Residual = FutRes<S, R>;

    fn from_output(output: Self::Output) -> Self {
        todo!()
    }

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match &self.0 {
            IoAction::None => ControlFlow::Continue(()),
            _ => ControlFlow::Break(self),
        }
    }
}

impl<S: ScratchProgram<R>, R: RenderBackend<S>> FromResidual for FutRes<S, R> {
    fn from_residual(residual: <Self as Try>::Residual) -> Self {
        residual
    }
}
