#![feature(trait_upcasting)]  // unfortunate that i need nightly

use std::collections::{VecDeque};
use std::fmt::Debug;
use std::mem::size_of;
use std::ops::Add;
use std::time::{Duration};

// Yes I know instant already forwards to std
#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant;
#[cfg(target_arch = "wasm32")]
pub use instant::Instant;

pub mod sprite;
pub mod builtins;
pub mod callback;
pub mod poly;
pub mod backend;

#[cfg(feature = "inspect")]
pub mod ui;

pub use sprite::*;
pub use builtins::*;
pub use poly::*;
pub use backend::*;
pub use callback::*;

pub trait ScratchProgram<R: RenderBackend<Self>>: Sized + 'static {
    type Msg: Debug + Copy + 'static;
    type Globals;
    type UnitIfNoAsync;  // TODO: use const generics?  // TODO: remove if im just forwarding anyway

    fn create_initial_state() -> (Self::Globals, Vec<Box<dyn Sprite<Self, R>>>);

    // TODO: make this part of create_initial_state?
    fn get_costumes() -> Vec<ScratchAsset>;

    // TODO: this is going to move to trait Sprite and the ctx method will accept only the resolved id. ImgId wrapper type?
    fn costume_by_name(name: Str) -> Option<usize>;

    fn get_credits() -> &'static str;
}

/// Types for Msg and Globals are generated for a specific scratch program by the compiler.
/// This crate needs to be generic over the user program but there will only ever be one instantiation of this generic in a given application.
pub struct World<S: ScratchProgram<R>, R: RenderBackend<S>> {
    bases: VecDeque<SpriteBase>,
    custom: VecDeque<Box<dyn Sprite<S, R>>>,
    globals: S::Globals,
    scripts: Vec<Script<S, R>>,
    last_ask_id: usize,  // TODO: non-blocking input that increments this
    pub mode: RunMode,
    events: VecDeque<SEvent>,
    pub futs_this_frame: usize, // Inspect Only. TODO: graph

}

