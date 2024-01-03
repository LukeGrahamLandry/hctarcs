use std::borrow::Borrow;
use std::collections::{VecDeque};
use std::env;

pub mod sprite;
pub mod builtins;
pub mod callback;
pub mod poly;
pub mod backend;

pub use sprite::*;
pub use builtins::*;
pub use poly::*;
pub use backend::*;
use crate::callback::{Action, Callback, IoAction, Script};

pub trait ScratchProgram<R: RenderBackend<Self>>: Sized + 'static {
    type Msg: Copy;
    type Globals;

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

impl<S: ScratchProgram<R>, R: RenderBackend<S>> World<S, R> {
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

    // TODO: the compiler knows which messages each type wants to listen to.
    //       it could generate a separate array for each and have no virtual calls or traversing everyone on each message
    pub fn broadcast(&mut self, render: &mut R::Handle<'_>, msg: Trigger<S::Msg>) {
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

    // UNUSED thus far
    // TODO: there would also be a version for broadcast and wait that adds listeners to its personal stack.
    pub fn broadcast_toplevel_async(&mut self, render: &mut R::Handle<'_>, msg: Trigger<S::Msg>) {
        let sprites = self.bases.iter_mut().zip(self.custom.iter_mut());
        let globals = &mut self.globals;
        for (owner, (sprite, c)) in sprites.enumerate() {
            let action = Self::stub_async_receive(c, &mut FrameCtx {
                sprite,
                globals,
                render,
            }, msg.clone());  // TODO: this shouldn't need access to the Ctx
            self.scripts.push(Script {
                next: vec![IoAction::Call(action)],
                owner,
            });
        }
    }

    // UNUSED thus far
    // TODO: this should not return while executing a custom block with "run without screen refresh"
    // TODO: compiler warning if you sleep in a "run without screen refresh" block
    /// Allows each suspended script to run to the next await point if its action has resolved.
    pub fn poll(&mut self, render: &mut R::Handle<'_>) {
        let globals = &mut self.globals;
        self.scripts.retain_mut(|c| {
            match c.next.pop() {  // Take the next thing from the stack of futures we're waiting on.
                None => false,  // If its empty, this script is finished.
                Some(action) => match action {  // Poll the future.
                    IoAction::Call(mut f) => {  // Call some other async function.
                        let sprite = &mut self.bases[c.owner];
                        let ctx = &mut FrameCtx {
                            sprite,
                            globals,
                            render,
                        };

                        let (action, next) = f(ctx);

                        // Its a stack, so push the next handler and then push the action that must resolve before calling it.
                        match next {
                            Callback::Then(callback) => c.next.push(IoAction::Call(callback)),
                            Callback::Again => c.next.push(IoAction::Call(f)),
                            Callback::None => {}
                        }
                        c.next.push(action);
                        true
                    },
                    IoAction::LoopYield | IoAction::None => true,  // Yields instantly resolve.
                    _ => todo!(),
                }
            }
        });
    }

    // UNUSED thus far
    fn stub_async_receive(s: &mut Box<dyn Sprite<S, R>>, ctx: &mut FrameCtx<S, R>, msg: Trigger<S::Msg>) -> Box<Action<S, R>> {
        todo!("implement this in the compiler")
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
