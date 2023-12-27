use std::fs;
use compiler::ast::Project;
use compiler::scratch_schema::parse;

fn main() {
    // let raw = fs::read_to_string("/Users/luke/Documents/scratch.json").unwrap();
    let raw = fs::read_to_string("target/linrays/project.json").unwrap();
    let project = parse(raw.as_str()).unwrap();
    println!("{:?}", project);
    let project: Project = project.into();
    println!();
    println!("{:?}", project);
}