// TODO: make the rendering backend generic over the async backend so you could drop in replace a real async runtime?
// TODO: async is wip. needs massive cleanup.
// I dare you to fix the 'static
impl<S: ScratchProgram<R>, R: RenderBackend<S> + 'static> World<S, R> {
    pub fn new() -> Self {
        let (globals, custom) = S::create_initial_state();

        if args().any(|arg| &arg == "--credits") {
            println!("{}", S::get_credits())
        }
        World {
            bases: vec![SpriteBase::default(); custom.len()].into(),
            custom: custom.into(),
            globals,
            scripts: vec![],
            last_ask_id: 0,
            mode: RunMode::Turbo,  // TODO: pass default on cli for when not inspect
            events: Default::default(),
            futs_this_frame: 0,
        }
    }

    pub fn restart(&mut self) {
        let mode = self.mode;
        *self = Self::new();
        self.mode = mode;
        self.events.push_back(SEvent::UiClearPen);
        self.broadcast_toplevel_async(Trigger::FlagClicked);
    }

    pub fn broadcast(&mut self, render: &mut R::Handle<'_>, msg: Trigger<S::Msg>) {
        if size_of::<S::UnitIfNoAsync>() == 0 {
            self.broadcast_sync(render, msg);
        } else {
            self.broadcast_toplevel_async(msg);
        }
    }

    // TODO: the compiler knows which messages each type wants to listen to.
    //       it could generate a separate array for each and have no virtual calls or traversing everyone on each message
    pub fn broadcast_sync(&mut self, render: &mut R::Handle<'_>, msg: Trigger<S::Msg>) {
        assert_zero_sized::<S::UnitIfNoAsync>();
        let sprites = self.bases.iter_mut().zip(self.custom.iter_mut());
        let globals = &mut self.globals;
        for (sprite, c) in sprites {
            c.receive(&mut FrameCtx {
                sprite,
                globals,
                render,
            }, msg.clone());
        }
    }

    // TODO: there would also be a version for broadcast and wait that adds listeners to its personal stack.
    pub fn broadcast_toplevel_async(&mut self, msg: Trigger<S::Msg>) {
        for (owner, c) in self.custom.iter().enumerate() {
            let action = c.receive_async(msg);
            self.scripts.push(Script {
                next: vec![IoAction::Call(action)],
                owner,
            });
        }
    }

    // TODO: remove cause this is almost never what you want
    /// Hang the thread until all scripts are finished OR waiting on timers.
    /// This is kinda like "Run without screen refresh" (but for everything, not just some custom blocks) and skipping timers.
    /// This will hang forever if the program has an infinite loop.
    pub fn poll_until_waiting(&mut self, render: &mut R::Handle<'_>) {
        while self.poll(render) {
            // Spin
        }
    }

    pub fn run_frame(&mut self, render: &mut R::Handle<'_>) {
        self.futs_this_frame = 0;
        for e in self.events.drain(0..) {
            match e {
                SEvent::UiClearPen => render.pen_clear(),
            }
        }

        match self.mode {
            RunMode::Turbo => self.poll_turbo(render),
            RunMode::Compat => {
                self.poll(render);
            }
            RunMode::Manual(req) => {
                if req {
                    self.poll(render);
                    self.mode = RunMode::Manual(false);
                }
            }
        }
    }

    // TODO: this is unfortunate: i imagine getting the time is slow as fuck. can i have like waker thingy in another thread?
    /// Polls as many times as possible within one frame or until all scripts are waiting on timers.
    pub fn poll_turbo(&mut self, render: &mut R::Handle<'_>) {
        if self.scripts.is_empty() { return; }  // fast path for sync or finished programs

        // TODO: tune this number. i guess need to leave some time in the frame for rendering?
        // TODO: I really have no sense of scale of how much work you're supposed to do in a frame
        let stop_time = Instant::now().add(Duration::from_millis(16));

        loop {
            let mut progress = true;
            // TODO: tune this number
            for _ in 0..100 {  // I want to check the time less often
                progress = self.poll(render);
                if !progress {
                    break
                }
            }
            if (!progress) || (Instant::now() > stop_time) {
                break
            }
        }
    }

    // TODO: wrong! each script needs to be pushing to its own stack. scripts should be in a Concurrent(...)
    // TODO: all this async stuff should really go in callback cause that's kinda empty. maybe rename that world and try to get to mostly empty lib file.
    // TODO: this should not return while executing a custom block with "run without screen refresh"
    // TODO: compiler warning if you sleep in a "run without screen refresh" block
    // This is the big sexy switch statement at the heart of the universe.
    // TODO: should try to break these up into methods but can't borrow self
    /// Allows each suspended script to run to the next await point if its action has resolved.
    pub fn poll(&mut self, render: &mut R::Handle<'_>) -> bool {
        if self.scripts.is_empty() { return false; }  // fast path for sync or finished programs
        let mut made_progress = false;  // Set to true if every script was just waiting on io/timer

        self.scripts.retain_mut(|c| {
            // (break) to retain and yield the script until next poll.
            // (continue) when future resolved and want to pop the next immediately.
            loop {  // TODO: replace the body of the loop with a method returning enum[made_progress, return] (true, false)=ScriptFinished   (true, false)=ProgressYield   (false, true)=WaitingYield (false, false)=unreachable
                // println!("{:?}\n=======", c.next);

                #[cfg(feature = "inspect")]
                {
                    self.futs_this_frame += 1;
                }

                let current = c.next.pop();
                // println!("{current:?}");
                match current {  // Take the next thing from the stack of futures we're waiting on.
                    None => {  // If its empty, this script is finished.
                        made_progress = true;
                        return /*from closure*/ false
                    },
                    Some(action) => match action {  // Poll the future.
                        IoAction::Call(mut f) => {  // Call some other async function.
                            let sprite = &mut self.bases[c.owner];
                            // Note the deref! I want to type erase the contents of the box not the box itself.
                            let custom: &mut dyn Sprite<S, R> = &mut *self.custom[c.owner];  // This is what needs trait_upcasting
                            let ctx = &mut FrameCtx {
                                sprite,
                                globals: &mut self.globals,
                                render,
                            };

                            let (action, next) = f(ctx, custom);
                            // println!("Returned {action:?} {next:?}");

                            // Its a stack, so push the next handler and then push the action that must resolve before calling it.
                            match next {
                                Callback::Then(callback) => c.next.push(IoAction::Call(callback)),
                                Callback::Again => c.next.push(IoAction::Call(f)),
                                Callback::Done => {}
                            }
                            c.next.push(action);
                            made_progress = true;
                            break
                        },

                        // TODO: this might be waiting two frames since fn return already yields so maybe should be treated as IoAction::None
                        IoAction::LoopYield => { // Yields instantly resolve
                            made_progress = true;
                            break  // But there could be more in the script's stack.
                        },
                        // TODO: ideally the compiler wouldn't ever emit these since they dont do anything.
                        IoAction::None => {
                            made_progress = true;
                            continue
                        }
                        // TODO: could the compiler do these without allocating the vec since im just appending to one I already have anyway? maybe have a method on the Ctx to push actions?
                        IoAction::Sequential(actions) => {
                            c.next.extend(actions.into_iter().rev());  // Reverse for because c.next is a stack
                            made_progress = true;
                            continue
                        }
                        IoAction::sleep(seconds) => {
                            if seconds > 0.0 {
                                // Note: not from_seconds because dont want to round down to zero
                                let replace = IoAction::WaitUntil(Instant::now().add(Duration::from_millis((seconds * 1000.0)as u64)));
                                made_progress = true;
                                c.next.push(replace);
                                break
                            } else {  // TODO: does scratch yield for sleep(0)?
                                continue
                            }
                        }
                        IoAction::WaitUntil(time) => {
                            let now = Instant::now();
                            if now <= time {  // Still waiting, no progress
                                c.next.push(IoAction::WaitUntil(time));
                                break
                            } else {
                                made_progress = true;
                                continue
                            }
                        }
                        IoAction::Ask(question) => {
                            made_progress = true;
                            // TODO: what if another script is already waiting on an ask?
                            println!("ASK: {question}");
                            c.next.push(IoAction::WaitForAsk(self.last_ask_id + 1));
                            break
                        }
                        IoAction::WaitForAsk(id) => {
                            if self.last_ask_id < id {  // Not done yet, no progress
                                println!("TODO: WaitForAsk will never resolve. implement stdin polling");
                                c.next.push(IoAction::WaitForAsk(id));
                                break
                            } else {
                                made_progress = true;
                                continue
                            }
                        }

                        IoAction::BroadcastWait(_) => todo!(),
                        IoAction::CloneMyself => todo!(),
                        IoAction::Concurrent(actions) => {
                            // TODO: this is very wrong but works for now. if i dont guarantee execution order this isn't technically invalid.
                            c.next.extend(actions.into_iter());
                            made_progress = true;
                            continue
                        }
                    }
                }

                #[allow(unreachable_code)]
                { unreachable!("end of loop. use explicit continue cause im afraid of forgetting") }
            }

            // println!("Yield");
            return /*from closure*/  true
        });

        made_progress
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum RunMode {
    Turbo,
    Compat,
    Manual(bool),
}

// TODO: UNUSED thus far
#[allow(dead_code)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum PollState {
    Waiting,
    Finished,
    Working,
}

