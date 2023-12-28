use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use crate::ast::{BinOp, Expr, Project, Sprite, Stmt, Trigger, UnOp};

pub fn emit_rust(project: &Project) -> String {
    let msgs: HashSet<Trigger> = project.targets.
        iter()
        .map(|target|
            target.functions.iter().map(|f| f.start.clone())
        )
        .flatten()
        .collect();
    let msg_fields: String = msgs.iter().map(|t| format!("{}{}, \n", if *t == Trigger::FlagClicked {"#[default]"} else {""}, t.to_string())).collect();
    let body: String = project.targets.iter().map(|target| Emit { project, target, triggers: HashMap::new() }.emit()).collect();
    let sprites: String = project.targets
        .iter()
        .filter(|target| !target.is_stage)  // TODO: wrong cause stage can have scripts but im using it as special magic globals so need to rethink.
        .map(|target| format!("Box::new({}::default()), ", target.name))
        .collect();

    format!(r#"
{HEADER}
fn main() {{
    World::run_program(Stage::default(), vec![{sprites}])
}}
#[derive(Copy, Clone, Eq, PartialEq, Hash, Default)]
enum Msg {{
    {msg_fields}
}}
    {body}"#)
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
//! This file is @generated from a Scratch project by github.com/LukeGrahamLandry/hctarcs
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(unused_parens)]
#![allow(unused_variables)]
use runtime::sprite::{SpriteBase, Sprite};
use runtime::{builtins, World};
"#;

impl<'src> Emit<'src> {
    fn emit(&mut self) -> String {
        let fields: String = self.target.fields.iter().map(|v| {
            format!("   {}: f64,\n", self.project.var_names[v.0])  // TODO: types
        }).collect();
        let procs: String = self.target.procedures.iter().map(|t| {
            let args = if t.args.is_empty() {
                "".to_string()
            } else {
                let args: Vec<_> = t.args.iter().map(|v| self.project.var_names[v.0].clone() + ": f64").collect();
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
        let handlers: String = self.triggers.iter().map(|(trigger, body)| format!("Msg::{trigger} => {{{body}}},\n")).collect();
        // TODO: wrong! defaults are in the json
        format!(r##"
#[derive(Default, Clone)]
pub struct {0} {{
{fields}}}
impl {0} {{
{procs}
}}
impl Sprite<Msg, Stage> for {0} {{
    fn receive(&mut self, sprite: &mut SpriteBase, globals: &mut Stage, msg: Msg) {{
        match msg {{
            {handlers}
            _ => {{}}  // Ignored.
        }}
    }}
}}
        "##, self.target.name)
    }

    // TODO: Proper indentation
    fn emit_stmt(&mut self, stmt: &'src Stmt) -> String {
        match stmt {
            Stmt::BuiltinRuntimeCall(name, args) => {
                format!("sprite.{}({});\n", name, self.emit_args(args))
            },
            Stmt::SetField(v, e) => {
                format!("self.{} = {};\n", self.project.var_names[v.0], self.emit_expr(e))
            }
            Stmt::SetGlobal(v, e) => {
                format!("globals.{} = {};\n", self.project.var_names[v.0], self.emit_expr(e))
            }
            Stmt::If(cond, body) => {
                format!("if {} {{\n{} }}\n", self.emit_expr(cond), self.emit_block(body))
            }
            Stmt::IfElse(cond, body, body2) => {
                format!("if {} {{\n{} }} else {{\n{}}}\n", self.emit_expr(cond), self.emit_block(body), self.emit_block(body2))
            }
            Stmt::RepeatTimes(times, body) => {
                format!("for _ in 0..({} as usize) {{\n{}}}\n", self.emit_expr(times), self.emit_block(body))
            }
            Stmt::StopScript => "return;\n".to_string(),  // TODO: is this supposed to go all the way up the stack?
            Stmt::CallCustom(name, args) => {
                format!("self.{name}(sprite, globals, {});\n", self.emit_args(args))
            }
            _ => format!("todo!(r#\"{:?}\"#);\n", stmt)
        }
    }

    /// Comma seperated
    fn emit_args(&mut self, args: &'src [Expr]) -> String {
        // TODO: I shouldn't have to allocate the vec
        let args = args.iter().map(|e| self.emit_expr(e)).collect::<Vec<_>>();
        args.join(", ")
    }

    fn emit_block(&mut self, args: &'src [Stmt]) -> String {
        args.iter().map(|s| self.emit_stmt(s)).collect()
    }

    // Allocating so many tiny strings but it makes the code look so simple.
    fn emit_expr(&mut self, expr: &'src Expr) -> String {
        match expr {
            Expr::Bin(op, rhs, lhs) => {
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
                if let Some(infix) = infix {
                    return format!("({} {} {})", self.emit_expr(rhs), infix, self.emit_expr(lhs))
                }
                if *op == BinOp::Random {
                    // TODO: optimise if both are constant ints
                    return format!("builtins::dyn_rand({}, {})", self.emit_expr(rhs), self.emit_expr(lhs))
                }
                format!("todo!(r#\"{:?}\"#)", expr)
            },
            Expr::Un(op, e) => {
                match op {
                    UnOp::Not => format!("(!{})", self.emit_expr(e)),
                    UnOp::SuffixCall(name) => format!("({}.{name}())", self.emit_expr(e)),
                }
            }
            Expr::GetField(v) => format!("self.{}", self.project.var_names[v.0]),
            Expr::GetGlobal(v) => format!("globals.{}", self.project.var_names[v.0]),
            Expr::GetArgument(v) => self.project.var_names[v.0].clone(),
            Expr::Literal(s) => {
                match s.as_str() {  // TODO: proper strings and bools. HACK!
                    "true" => "1f64".to_string(),
                    "false" => "0f64".to_string(),
                    "Infinity" => "f64::INFINITY".to_string(),
                    "-Infinity" => "f64::NEG_INFINITY".to_string(),
                    "" => "0f64".to_string(),
                    _ => {
                        match s.parse::<f64>() {
                            Ok(v) => format!("({}f64)", v),
                            Err(_) => format!("\"{}\"", s.escape_default()),
                        }

                    }  // Brackets because I'm not sure of precedence for negative literals
                }
            },
            Expr::BuiltinRuntimeGet(name) => format!("sprite.{}()", name),
            _ => format!("todo!(r#\"{:?}\"#)", expr)
        }
    }
}
