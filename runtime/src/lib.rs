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
    _messages: VecDeque<S::Msg>
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
