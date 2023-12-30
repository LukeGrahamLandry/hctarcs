//! DIY shitty async runtime.
//! The idea is each stack of blocks will be split into many functions, each ending in an await point.
//! UNUSED thus far
#![allow(unused)]
use std::any::Any;
use crate::sprite::{Sprite, SpriteBase};

pub enum IoAction<Msg: Copy + Clone> {
    WaitSecs(f64),
    Ask(String),
    BroadcastWait(Msg),
    WaitingLoopTimes(usize, FuncId),
    Call(FuncId),
    None
}

pub struct IO<Msg: Copy + Clone> {
    action: IoAction<Msg>,
    owner: usize,  // Which instance requested this action
    func: Option<FuncId>,  // Which function should we call when the action is done,
    callstack: Vec<Box<dyn Any>>
}

// TODO: NonZero for niche optimization?
pub struct FuncId(usize);
