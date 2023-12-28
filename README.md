# hctarcs: Compile [Scratch](https://scratch.mit.edu) to Rust 

> Compiling scratch projects to native executables just seems backwards! 

- Implemented: Arithmetic & Logic, Global and Sprite Variables, Custom Blocks, Movement (goto), Pen (Single Pixels Only)
- Missing: Looks, Sound, Broadcasts, Wait, Rotation, Glide, Bounce, Cloning, Lists, User Input (keyboard & mouse), Loop Yielding 
- (Almost) Working Projects: [Johan-Mi/linrays](https://scratch.mit.edu/projects/726052645)

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
