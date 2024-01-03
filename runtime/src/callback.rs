//! DIY shitty async runtime.
//! The idea is each stack of blocks will be split into many functions, each ending in an await point.
//! UNUSED thus far
#![allow(unused)]
use std::any::Any;
use std::hint::black_box;
use std::pin::{Pin, pin};
use crate::sprite::{Sprite, SpriteBase};

pub enum IoAction<Msg: Copy> {
    WaitSecs(f64),
    Ask(String),
    BroadcastWait(Msg),
    WaitingLoopTimes(usize, FuncId),
    Call(Box<dyn ScratchFn<Msg>>),
    LoopYield,
    None
}

pub struct IO<Msg: Copy> {
    action: IoAction<Msg>,
    owner: usize,  // Which instance requested this action
    func: Option<FuncId>,  // Which function should we call when the action is done,
    callstack: Vec<Box<dyn Any>>
}

// TODO: NonZero for niche optimization?
#[derive(Eq, PartialEq)]
pub struct FuncId(usize);

pub struct TestCtx();

type State = usize;
pub trait ScratchFn<M: Copy> {
    fn run(&mut self, state: State, ctx: &mut TestCtx) -> (State, IoAction<M>);
}

struct DoSomething {
    i: usize
}

impl ScratchFn<usize> for DoSomething {
    fn run(&mut self, state: State, ctx: &mut TestCtx) -> (State, IoAction<usize>) {
        match state {
            0 => (1, IoAction::WaitSecs(1.0)),
            1 => (2, IoAction::BroadcastWait(123)),
            2 => {
                if self.i == 0 {
                    (3, IoAction::Call(Box::new(DoAsk::call(String::from("Hello")))))
                } else {
                    self.i -= 1;
                    (2, IoAction::LoopYield)
                }
            }
            3 => (3, IoAction::None),
            _ => unreachable!()
        }
    }
}

// type Callback<A: FnOnce(&mut TestCtx) -> Option<IoAction<M>, Callback<M>>, B, M: Copy> = A;

type Action<M> = dyn FnMut(&mut TestCtx) -> Callback<M>;
enum Callback<M: Copy> {  // this is equivalent to IO above
    Func(IoAction<M>, Box<Action<M>>),
    Again(IoAction<M>),
    None,
}

// you can even generate this in one pass i think,
// just anytime you hit an async call, you add another level of indentation.
// async fn (ctx, count): A; wait(1).await; B; broadcast(123).await; for i in 0..count { C; yield().await; }; D; DoAsk("Hello").await; return;
//
fn other_do_something(count: usize) -> Callback<usize> {
    // A
    Callback::Func(IoAction::WaitSecs(1.0), Box::new(move |ctx| {
        // B
        // ... do non async work
        let mut i = count;
        Callback::Func(IoAction::BroadcastWait(123), Box::new(move |ctx| {
            // ... do non async work
            if i == 0 {
                // D
                Callback::Func(IoAction::Call(Box::new(DoAsk::call(String::from("Hello")))), Callback::empty())
            } else {
                // C
                i -= 1;
                Callback::Again(IoAction::LoopYield)
            }
        }))
    }))
}

fn other_exe(mut f: Box<Action<usize>>, ctx: &mut TestCtx) {
    loop {
        let callback = f(ctx);
        match callback {
            Callback::Func(action, next) => {
                assert!(!matches!(action, IoAction::None), "unnecessary in this version");
                black_box(action);
                f = next;
            }
            Callback::None => break,
            Callback::Again(action) => {
                assert!(!matches!(action, IoAction::None), "unnecessary in this version");
                black_box(action);
            },
        }
    }
}

impl<M: Copy> Callback<M> {
    fn empty() -> Box<Action<M>> {
        Box::new(|ctx: &mut TestCtx| {
            Callback::None
        })
    }
}

impl DoSomething {
    fn call() -> Self {
        DoSomething {
            i: 0
        }
    }
}

impl DoAsk {
    fn call(args: String) -> Self {
        DoAsk(args)
    }
}

struct DoAsk(String);
impl ScratchFn<usize> for DoAsk {
    fn run(&mut self, state: State, ctx: &mut TestCtx) -> (State, IoAction<usize>) {
        if state == 0 {
            (1, IoAction::Ask(String::from("Hi")))
        } else {
            (2, IoAction::None)
        }
    }
}

enum ActionStack<M: Copy> {
    Concurrent(Vec<ActionStack<M>>),
    Sequential(Vec<ActionStack<M>>),
    Single(IoAction<M>)
}

fn exec<F: ScratchFn<usize>>(mut f: F, ctx: &mut TestCtx) {
    let mut state = 0usize;
    loop {
        let (next_state, action) = f.run(0, ctx);
        state = next_state;
        match action {
            IoAction::None => break,
            _ => {},
        }
    }
}

//
// struct TestSprite();
// struct TestCtx();
// struct SleepFut();
//
// impl TestSprite {
//     async fn do_something(&mut self, ctx: &mut TestCtx) {
//         ctx.wait(1).await;
//     }
// }
//
// impl TestCtx {
//     async fn wait(&mut self, s: usize) {
//         SleepFut().await
//     }
// }
//
// impl Future for SleepFut {
//     type Output = ();
//
//     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         todo!()
//     }
// }
//
// use std::future::Future;
// use std::ptr;
// use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
//
// const vtable: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
//
// fn run_many(tests: &mut [TestSprite], ctx: &mut TestCtx) {
//     let waker = unsafe { Waker::from_raw(RawWaker::new(ptr::null(), &vtable)) };
//     let mut futctx = Context::from_waker(&waker);
//     for sprite in tests {
//         let fut = pin!(sprite.do_something(ctx));
//         fut.poll(&mut futctx);
//     }
// }
//
//
// unsafe fn clone(data: *const ()) -> RawWaker {
//     RawWaker::new(ptr::null(), &vtable)
// }
//
// unsafe fn wake(data: *const ()) {
//
// }
//
// unsafe fn wake_by_ref(data: *const ()) {
//
// }
//
// unsafe fn drop(data: *const ()) {
//
// }
//
