use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
    pub targets: Vec<Sprite>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sprite {
    pub functions: Vec<Func>
}

/// A stack of scratch blocks with a trigger
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Func {
    pub start: Trigger,
    pub body: Vec<Stmt>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    pub op: Stmt
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Stmt {
    // Motion
    MoveSteps(f64),
    TurnLeftDeg(f64),
    TurnRightDeg(f64),
    /// Goto or GlideTo
    GoTo(Duration, PosTarget),
    PointAt(PosTarget),
    PointDeg(f64),
    IfEdgeBounce,
    SetRotStyle(RotStyle),

    // Looks
    Say(Duration, String),
    Think(Duration, String),
    SetVisible(bool),

    // Events
    MessageSend(EventId),
    MessageSendWait(EventId),

    // Control
    DeleteThisClone,
    // Sensing
    AskAndWait(String),
    SetDraggable(bool),

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
    Un(BinOp, Box<Expr>),
    GetVar(VarId),
    GetBuiltin(BuiltinVar)
}

// TODO: separate types?
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BuiltinVar {
    Timer,
    Loudness,
    Volume,

    // Sprite Locals
    Size,
    XPos,
    YPos,
    Direction,

    // Time
    DaysSince2000,
    Username,
    Year,
    Month,
    Day,
    WeekDay,
    Hour,
    Minute,
    Second,

    // Input
    LastAnswer,
    MouseX,
    MouseY,
    IsMouseDown,
    IsKeyPressed(KeyId)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    GT,
    LT,
    EQ,
    And,
    Or,
    StrJoin,
    StrLetterOf,
    StrContains
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum UnOp {
    Not,
    Round,
    Abs,
    Floor,
    Ceil,
    Sqrt,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Ln,
    PowE,
    Pow10
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RotStyle {
    LeftRight,
    DontRotate,
    AllAround
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PosTarget {
    Random,
    Mouse,
    Sprite(SpriteId),
    Pos { x: f64, y: f64 },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Trigger {
    FlagClicked,
    KeyPressed(KeyId),
    ThisSpriteClicked,
    BackdropSwitch(BackdropId),
    MessageReceive(EventId)
}

macro_rules! int_key {
    ($name:ident) => {
        #[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
        pub struct $name(usize);
    };
}

int_key!(BackdropId);
int_key!(VarId);
int_key!(EventId);
int_key!(SpriteId);
int_key!(KeyId);
