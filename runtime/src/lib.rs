#![feature(trait_upcasting)]  // unfortunate that i need nightly

use std::any::Any;
use std::collections::{VecDeque};
use std::env;
use std::mem::size_of;
use std::ops::Add;
use std::time::{Duration, Instant};

pub mod sprite;
pub mod builtins;
pub mod callback;
pub mod poly;
pub mod backend;

pub use sprite::*;
pub use builtins::*;
pub use poly::*;
pub use backend::*;
pub use callback::*;

pub trait ScratchProgram<R: RenderBackend<Self>>: Sized + 'static {
    type Msg: Copy + 'static;
    type Globals;
    type UnitIfNoAsync;  // TODO: use const generics?  // TODO: remove if im just forwarding anyway

    fn create_initial_state() -> (Self::Globals, Vec<Box<dyn Sprite<Self, R>>>);

    // TODO: make this part of create_initial_state?
    fn get_costumes() -> Vec<ScratchAsset>;

    // TODO: this is going to move to trait Sprite and the ctx method will accept only the resolved id. ImgId wrapper type?
    fn costume_by_name(name: Str) -> Option<usize>;
}

/// Types for Msg and Globals are generated for a specific scratch program by the compiler.
/// This crate needs to be generic over the user program but there will only ever be one instantiation of this generic in a given application.
pub struct World<S: ScratchProgram<R>, R: RenderBackend<S>> {
    bases: VecDeque<SpriteBase>,
    custom: VecDeque<Box<dyn Sprite<S, R>>>,
    globals: S::Globals,
    _messages: VecDeque<S::Msg>,
    scripts: Vec<Script<S, R>>
}

// I dare you to fix the 'static
impl<S: ScratchProgram<R>, R: RenderBackend<S> + 'static> World<S, R> {
    pub fn new() -> Self {
        let (globals, custom) = S::create_initial_state();
        credits();
        World {
            bases: vec![SpriteBase::default(); custom.len()].into(),
            custom: custom.into(),
            globals,
            _messages: VecDeque::new(),
            scripts: vec![],
        }
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

    // TODO: this is unfortunate: i imagine getting the time is slow as fuck. can i have like waker thingy in another thread?
    /// Polls as many times as possible within one frame or until all scripts are waiting on timers.
    pub fn poll_turbo(&mut self, render: &mut R::Handle<'_>) {
        if self.scripts.is_empty() { return; }  // fast path for sync or finished programs

        let stop_time = Instant::now().add(Duration::from_millis(16));

        loop {
            let mut progress = true;
            for _ in 0..100 {  // I want to check the time less often
                progress = self.poll(render);
                if !progress {
                    break
                }
            }
            if !progress || Instant::now() > stop_time {
                break
            }
        }
    }

    // TODO: this should not return while executing a custom block with "run without screen refresh"
    // TODO: compiler warning if you sleep in a "run without screen refresh" block
    /// Allows each suspended script to run to the next await point if its action has resolved.
    pub fn poll(&mut self, render: &mut R::Handle<'_>) -> bool {
        if self.scripts.is_empty() { return false; }  // fast path for sync or finished programs
        let mut made_progress = false;  // Set to true if every script was just waiting on io/timer
        let globals = &mut self.globals;
        self.scripts.retain_mut(|c| {
            loop {
                match c.next.pop() {  // Take the next thing from the stack of futures we're waiting on.
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
                                globals,
                                render,
                            };

                            let (action, next) = f(ctx, custom);

                            // Its a stack, so push the next handler and then push the action that must resolve before calling it.
                            match next {
                                Callback::Then(callback) => c.next.push(IoAction::Call(callback)),
                                Callback::Again => c.next.push(IoAction::Call(f)),
                                Callback::Done => {}
                            }
                            c.next.push(action);
                            made_progress = true;
                            return /*from closure*/ true
                        },
                        // TODO: maybe None should pop and try again instead of waiting a frame
                        IoAction::LoopYield => { // Yields instantly resolve
                            made_progress = true;
                            return /*from closure*/ true  // But there could be more in the script's stack.
                        },
                        // TODO: ideally the compiler wouldn't ever emit these since they dont do anything.
                        IoAction::None => {  // Instantly resolve + dont wait a frame before popping again
                            made_progress = true;
                        }
                        _ => todo!(),
                    }
                }
            }
        });

        made_progress
    }
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

// TODO: add project name and author if available
fn credits() {
    if env::args().len() > 1 {
        println!("{}", CREDITS)
    }
}

const CREDITS: &str = r#"This program is compiled from a Scratch project using github.com/LukeGrahamLandry/hctarcs
All projects shared on the Scratch website are covered by the Creative Commons Attribution Share-Alike license.
Scratch is a project of the Scratch Foundation, in collaboration with the Lifelong Kindergarten Group at the MIT Media Lab. It is available for free at https://scratch.mit.edu
"#;
