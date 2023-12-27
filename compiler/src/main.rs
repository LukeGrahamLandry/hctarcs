use std::fs;
use std::fs::create_dir_all;
use compiler::ast::Project;
use compiler::backend::rust::{CARGO_TOML, emit_rust};
use compiler::scratch_schema::parse;

fn main() {
    // let raw = fs::read_to_string("/Users/luke/Documents/scratch.json").unwrap();
    let raw = fs::read_to_string("target/linrays/project.json").unwrap();
    let project = parse(raw.as_str()).unwrap();
    println!("{:?}", project);
    let project: Project = project.into();
    println!();
    println!("{:?}", project);
    println!();

    create_dir_all("target/scratch_out/src").unwrap();
    fs::write("target/scratch_out/src/main.rs", emit_rust(&project)).unwrap();
    fs::write("target/scratch_out/Cargo.toml", CARGO_TOML).unwrap();
}
