mod parse_project;

use std::collections::HashMap;
use std::fs;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Result, Value};
use crate::parse_project::parse;


pub fn typed_example(){
    let raw = fs::read_to_string("/Users/luke/Documents/scratch.json").unwrap();
    let project = parse(raw.as_str()).unwrap();
    println!("{:?}", project);
}
 