// TODO: many of the handy render libraries have some asset loading system, is it worth trying to hook in to that?
pub enum ScratchAsset {
    Embed(&'static [u8]),

    #[cfg(any(feature = "fetch-assets", target_arch = "wasm32"))]
    FetchUrl(&'static str),
}

impl ScratchAsset {
    pub fn get<F: FnOnce(&[u8]) -> R, R>(&self, f: F) -> R {
        match self {
            ScratchAsset::Embed(bytes) => f(bytes),

            #[cfg(all(feature = "fetch-assets", not(target_arch = "wasm32")))]
            ScratchAsset::FetchUrl(url) => {
                use std::io::Read;
                let mut res = ureq::get(url)
                    .set("User-Agent", "github/lukegrahamlandry/hctarcs/runtime")
                    .call().unwrap().into_reader();
                let mut buf = Vec::<u8>::new();
                res.read_to_end(&mut buf).unwrap();

                f(&buf)
            },

            // TODO: what about wasi?
            #[cfg(target_arch = "wasm32")]
            ScratchAsset::FetchUrl(url) => todo!("use browser fetch api?"),
        }
    }
}


#[cfg(not(target_arch = "wasm32"))]
pub fn args() -> impl Iterator<Item=String> {
    std::env::args()
}
#[cfg(target_arch = "wasm32")]
pub fn args() -> impl Iterator<Item=String> {
    vec![].into_iter()
}

enum SEvent {
    UiClearPen
}
