//! DIY shitty async runtime.
//! The idea is each stack of blocks will be split into many functions, each ending in an await point.
//! UNUSED thus far
#![allow(unused)]
use std::any::Any;
use std::collections::VecDeque;
use std::hint::black_box;
use std::pin::{Pin, pin};
use crate::{credits, FrameCtx, RenderBackend, ScratchProgram, Trigger, World};
use crate::sprite::{Sprite, SpriteBase};

pub enum IoAction<S: ScratchProgram<R>, R: RenderBackend<S>> {
    WaitSecs(f64),
    Ask(String),
    BroadcastWait(S::Msg),
    Call(Box<Action<S, R>>),
    LoopYield,
    None,
    Concurrent(Vec<IoAction<S, R>>),
    Sequential(Vec<IoAction<S, R>>),
}

pub type Action<S, R> = dyn FnMut(&mut FrameCtx<S, R>) -> (IoAction<S, R>, Callback<S, R>);
pub struct Script<S: ScratchProgram<R>, R: RenderBackend<S>>  {
    pub next: Vec<IoAction<S, R>>,
    pub owner: usize,  // Which instance requested this action
}

// Necessary because closures can't return a reference to themselves.
pub enum Callback<S: ScratchProgram<R>, R: RenderBackend<S>> {
    /// Resolved. Call this action next.
    Then(Box<Action<S, R>>),
    /// Not yet resolved. Must call the same action again. (ex. body of loop).
    Again,
    /// Fully resolved.
    None,
}

// fn do_something<S: ScratchProgram<R>, R: RenderBackend<S>>(count: usize) -> (IoAction<S, R>, Callback<S, R>) {
//     (IoAction::None, Callback::Then(Box::new(move |ctx| {
//         // sync work...
//         (IoAction::WaitSecs(1.0), Callback::Then(Box::new(move |ctx| {
//                 // sync work...
//                 let mut i = count;
//                 (IoAction::None, Callback::Then(Box::new(move |ctx| {
//                     if i == 0 {
//                         // sync work... (after end of loop)
//                         (IoAction::Ask(String::from("Hello")), Callback::None)
//                     } else {
//                         i -= 1;
//                         // sync work... (body of loop)
//                         // Note: instead of LoopYield, it could be IoAction::Call(|ctx| { ... }) if there's an await point in the body.
//                         (IoAction::LoopYield, Callback::Again)
//                     }
//                 })))
//             })
//         ))
//     })))
// }
