//! Raw structure of a scratch project json file.

#![allow(non_snake_case)]
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Result;

#[derive(Serialize, Deserialize, Debug)]
pub struct ScratchProject {
    pub targets: Vec<RawSprite>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RawSprite {
    pub isStage: bool,
    pub name: String,
    pub variables: HashMap<String, Operand>,
    pub lists: HashMap<String, Operand>,
    pub blocks: HashMap<String, Block>,
    pub costumes: Vec<Costume>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Costume {
    pub name: String,
    pub md5ext: String,
    pub dataFormat: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block {
    pub opcode: String,
    pub next: Option<String>,
    pub parent: Option<String>,
    pub inputs: Option<Input>,
    pub fields: Option<Field>,
    pub mutation: Option<Mutation>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Mutation {
    // TODO: how do i deal with nested json? These are lists of strings.
    pub argumentdefaults: Option<String>,
    pub argumentids: Option<String>,
    pub argumentnames: Option<String>,
    pub warp: Option<BoolOrString>,  // TODO: ffs

    pub proccode: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum BoolOrString {
    B(bool),
    S(String),
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
    Un {
        OPERAND: Operand,
    },
    Operands {  // logic
        OPERAND1: Operand,
        OPERAND2: Operand,
    },
    // Order matters because Branch2 extends Branch1!
    Branch2 {
        CONDITION: Operand,
        SUBSTACK: Operand,
        SUBSTACK2: Operand,
    },
    Branch1 {
        CONDITION: Operand,
        SUBSTACK: Operand
    },
    ForLoop {
        TIMES: Operand,
        SUBSTACK: Operand
    },
    SecretForLoop {
        VALUE: Operand,
        SUBSTACK: Operand
    },
    Val {  // variable set
        VALUE: Operand,
    },
    Custom {
        custom_block: Operand,
    },
    Broadcast {
        BROADCAST_INPUT: Operand
    },
    Pos {
        X: Operand,
        Y: Operand,
    },
    Range {
        FROM: Operand,
        TO: Operand,
    },
    Colour {
        COLOR: Operand,
    },
    ListBoth {
        INDEX: Operand,
        ITEM: Operand,
    },
    ListItem {
        ITEM: Operand,
    },
    ListIndex {
        INDEX: Operand,
    },
    CharStr {
        LETTER: Operand,
        STRING: Operand,
    },
    StrPair {
        STRING1: Operand,
        STRING2: Operand,
    },
    Str {
        STRING: Operand,
    },
    Size {
        SIZE: Operand,
    },
    Costume {
        COSTUME: Operand
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
    Msg {
        BROADCAST_OPTION: Operand,
    },
    List {
        LIST: Operand,
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
    VarNum(String, usize),
    VarF(String, f64),
    Var(String, Option<()>),
    TwoNames(usize, String, String),
    Nothing(usize, Option<()>),
    NNSS(usize, (usize, String, String)),
    List(String, Vec<()>),
    // Unknown(Value)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ArgRef(usize, String, String);

impl Operand {
    pub fn opt_block(&self) -> Option<&str> {
        match self {
            Operand::ExprRef(_, s) => Some(s.as_str()),
            Operand::ExprRefExtra(_, s, _) => Some(s.as_str()),
            Operand::Nothing(_, _) => None,
            _ => panic!("Expected opt_block found {:?}, maybe could return None but should check", self),
        }
    }

    pub fn constant(&self) -> Option<&str> {
        match self {
            Operand::Constant(_, (_, s)) => Some(s.as_ref()),
            _ => None
        }
    }

    pub fn unwrap_var(&self) -> &str {
        match self.opt_var() {
            Some(s) => s,
            None => panic!("Failed to unwrap operand var {self:?}"),
        }
    }

    pub fn opt_var(&self) -> Option<&str> {
        match self {
            Operand::Var(s, _) |
            Operand::ArgName(s, _) |
            Operand::VarF(s, _) |
            Operand::List(s, _) |
            Operand::VarNum(s, _) |
            Operand::ArgRef(_, (_, s, _), _) => Some(s),
            Operand::NNSS(_, (_, s, _)) => Some(s),
            _ => None,
        }
    }
}

impl Input {
    pub fn unwrap_one(&self) -> &Operand {
        match self {
            Input::NumUn { NUM } => NUM,
            Input::Val { VALUE } => VALUE,
            Input::Colour { COLOR } => COLOR,
            Input::Size { SIZE } => SIZE,
            Input::Costume { COSTUME } => COSTUME,
            Input::Custom { custom_block } => custom_block,
            Input::Named(vals) => if vals.len() == 1 {
                    vals.iter().next().unwrap().1
                } else {
                    panic!("Expected single Operand in Input but found {:?}", self)
                }
            _ => panic!("Expected single Operand in Input but found {:?}", self)
        }
    }

    pub fn unwrap_pair(&self) -> (&Operand, &Operand) {
        match self {
            Input::NumBin { NUM1, NUM2 } => (NUM1, NUM2),
            Input::Operands { OPERAND1, OPERAND2 } => (OPERAND1, OPERAND2),
            Input::Range { FROM, TO } => (FROM, TO),
            Input::Pos { X, Y } => (X, Y),
            _ => panic!("Expected two Operand in Input but found {:?}", self)
        }
    }
}

pub enum StopOp {
    ThisScript,
    All,
}

impl Field {
    // TODO: damn this sucks, can i offload some to serde?
    pub fn unwrap_stop(&self) -> StopOp {
        match self {
            Field::Stop { STOP_OPTION } => {
                match STOP_OPTION {
                    Operand::Var(name, _) => {
                        match name.as_str() {
                            "this script" => StopOp::ThisScript,
                            "all" => StopOp::All,
                            _ => panic!("Expected StopOp found {name}"),
                        }
                    }
                    _ => panic!("Expected Var")
                }
            }
            _ => panic!("Expected Stop found {:?}", self)
        }
    }
}

impl Mutation {
    pub fn arg_ids(&self) -> Vec<String> {
        serde_json::from_str(self.argumentids.as_ref().unwrap()).unwrap()
    }

    pub fn arg_names(&self) -> Vec<String> {
        serde_json::from_str(self.argumentnames.as_ref().expect("Func Proto")).unwrap()
    }

    pub fn arg_defaults(&self) -> Vec<String> {
        serde_json::from_str(self.argumentdefaults.as_ref().expect("Func Proto")).unwrap()
    }

    pub fn arity(&self) -> usize {
        self.proccode.as_ref().unwrap().chars().filter(|c| *c == '%').count()
    }

    pub fn name(&self) -> &str {  // TODO: kinda hacky, chops off the %d terms
        let s = &self.proccode.as_ref().unwrap().as_str()[0..self.proccode.as_ref().unwrap().len()-(self.arity()*3)];
        assert_eq!(s.chars().filter(|c| *c == '%').count(), 0);
        s
    }
}

pub fn parse(raw_json: &str) -> Result<ScratchProject> {
    serde_json::from_str(raw_json)
}
