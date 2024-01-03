//! DIY shitty async runtime.
//! The idea is each stack of blocks will be split into many functions, each ending in an await point.
//! UNUSED thus far
#![allow(unused)]
use std::any::Any;
use std::collections::VecDeque;
use std::hint::black_box;
use std::mem::{size_of, size_of_val};
use std::pin::{Pin, pin};
use crate::{credits, FrameCtx, RenderBackend, ScratchProgram, Trigger, World};
use crate::sprite::{Sprite, SpriteBase};

pub enum IoAction<S: ScratchProgram<R>, R: RenderBackend<S>> {
    WaitSecs(f64),
    Ask(String),
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

const fn assert_zero_sized<T>(){
    if size_of::<T>() != 0 {  // assert_eq isn't const
        panic!("Expected zero-sized.");
    }
}

impl<S: ScratchProgram<R>, R: RenderBackend<S>> IoAction<S, R> {
    // TODO: hefty type signature
    fn then<F: FnMut(&mut FrameCtx<S, R>, &mut dyn Any) -> FutOut<S, R> + 'static>(self, f: F) -> FutOut<S, R> {
        assert_zero_sized::<F>();  // If no captures, we dont have to allocate. (Note: Rust uses fat ptrs instead of storing vtable ptr in the value)
        (self, Callback::Then(Box::new(f)))
    }

    fn then_boxed(self, f: impl FnMut(&mut FrameCtx<S, R>, &mut dyn Any) -> FutOut<S, R> + 'static) -> FutOut<S, R> {
        (self, Callback::Then(Box::new(f)))
    }

    fn again(self) -> FutOut<S, R> {
        (self, Callback::Again)
    }

    fn done(self) -> FutOut<S, R> {
        (self, Callback::Done)
    }
}

fn do_something<S: ScratchProgram<R>, R: RenderBackend<S>, O: Sprite<S, R>>(count: usize) -> FutOut<S, R> {
    // Note: can't access the Ctx yet.
    IoAction::None.then(move |ctx, this| {
        let this: &mut O = ctx.trusted_cast(this);
        // sync work...
        IoAction::WaitSecs(1.0).then(move |ctx, this| {
            let this: &mut O = ctx.trusted_cast(this);
            // sync work...
            let mut i = count;
            IoAction::None.then_boxed(move |ctx, this| {
                let this: &mut O = ctx.trusted_cast(this);
                if i == 0 {
                    // sync work... (after end of loop)
                    IoAction::Ask(String::from("Hello")).done()
                } else {
                    i -= 1;
                    // sync work... (body of loop)
                    // Note: instead of LoopYield, it could be IoAction::Call(|ctx| { ... }) if there's an await point in the body.
                    IoAction::LoopYield.again()
                }
            })
        })
    })
}
