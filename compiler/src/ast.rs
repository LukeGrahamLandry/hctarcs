use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
    pub targets: Vec<Sprite>,
    pub var_names: Vec<String>,
    pub expected_types: Vec<Option<SType>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sprite {
    pub functions: Vec<Func>,
    pub procedures: Vec<Proc>,
    pub fields: Vec<VarId>,
    pub name: String,
    pub is_stage: bool,
    pub is_singleton: bool,
}

/// A stack of scratch blocks with a trigger
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Func {
    pub start: Trigger,
    pub body: Vec<Stmt>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Proc {
    pub name: String,
    pub body: Vec<Stmt>,
    pub args: Vec<VarId>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    pub op: Stmt
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Stmt {
    // Control
    // Repeat(Vec<Stmt>),
    RepeatTimes(Expr, Vec<Stmt>),
    If(Expr, Vec<Stmt>),
    IfElse(Expr, Vec<Stmt>, Vec<Stmt>),
    // WaitUntil(Expr),
    // RepeatUntil(Expr, Vec<Stmt>),
    StopScript,
    Exit,
    RepeatTimesCapture(Expr, Vec<Stmt>, VarId, Scope),

    // Variables
    SetField(VarId, Expr),
    SetGlobal(VarId, Expr),
    ListSet(Scope, VarId, Expr, Expr),
    ListPush(Scope, VarId, Expr),
    ListClear(Scope, VarId),
    BroadcastWait(Expr),

    // Other
    BuiltinRuntimeCall(String, Vec<Expr>),
    CallCustom(String, Vec<Expr>),  // TODO: func name should be a VarId
    UnknownOpcode(String)
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

    BuiltinRuntimeGet(String),
    Literal(String),  // TODO: parse it in parser
    UnknownExpr(String)
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
    StrJoin,
    // StrLetterOf,
    // StrContains,
    Random
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub enum Scope {
    Instance,
    Global
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum UnOp {
    Not,
    SuffixCall(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub enum SType {
    // Integer,
    Number,
    Bool,
    Str,
    ListPolymorphic
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

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum Trigger {
    FlagClicked,
    /// NOT a safe_str
    Message(String), // TODO: VarId ?
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
