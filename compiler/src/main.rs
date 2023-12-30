use std::fs;
use std::fs::create_dir_all;
use std::process::Command;
use compiler::ast::Project;
use compiler::backend::rust::{CARGO_TOML, emit_rust};
use compiler::scratch_schema::parse;

fn main() {
    compile("target/linrays", "target/scratch_out");
    compile("target/tres", "target/scratch_out_tres");
}

fn compile(input: &str, output: &str) {
    let raw = fs::read_to_string(format!("{input}/project.json")).unwrap();
    let project = parse(raw.as_str()).unwrap();
    let project: Project = project.into();
    println!();
    println!("{:?}", project);
    println!();
    create_dir_all(format!("{output}/src")).unwrap();
    fs::write(format!("{output}/src/main.rs"), emit_rust(&project)).unwrap();
    fs::write(format!("{output}/Cargo.toml"), CARGO_TOML).unwrap();
    assert!(Command::new("cargo").arg("fmt").current_dir(output).status().unwrap().success());
}
