use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Result, Value};

#[derive(Serialize, Deserialize, Debug)]
pub struct ScratchProject {
    pub targets: Vec<Sprite>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Sprite {
    pub isStage: bool,
    pub name: String,
    pub variables: Map<String, Value>,
    pub blocks: HashMap<String, Block>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block {
    pub opcode: String,
    pub next: Option<String>,
    pub parent: Option<String>,
    pub inputs: Value,
    pub fields: Value
}

pub fn parse(raw_json: &str) -> Result<ScratchProject> {
    serde_json::from_str(raw_json)
}
