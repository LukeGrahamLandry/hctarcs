use std::collections::{HashMap, HashSet};
use crate::ast::{BinOp, Expr, Project, Scope, Sprite, Stmt, SType, Trigger, UnOp, VarId};
use crate::parse::{infer_type, safe_str};

pub fn emit_rust(project: &Project) -> String {
    let msgs: HashSet<Trigger> = project.targets.
        iter()
        .map(|target|
            target.functions.iter().map(|f| f.start.clone())
        )
        .flatten()
        .filter(|t| matches!(t, Trigger::Message(_)))
        .collect(); // TODO: Im not convinced this has no duplicates
    let msg_fields: HashSet<String> = msgs.iter().map(|t| {
        let name = match t {
            Trigger::Message(name) => name,
            _ => unreachable!(),
        };
        format!("{}, \n", safe_str(name))
    }).collect();
    let msg_fields: String = msg_fields.into_iter().collect();
    let body: String = project.targets.iter().map(|target| Emit { project, target, triggers: HashMap::new() }.emit()).collect();
    let sprites: String = project.targets
        .iter()
        .filter(|target| !target.is_stage)  // TODO: wrong cause stage can have scripts but im using it as special magic globals so need to rethink.
        .map(|target| format!("Box::new({}::default()), ", target.name))
        .collect();
    let msg_names: String = msgs.iter().map(|t| {
        let name = match t {
            Trigger::Message(name) => name,
            _ => unreachable!(),
        };
        format!("\"{}\"=>Msg::{}, \n", name.escape_default(), safe_str(name))
    }).collect();

    format!(r#"
{HEADER}
fn main() {{
    World::run_program(Stage::default(), vec![{sprites}])
}}
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
enum Msg {{
    {msg_fields}
}}
fn msg_of(value: Cow<str>) -> Msg {{
        match value.as_ref() {{
            {msg_names}
            _ => panic!("Tried to send invalid computed message: {{value}}"),
        }}
}}

    {body}"#)
}

fn format_trigger(project: &Project, value: &Trigger) -> String {
    match value {
        Trigger::FlagClicked => "Trigger::FlagClicked".to_string(),
        Trigger::Message(name) => format!("Trigger::Message(Msg::{})", safe_str(name)),
    }
}

struct Emit<'src> {
    project: &'src Project,
    target: &'src Sprite,
    triggers: HashMap<Trigger, String>
}

pub const CARGO_TOML: &str = r#"
[package]
name = "scratch_out"
version = "0.1.0"
edition = "2021"

[dependencies]
runtime = { path = "../../runtime" }

[profile.release]
panic = "abort"
strip = "debuginfo" # true
# lto = true
# codegen-units = 1
"#;

const HEADER: &str = r#"
//! This file is @generated from a Scratch project using github.com/LukeGrahamLandry/hctarcs
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(unused_parens)]
#![allow(unused_variables)]
use runtime::sprite::{SpriteBase, Sprite, Trigger};
use runtime::{builtins, World};
use runtime::builtins::NumOrStr;
use std::borrow::Cow;
"#;


