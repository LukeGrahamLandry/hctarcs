//! DIY shitty async runtime.
//! The idea is each stack of blocks will be split into many functions, each ending in an await point.
#![allow(unused)]
use std::any::Any;
use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
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
    SleepSecs(f64),
    WaitUntil(Instant),
    // TODO: compiler doesnt emit these, it calls the context method.
    // TODO: its awkward that I have this fusion between requests and waiting for requests
    /// https://en.scratch-wiki.info/wiki/Ask_()_and_Wait_(block)
    Ask(String, usize),  // Tell the event loop to request user input. Replaced with WaitForAsk(_).
    WaitForAsk(usize), // sprite id
    BroadcastWait(S::Msg),
    Loop(Box<FnFutLoop<S, R>>),
    /// This means you can move captures out of the closure.
    /// This is useful for function bodies where you need to return a callable future but you want to
    /// capture arguments. TODO: this doesnt solve nested loops.
    CallOnce(Box<FnFutOnce<S, R>>),
    UserFnBody(Box<FnFutOnce<S, R>>, &'static str),  // TODO: make this an index
    CloneMyself,
    LoopYield,
    StopAllScripts,
    StopCurrentScript,
    CallMarker(&'static str),  // TODO: make this an index
    None,
    // TODO: This should be a Vec<Script> instead since you might want to wait on other sprites.
    //       then receive should return a vec![ioaction] and the runtime turns it into one of these or flattens it.
    //       then the top level of the runtime can also be one of these and everything's consistant.
    Concurrent(Vec<IoAction<S, R>>),
    /// This is immediately expanded by the runtime into the current script.
    /// It's just a way to do multiple things where one was expected.
    Sequential(Vec<IoAction<S, R>>),
    ConcurrentScripts(Vec<Script<S, R>>),
}

pub enum LoopRes<S: ScratchProgram<R>, R: RenderBackend<S>> {
    Continue(IoAction<S, R>),
    Break(IoAction<S, R>),
}

// TODO: this can go on FutOut for breakpoints and is zero sized in not(inspect)
#[cfg(feature = "inspect")]
pub type DbgId = usize;
#[cfg(not(feature = "inspect"))]
pub type DbgId = ();

// This any is very unfortunate
pub type FnFut<S, R> = dyn FnMut(&mut FrameCtx<S, R>, &mut dyn Any) -> FutOut<S, R>;
pub type FnFutLoop<S, R> = dyn FnMut(&mut FrameCtx<S, R>, &mut dyn Any) -> LoopRes<S, R>;
pub type FnFutOnce<S, R> = dyn FnOnce(&mut FrameCtx<S, R>, &mut dyn Any) -> FutOut<S, R>;
pub type FutOut<S, R> = (IoAction<S, R>, Callback<S, R>);

#[derive(Debug)]
pub struct Script<S: ScratchProgram<R>, R: RenderBackend<S>>  {
    pub next: Vec<IoAction<S, R>>,
    pub owner: usize,  // Which instance requested this action
}

// Necessary because closures can't return a reference to themselves.
pub enum Callback<S: ScratchProgram<R>, R: RenderBackend<S>> {
    /// Resolved. Call this action next.
    // TODO: this could be an ioaction and make everything more consistant
    ThenOnce(Box<FnFutOnce<S, R>>),
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
    pub fn then_once<F: FnOnce(&mut FrameCtx<S, R>, &mut dyn Any) -> FutOut<S, R> + 'static>(self, f: F) -> FutOut<S, R> {
        // but im using this to capture args
        // assert_zero_sized::<F>();  // If no captures, we dont have to allocate. (Note: Rust uses fat ptrs instead of storing vtable ptr in the value)
        (self, Callback::ThenOnce(Box::new(f)))
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
}

// TODO: i'd rather the syntax be list of stmts semicolon terminated with optional last than current where blocks need to be { wrapped }
// Comptime assert that a function call is synchronous. Just a sanity check for the compiler.
#[macro_export]
macro_rules! nosuspend {
    ($call:expr) => {{
        let _: () = $call;
    }};
}

impl<S: ScratchProgram<R>, R: RenderBackend<S>> Debug for IoAction<S, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IoAction::WaitUntil(time) => write!(f, "WaitUntil({time:?})"),
            IoAction::Ask(q, id) => write!(f, "Ask({q:?}, {id})"),
            IoAction::WaitForAsk(id) => write!(f, "WaitForAsk({id})"),
            IoAction::BroadcastWait(msg) => write!(f, "BroadcastWait({msg:?})"),
            IoAction::Loop(_) => write!(f, "Loop(...)"),
            IoAction::CloneMyself => write!(f, "CloneMyself"),
            IoAction::LoopYield => write!(f, "LoopYield"),
            IoAction::None => write!(f, "None"),
            IoAction::Concurrent(c) => write!(f, "Concurrent({c:?})"),
            IoAction::Sequential(c) => write!(f, "Sequential({c:?})"),
            IoAction::SleepSecs(s) => write!(f, "StartSleep({s})"),
            IoAction::StopAllScripts => write!(f, "StopAllScripts"),
            IoAction::StopCurrentScript => write!(f, "StopCurrentScript"),
            IoAction::CallOnce(_) => write!(f, "CallOnce(FnFutOnce...)"),
            IoAction::UserFnBody(_, name) => write!(f, "UserFnBody({name})"),
            IoAction::ConcurrentScripts(s) => write!(f, "ConcurrentScripts(...)"),
            IoAction::CallMarker(name) =>  write!(f, "CallMarker({name})"),
        }
    }
}

impl<S: ScratchProgram<R>, R: RenderBackend<S>> Debug for Callback<S, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Callback::Again => write!(f, "Callback::Again"),
            Callback::Done => write!(f, "Callback::Done"),
            Callback::ThenOnce(s) =>write!(f, "Callback::ThenOnce(FnFutOnce...)"),
        }
    }
}


// #[macro_export]
// macro_rules! fut {
//     ($($part:stmt);*; $res:expr) => {{
//         move |ctx, this| {
//             let this: &mut Self = ctx.trusted_cast(this);
//             $($part);*;
//             $res
//         }
//     }};
// }

// cant use this because it breaks formatter?
#[macro_export]
macro_rules! async_fn {
    ($name:expr, $body:block) => {{
        IoAction::UserFnBody(Box::new(move |ctx, this| {{
            let this: &mut Self = ctx.trusted_cast(this);
            let __inner: IoAction<Stage, Backend> = $body;
            __inner.done()
        }}), $name)
    }};
}

