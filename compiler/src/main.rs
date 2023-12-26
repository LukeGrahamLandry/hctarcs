use std::fs;
use compiler::scratch_schema::parse;

fn main() {
    let raw = fs::read_to_string("/Users/luke/Documents/scratch.json").unwrap();
    let project = parse(raw.as_str()).unwrap();
    println!("{:?}", project);
}