impl<'src> Emit<'src> {
    fn emit(&mut self) -> String {
        let fields: String = self.target.fields.iter().map(|&v| {
            format!("   {}: {},\n", self.project.var_names[v.0], self.inferred_type_name(v))
        }).collect();
        let procs: String = self.target.procedures.iter().map(|t| {
            let args = if t.args.is_empty() {
                "".to_string()
            } else {
                let args: Vec<_> = t.args.iter().map(|&v| {
                    self.project.var_names[v.0].clone() + ": " + self.inferred_type_name(v)
                }).collect();
                format!(", {}", args.join(", "))
            };
            format!("fn {}(&mut self, sprite: &mut SpriteBase, globals: &mut Stage{}){{\n{}}}\n\n", t.name, args, self.emit_block(&t.body))
        }).collect();
        for func in &self.target.functions {
            let body = self.emit_block(&func.body);
            let handler = match self.triggers.get(&func.start) {
                Some(prev) => prev.clone() + body.as_str(),
                None => body
            };
            self.triggers.insert(func.start.clone(), handler);
        }
        let handlers: String = self.triggers.iter().map(|(trigger, body)| {
            format!("{} => {{{body}}},\n", format_trigger(&self.project, trigger))
        }).collect();
        // TODO: wrong! defaults are in the json
        format!(r##"
#[derive(Default, Clone, Debug)]
pub struct {0} {{
{fields}}}
impl {0} {{
{procs}
}}
impl Sprite<Msg, Stage> for {0} {{
    fn receive(&mut self, sprite: &mut SpriteBase, globals: &mut Stage, msg: Trigger<Msg>) {{
        match msg {{
            {handlers}
            _ => {{}}  // Ignored.
        }}
    }}

    // Grumble grumble object safety...
    fn clone_boxed(&self) -> Box<dyn Sprite<Msg, Stage>> {{ Box::new(self.clone()) }}
}}"##, self.target.name)
    }

    // TODO: Proper indentation
    fn emit_stmt(&mut self, stmt: &'src Stmt) -> String {
        match stmt {
            Stmt::BuiltinRuntimeCall(name, args) => {
                format!("sprite.{}({});\n", name, self.emit_args(args))
            },
            Stmt::SetField(v, e) => {
                format!("self.{} = {};\n", self.project.var_names[v.0], self.emit_expr(e, self.project.expected_types[v.0].clone()))
            }
            Stmt::SetGlobal(v, e) => {
                format!("globals.{} = {};\n", self.project.var_names[v.0], self.emit_expr(e, self.project.expected_types[v.0].clone()))
            }
            Stmt::If(cond, body) => {
                format!("if {} {{\n{} }}\n", self.emit_expr(cond, Some(SType::Bool)), self.emit_block(body))
            }
            Stmt::IfElse(cond, body, body2) => {
                format!("if {} {{\n{} }} else {{\n{}}}\n", self.emit_expr(cond, Some(SType::Bool)), self.emit_block(body), self.emit_block(body2))
            }
            Stmt::RepeatTimes(times, body) => {
                format!("for _ in 0..({} as usize) {{\n{}}}\n", self.emit_expr(times, Some(SType::Number)), self.emit_block(body))
            }
            Stmt::RepeatTimesCapture(times, body, v, s) => {
                // There are no real locals so can't have name conflicts
                format!("for i in 0..({} as usize) {{\n{} = i as f64;\n{}}}\n", self.emit_expr(times, Some(SType::Number)), self.ref_var(*s, *v), self.emit_block(body))
            }
            Stmt::StopScript => "return;\n".to_string(),  // TODO: is this supposed to go all the way up the stack?
            Stmt::CallCustom(name, args) => {
                format!("self.{name}(sprite, globals, {});\n", self.emit_args(args))
            }
            Stmt::ListSet(s, v, i, item) => format!("{}[{} as usize] = NumOrStr::from({});\n", self.ref_var(*s, *v), self.emit_expr(i, Some(SType::Number)), self.emit_expr(item, None)),  // TODO: what happens on OOB?
            Stmt::ListPush(s, v, item) => format!("{}.push(NumOrStr::from({}));\n", self.ref_var(*s, *v), self.emit_expr(item, None)),  // TODO: what happens on OOB?
            Stmt::ListClear(s, v) => format!("{}.clear();\n", self.ref_var(*s, *v)),
            Stmt::BroadcastWait(name) => {
                // TODO: do the conversion at comptime when possible. it feels important enough to have the check if param is a literal
                // TODO: multiple receivers HACK
                assert!(self.target.is_singleton);
                format!("self.receive(sprite, globals, Trigger::Message(msg_of({})));\n", self.emit_expr(name, Some(SType::Str)))
            }
            _ => format!("todo!(r#\"{:?}\"#);\n", stmt)
        }
    }

    /// Comma seperated
    fn emit_args(&mut self, args: &'src [Expr]) -> String {
        // TODO: I shouldn't have to allocate the vec
        let args = args.iter().map(|e| self.emit_expr(e, None)).collect::<Vec<_>>();
        args.join(", ")
    }

    fn emit_block(&mut self, args: &'src [Stmt]) -> String {
        args.iter().map(|s| self.emit_stmt(s)).collect()
    }

    // Allocating so many tiny strings but it makes the code look so simple.
    fn emit_expr(&mut self, expr: &'src Expr, t: Option<SType>) -> String {
        match expr {
            Expr::Bin(op, rhs, lhs) => {
                // TODO: clean up `[true/false literal] == [some bool expr]`
                let infix = match op {
                    BinOp::Add => Some("+"),
                    BinOp::Sub => Some("-"),
                    BinOp::Mul => Some("*"),
                    BinOp::Div => Some("/"),
                    BinOp::GT => Some(">"),
                    BinOp::LT => Some("<"),
                    BinOp::EQ => Some("=="),
                    BinOp::And => Some("&&"),
                    BinOp::Or => Some("||"),
                    _ => None
                };
                let arg_t = match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::GT | BinOp::Random | BinOp::LT => Some(SType::Number),
                    BinOp::And | BinOp::Or => Some(SType::Bool),
                    BinOp::StrJoin => Some(SType::Str),
                    _ => None
                };
                // TODO: this is icky
                let arg_t = if *op == BinOp::EQ {
                    let rhs_t = infer_type(&self.project, rhs);
                    let lhs_t = infer_type(&self.project, lhs);
                    let goal = match (lhs_t, rhs_t) {
                        (None, None) => None, // cool beans
                        (Some(lhs_t), Some(rhs_t)) => {
                            if lhs_t == rhs_t {
                                Some(lhs_t)
                            } else {
                                match (&lhs_t, &rhs_t) {
                                    (&SType::Number, &SType::Str) | (&SType::Str, &SType::Number) => Some(SType::Number),
                                    _ => panic!("Need rule for {:?} == {:?}", lhs_t, rhs_t)
                                }
                            }
                        }
                        (None, Some(rhs_t)) => Some(rhs_t),
                        (Some(lhs_t), None) => Some(lhs_t),
                    };

                    goal
                } else {
                    arg_t
                };

                let (a, b) = (self.emit_expr(rhs, arg_t.clone()), self.emit_expr(lhs, arg_t));
                if let Some(infix) = infix {
                    return format!("({} {} {})", a, infix, b)
                }
                if *op == BinOp::Random {
                    // TODO: optimise if both are constant ints
                    return format!("builtins::dyn_rand({}, {})", a, b)
                }
                if *op == BinOp::StrJoin {
                    return format!("Cow::from({}.to_string() + {}.as_ref())", a, b)
                }
                format!("todo!(r#\"{:?}\"#)", expr)
            },
            Expr::Un(op, e) => {
                match op {
                    UnOp::Not => format!("(!{})", self.emit_expr(e, Some(SType::Bool))),
                    UnOp::SuffixCall(name) => format!("({}.{name}())", self.emit_expr(e, Some(SType::Number))),
                }
            }
            Expr::GetField(v) => self.ref_var(Scope::Instance, *v),
            Expr::GetGlobal(v) => self.ref_var(Scope::Global, *v),
            Expr::GetArgument(v) => self.project.var_names[v.0].clone(),
            Expr::Literal(s) => {
                match s.as_str() {  // TODO: proper strings and bools. HACK!
                    "true" | "false" => if Some(SType::Bool) == t {
                        s.parse::<bool>().unwrap().to_string()
                    } else {
                        s.clone()
                    },
                    "Infinity" => "f64::INFINITY".to_string(),
                    "-Infinity" => "f64::NEG_INFINITY".to_string(),
                    "" => "0f64".to_string(),
                    _ => {
                        match s.parse::<f64>() {
                            Ok(v) => format!("({}f64)", v),
                            Err(_) => format!("Cow::from(\"{}\")", s.escape_default()),
                        }

                    }  // Brackets because I'm not sure of precedence for negative literals
                }
            },
            Expr::ListLen(s, v) => format!("({}.len() as f64)", self.ref_var(*s, *v)),
            Expr::ListGet(s, v, i) => {
                // TODO: maybe strings should be in an Rc
                let prefix = match t {
                    Some(SType::Number) => "f64::from(",
                    Some(SType::Str) => "Cow::from(",
                    _ => "(",
                };
                format!("{prefix}{}[{} as usize].clone())", self.ref_var(*s, *v), self.emit_expr(i, Some(SType::Number)))
            },  // TODO: what happens on OOB?
            Expr::BuiltinRuntimeGet(name) => format!("sprite.{}()", name),
            Expr::StringGetIndex(string, index) => {
                // TODO: cows everywhere!
                // TODO: don't evaluate index twice!
                format!("Cow::from(&{0}.as_ref()[({1} as usize)..({1} as usize)+1])", self.emit_expr(string, Some(SType::Str)), self.emit_expr(index, Some(SType::Number)))
            }
            _ => format!("todo!(r#\"{:?}\"#)", expr)
        }
    }

    fn ref_var(&mut self, scope: Scope, v: VarId) -> String {
        match scope {
            Scope::Instance => format!("self.{}", self.project.var_names[v.0]),
            Scope::Global => format!("globals.{}", self.project.var_names[v.0]),
        }
    }

    fn inferred_type_name(&self, v: VarId) -> &'static str {
        match &self.project.expected_types[v.0] {
            None => "f64 /* guess */",
            Some(t) => type_name(t.clone()),
        }
    }
}

fn type_name(t: SType) -> &'static str {
    match t {
        SType::Number => "f64",
        SType::ListPolymorphic => "Vec<NumOrStr>",
        SType::Bool => "bool",
        SType::Str => "Cow<'static, str>",
    }
}
