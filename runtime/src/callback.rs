//! DIY shitty async runtime.
//! The idea is each stack of blocks will be split into many functions, each ending in an await point.
#![allow(unused)]
use std::any::Any;
use std::collections::VecDeque;
use std::hint::black_box;
use std::mem::{size_of, size_of_val};
use std::ops::Add;
use std::pin::{Pin, pin};
use std::time::{Duration};
use crate::{FrameCtx, RenderBackend, ScratchProgram, Trigger, World};
use crate::sprite::{Sprite, SpriteBase};
use crate::Instant;

// TODO: try to clean up the concurrency model. Relationship between IoAction, FnFut, and FutOut feels a bit over complicated.
pub enum IoAction<S: ScratchProgram<R>, R: RenderBackend<S>> {
    WaitUntil(Instant),
    // TODO: compiler doesnt emit these, it calls the context method.
    // TODO: its awkward that I have this fusion between requests and waiting for requests
    /// https://en.scratch-wiki.info/wiki/Ask_()_and_Wait_(block)
    Ask(String),  // Tell the event loop to request user input. Replaced with WaitForAsk(_).
    WaitForAsk(usize),  // Suspend until the input request with id _ is fulfilled.
    BroadcastWait(S::Msg),
    Call(Box<FnFut<S, R>>),
    CloneMyself,
    LoopYield,
    None,
    Concurrent(Vec<IoAction<S, R>>),
    Sequential(Vec<IoAction<S, R>>),
}

// This any is very unfortunate
pub type FnFut<S, R> = dyn FnMut(&mut FrameCtx<S, R>, &mut dyn Any) -> FutOut<S, R>;
pub type FutOut<S, R> = (IoAction<S, R>, Callback<S, R>);

pub struct Script<S: ScratchProgram<R>, R: RenderBackend<S>>  {
    pub next: Vec<IoAction<S, R>>,
    pub owner: usize,  // Which instance requested this action
}

// Necessary because closures can't return a reference to themselves.
pub enum Callback<S: ScratchProgram<R>, R: RenderBackend<S>> {
    /// Resolved. Call this action next.
    Then(Box<FnFut<S, R>>),
    /// Not yet resolved. Must call the same action again. (ex. body of loop).
    Again,
    /// Fully resolved.
    Done,
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

impl<S: ScratchProgram<R>, R: RenderBackend<S>> IoAction<S, R> {
    pub fn sleep(seconds: f64) -> Self {
        if seconds > 0.0 {
            IoAction::WaitUntil(Instant::now().add(Duration::from_secs(seconds as u64)))
        } else {  // TODO: does scratch yield for sleep(0)?
            IoAction::None
        }
    }

    // TODO: hefty type signature
    pub fn then<F: FnMut(&mut FrameCtx<S, R>, &mut dyn Any) -> FutOut<S, R> + 'static>(self, f: F) -> FutOut<S, R> {
        assert_zero_sized::<F>();  // If no captures, we dont have to allocate. (Note: Rust uses fat ptrs instead of storing vtable ptr in the value)
        (self, Callback::Then(Box::new(f)))
    }

    pub fn then_boxed(self, f: impl FnMut(&mut FrameCtx<S, R>, &mut dyn Any) -> FutOut<S, R> + 'static) -> FutOut<S, R> {
        (self, Callback::Then(Box::new(f)))
    }

    pub fn again(self) -> FutOut<S, R> {
        (self, Callback::Again)
    }

    pub fn done(self) -> FutOut<S, R> {
        (self, Callback::Done)
    }

    // TODO: you dont want to use these because the allocate a lot
    // (IoAction is never zero sized so boxing trick doesn't help)
    pub fn append(self, other: IoAction<S, R>) -> IoAction<S, R> {
        IoAction::Sequential(vec![self, other])
    }

    pub  fn append_call(self, f: impl FnMut(&mut FrameCtx<S, R>, &mut dyn Any) -> FutOut<S, R> + 'static) -> IoAction<S, R> {
        IoAction::Sequential(vec![self, IoAction::Call(Box::new(f))])
    }
}

pub fn forward_to_sync<S: ScratchProgram<R>, R: RenderBackend<S>, O: Sprite<S, R> + Sized>(msg: Trigger<S::Msg>) -> Box<FnFut<S, R>> {
    Box::new(move |ctx, this| {
        let this: &mut O = ctx.trusted_cast(this);
        this.receive(ctx, msg);
        IoAction::None.done()
    })
}

