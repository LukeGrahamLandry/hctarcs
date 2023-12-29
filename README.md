# hctarcs: Compile [Scratch](https://scratch.mit.edu) to Rust 

> Compiling scratch projects to native executables just seems backwards! 

- Implemented: Arithmetic & Logic, Global and Sprite Variables, Custom Blocks, Movement (goto), Pen (Single Pixels Only)
- Missing: Looks, Sound, Broadcasts, Wait, Rotation, Glide, Bounce, Cloning, Lists, User Input (keyboard & mouse), Loop Yielding 
- (Almost) Working Projects: [Johan-Mi/linrays](https://scratch.mit.edu/projects/726052645)

BEWARE: There's no sandbox-ing. Only run projects on your computer if you understand what they're doing. 
*If you can think of a way to get arbitrary code execution with this, please let me know!*

<!--
## Build

- [Install Rust](https://www.rust-lang.org/tools/install)
- `git clone "https://github.com/LukeGrahamLandry/hctarcs.git" && cd hctarcs`
- `cargo build --release`

## Usage

- Export your scratch project to a .sb3 file.
- `cargo run --release --bin compiler`
- `cd target/scratch_out`
- `cargo run`
-->

## Goals

- Faster than scratch-vm and faster than turbowarp
- Target native smaller than turbowarp (both electron (lol) and webview)
- Target wasm smaller than turbowarp's embed 

## How It Works: Compiler

Wasm Build: `cargo build --release --target wasm32-unknown-unknown --bin compiler`

## How It Works: Runtime

Unlike TurboWarp, this does not include any code from the original Scratch runtime.  
winit creates a window and softbuffer puts pixels on it. 
