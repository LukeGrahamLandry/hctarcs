use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::scratch_schema::Costume;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
    pub targets: Vec<Sprite>,
    pub var_names: Vec<String>,
    pub expected_types: Vec<Option<SType>>,
    pub triggers_by_name: HashMap<String, VarId>,
    pub any_async: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sprite {
    pub scripts: Vec<Func>,
    pub procedures: Vec<Proc>,
    pub fields: Vec<VarId>,
    pub field_defaults: Vec<Option<Expr>>,
    pub name: String,
    pub is_stage: bool,
    pub is_singleton: bool,
    pub costumes: Vec<Costume>,
    pub any_async: bool,
}

/// A stack of scratch blocks with a trigger
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Func {
    pub start: Trigger,
    pub body: Vec<Stmt>,
    pub needs_async: bool
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Proc {
    pub name: String,
    pub body: Vec<Stmt>,
    pub args: Vec<VarId>,
    pub needs_async: bool
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    pub op: Stmt
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Stmt {
    // Control
    RepeatTimes(Expr, Vec<Stmt>),
    If(Expr, Vec<Stmt>),
    RepeatUntil(Expr, Vec<Stmt>),
    IfElse(Expr, Vec<Stmt>, Vec<Stmt>),
    // WaitUntil(Expr),
    StopScript,
    Exit,
    RepeatTimesCapture(Expr, Vec<Stmt>, VarId, Scope),

    CloneMyself,
    WaitSeconds(Expr),
    AskAndWait(Expr),

    // Variables
    SetField(VarId, Expr),
    SetGlobal(VarId, Expr),
    ListSet(Scope, VarId, Expr, Expr),
    ListPush(Scope, VarId, Expr),
    ListClear(Scope, VarId),
    ListRemoveIndex(Scope, VarId, Expr),

    // Other
    BuiltinRuntimeCall(String, Vec<Expr>),
    CallCustom(String, Vec<Expr>),  // TODO: func name should be a VarId
    BroadcastWait(Expr),
    UnknownOpcode(String),
    Empty
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Duration {
    Default,
    Secs(f64)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Expr {
    Bin(BinOp, Box<Expr>, Box<Expr>),
    Un(UnOp, Box<Expr>),
    GetField(VarId),
    GetGlobal(VarId),
    GetArgument(VarId),
    ListGet(Scope, VarId, Box<Expr>),
    ListLen(Scope, VarId),
    StringGetIndex(Box<Expr>, Box<Expr>),
    Empty,
    IsNum(Box<Expr>),

    BuiltinRuntimeGet(String),
    Literal(String),  // TODO: parse it in parser
    UnknownExpr(String),
    ListLiteral(Vec<f64>),
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    GT,
    LT,
    EQ,
    And,
    Or,
    Pow,
    Mod,
    StrJoin,
    // StrLetterOf,
    // StrContains,
    Random
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub enum Scope {
    Instance,
    Global,
    Argument,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum UnOp {
    Not,
    StrLen,
    SuffixCall(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash, Copy)]
pub enum SType {
    // Integer,
    Number,
    Bool,
    Str,
    ListPoly,
    Poly
}

// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub enum RotStyle {
//     LeftRight,
//     DontRotate,
//     AllAround
// }
//
// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub enum PosTarget {
//     Random,
//     Mouse,
//     Sprite(SpriteId),
//     Pos { x: f64, y: f64 },
// }

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum Trigger {
    FlagClicked,
    SpriteClicked,
    /// Corresponding name is NOT a safe_str
    Message(VarId),
    // KeyPressed(KeyId),
    // ThisSpriteClicked,
    // BackdropSwitch(BackdropId),
    // MessageReceive(EventId)
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum CallType {
    Suspend,
    NoSuspend,
}

macro_rules! int_key {
    ($name:ident) => {
        #[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
        pub struct $name(pub usize);
    };
}

// int_key!(BackdropId);
int_key!(VarId);
// int_key!(EventId);
// int_key!(SpriteId);
// int_key!(KeyId);


impl Sprite {
    // TODO: hash table
    pub fn lookup_proc(&self, name: &str) -> Option<&Proc> {
        self.procedures.iter().find(|p| p.name == name)
    }
}

