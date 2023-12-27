//! Raw structure of a scratch project json file.

#![allow(non_snake_case)]
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Result, Value};

#[derive(Serialize, Deserialize, Debug)]
pub struct ScratchProject {
    pub targets: Vec<RawSprite>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RawSprite {
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
    pub inputs: Option<Input>,
    pub fields: Option<Field>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Input {
    NumBin { // math
        NUM1: Operand,
        NUM2: Operand,
    },
    NumUn {
        NUM: Operand,
    },
    Operands {  // logic
        OPERAND1: Operand,
        OPERAND2: Operand,
    },
    Cond {  // if
        CONDITION: Operand,
    },
    Val {  // variable set
        VALUE: Operand,
    },
    Custom {
        custom_block: Operand,
    },
    Pos {
        X: Operand,
        Y: Operand,
    },
    Range {
        FROM: Operand,
        TO: Operand,
    },
    // TODO: how to match empty?
    // Empty {},  // This matches everything, not just empty
    Named(HashMap<String, Operand>),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Field {
    Val {
        VALUE: Operand,
    },
    Var {
        VARIABLE: Operand,
    },
    Op {
        OPERATOR: Operand,
    },
    Stop {
        STOP_OPTION: Operand
    },
    // TODO: How to match empty?
    Named(HashMap<String, Operand>),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Operand {
    ExprRef(usize, String),
    ExprRefExtra(usize, String, (usize, String)),
    ArgRef(usize, (usize, String, String), (usize, String)),
    Constant(usize, (usize, String)),
    ArgName(String, String),
    Var(String, Option<()>),
    // Unknown(Value)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ArgRef(usize, String, String);

impl Operand {
    pub fn block(&self) -> Option<&str> {
        match self {
            Operand::ExprRef(_, s) => Some(s.as_str()),
            Operand::ExprRefExtra(_, s, _) => Some(s.as_str()),
            _ => None
        }
    }

    pub fn constant(&self) -> Option<&str> {
        match self {
            Operand::Constant(_, (_, s)) => Some(s.as_ref()),
            _ => None
        }
    }
}

pub fn parse(raw_json: &str) -> Result<ScratchProject> {
    serde_json::from_str(raw_json)
}
