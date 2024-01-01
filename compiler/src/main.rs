use std::fs;
use std::fs::create_dir_all;
use std::process::Command;
use compiler::ast::Project;
use compiler::backend::rust::{CARGO_TOML, emit_rust};
use compiler::scratch_schema::parse;

fn main() {
    let linrays = compile("target/linrays", "out/gen/linrays");
    compile("target/tres", "out/gen/tres");
    assert_eq!(linrays.matches("Poly").count(), 1, "You broke type inference.");

    compile("target/sanity", "out/gen/sanity");  // TODO: assert that passes
    compile("target/mandelbrot", "out/gen/mandelbrot");
}

fn compile(input: &str, output: &str) -> String{
    let raw = fs::read_to_string(format!("{input}/project.json")).unwrap();
    let project = parse(raw.as_str()).unwrap();
    let project: Project = project.into();
    for (i, name) in project.var_names.iter().enumerate() {
        //println!("   - VarId({i}) = {name} is a {:?}", project.expected_types[i])
    }
    let result = emit_rust(&project);
    create_dir_all(format!("{output}/src")).unwrap();
    fs::write(format!("{output}/src/main.rs"), &result).unwrap();
    let cargotoml = CARGO_TOML.replace("scratch_out", &input.replace("/", "_"));
    fs::write(format!("{output}/Cargo.toml"), cargotoml).unwrap();

    println!("[INFO] {} Polymorphic Type Annotations", result.matches(": Poly").count());
    println!("[INFO] {} Polymorphic Constructor Calls", result.matches("Poly::from").count());

    // Note: Starting a new process hangs forever when running in RustRover's profiler???
    println!("Running cargo fmt...");
    assert!(Command::new("cargo").arg("fmt").current_dir(output).status().unwrap().success());
    println!("Running cargo check...");
    assert!(Command::new("cargo").arg("check").current_dir(output).output().unwrap().status.success());
    assert_eq!(result.matches("Poly::from(Poly::from").count(), 0, "redundant Poly construct");

    // TODO
    // assert_eq!(result.matches("Str::from(Str::from").count(), 0, "redundant Str construct");
    // assert_eq!(result.matches(".clone().as_str()").count(), 0, "redundant clone");
    // assert_eq!(result.matches(".clone().as_num()").count(), 0, "redundant clone");
    result
}
