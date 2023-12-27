//! Converting a structure from scratch_schema to an AST.

use std::collections::HashMap;
use crate::ast::{BinOp, Expr, Func, Proc, Project, Sprite, Stmt, SType, Trigger, UnOp, VarId};
use crate::ast::Expr::UnknownExpr;
use crate::scratch_schema::{Block, Field, Input, Operand, RawSprite, ScratchProject, StopOp};

macro_rules! unwrap_input {
    ($block:ident, $pattern:pat => $body:block) => {
        match &$block.inputs {
            Some($pattern) => $body,
            _ => panic!("Wrong Input for opcode {}: {:?}", $block.opcode, $block.inputs),
        }
    };
}

macro_rules! unwrap_field {
    ($block:ident, $pattern:pat => $body:block) => {
        match &$block.fields {
            Some($pattern) => $body,
            _ => panic!("Wrong Field for opcode {}: {:?}", $block.opcode, $block.fields),
        }
    };
}

impl From<ScratchProject> for Project {
    fn from(value: ScratchProject) -> Self {
        let mut proj = Project { targets: vec![], var_names: vec![] };

        let mut stages = value.targets.iter().filter(|t| t.isStage);
        let stage = stages.next().unwrap();
        assert!(stages.next().is_none());
        let globals = get_vars(&mut proj, stage);

        for target in &value.targets {
            let fields = get_vars(&mut proj, target);
            let result = Parser { project: &mut proj, target, fields, globals: &globals, args_by_name: HashMap::new() }.parse();
            proj.targets.push(result);
        }

        proj
    }
}

fn get_vars(proj: &mut Project, target: &RawSprite) -> HashMap<String, VarId> {
    target.variables.iter().map(| (_, v)| {
        let name = v.unwrap_var();
        (name.to_string(), proj.next_var(name))
    }).collect()
}

struct Parser<'src> {
    project: &'src mut Project,
    target: &'src RawSprite,
    fields: HashMap<String, VarId>,
    globals: &'src HashMap<String, VarId>,
    args_by_name: HashMap<String, VarId>,
}

impl<'src> Parser<'src> {
    fn parse(mut self) -> Sprite {
        println!("Parse Sprite {}", self.target.name);
        validate(self.target);

        let mut functions = vec![];
        let entry = self.target.blocks.iter().filter(|(_, v)| v.opcode.starts_with("event_"));
        for (name, block) in entry {
            println!("Parse Func {name}");
            let start = self.parse_trigger(block);
            functions.push(Func {
                start,
                body: self.parse_body(block.next.as_deref()),
            });
        }
        let mut procedures = vec![];
        let defs = self.target.blocks.iter().filter(|(_, v)| v.opcode == "procedures_definition");
        for (name, block) in defs {
            println!("Parse Proc {name}");

            assert!(matches!(block.inputs, Some(Input::Custom { .. })));
            let proto = unwrap_arg_block(self.target, block);
            assert_eq!(proto.opcode, "procedures_prototype");
            let proto = proto.mutation.as_ref().unwrap();
            // TODO: arg names are not globally unique
            let args: Vec<_> = proto.arg_names().iter().map(|n| self.project.next_var(n)).collect();
            self.args_by_name = proto.arg_names().iter().zip(args.iter()).map(|(k, v)| (k.clone(), *v)).collect();
            procedures.push(Proc {
                name: safe_str(proto.name()),
                body: self.parse_body(block.next.as_deref()),
                args,
            });
            self.args_by_name.clear();
        }

        Sprite {
            functions,
            procedures,
            fields: self.fields.iter().map(|(_, v)| *v).collect(),
            name: self.target.name.clone()
        }
    }

    fn parse_body(&mut self, mut next: Option<&'src str>) -> Vec<Stmt> {
        let mut body = vec![];
        while next.is_some() {
            let val = self.target.blocks.get(next.unwrap()).unwrap();
            body.push(self.parse_stmt(val));
            next = val.next.as_deref();
        }
        body
    }

    fn parse_stmt(&mut self, block: &'src Block) -> Stmt {
        match block.opcode.as_str() {
            "control_if_else" => unwrap_input!(block, Input::Branch2 { CONDITION, SUBSTACK, SUBSTACK2 } => {
                Stmt::IfElse(self.parse_op_expr(CONDITION), self.parse_body(Some(SUBSTACK.unwrap_block())), self.parse_body(Some(SUBSTACK2.unwrap_block())))
            }),
            "control_if" => unwrap_input!(block, Input::Branch1 { CONDITION, SUBSTACK } => {
                Stmt::If(self.parse_op_expr(CONDITION), self.parse_body(Some(SUBSTACK.unwrap_block())))
            }),
            "control_repeat" => unwrap_input!(block, Input::ForLoop { TIMES, SUBSTACK } => {
                Stmt::RepeatTimes(self.parse_op_expr(TIMES), self.parse_body(Some(SUBSTACK.unwrap_block())))
            }),
            "control_stop" => {
                match block.fields.as_ref().unwrap().unwrap_stop() {
                    StopOp::ThisScript => Stmt::StopScript
                }
            },
            "data_setvariableto" => unwrap_field!(block, Field::Var { VARIABLE } => {
                let value = self.parse_op_expr(block.inputs.as_ref().unwrap().unwrap_one());
                match self.fields.get(VARIABLE.unwrap_var()) {
                    Some(v) => Stmt::SetField(*v, value),
                    None => {
                        let v = self.globals.get(VARIABLE.unwrap_var()).unwrap();
                        Stmt::SetGlobal(*v, value)
                    }
                }
            }),
            "data_changevariableby" => unwrap_field!(block, Field::Var { VARIABLE } => {  // TODO: this could have a new ast node and use prettier +=
                let value = self.parse_op_expr(block.inputs.as_ref().unwrap().unwrap_one());
                match self.fields.get(VARIABLE.unwrap_var()) {
                    Some(v) => Stmt::SetField(*v, Expr::Bin(BinOp::Add, Box::new(Expr::GetField(*v)), Box::new(value))),
                    None => {
                        let v = self.globals.get(VARIABLE.unwrap_var()).unwrap();
                        Stmt::SetGlobal(*v, Expr::Bin(BinOp::Add, Box::new(Expr::GetGlobal(*v)), Box::new(value)))
                    }
                }
            }),
            "procedures_call" => unwrap_input!(block, Input::Named(args) => {
                let proto = block.mutation.as_ref().unwrap();
                let args = proto.arg_ids().iter()
                    .map(|id| args.get(id).unwrap())
                    .map(|o| self.parse_op_expr(o))
                    .collect();

                Stmt::CallCustom(safe_str(proto.name()), args)
            }),
            _ => if let Some(proto) = runtime_prototype(block.opcode.as_str()) {
                let args = match proto {
                    &[] => vec![],
                    &[SType::Number] => vec![self.parse_op_expr(block.inputs.as_ref().unwrap().unwrap_one())],
                    _ => vec![UnknownExpr(format!("call({:?})", proto))],
                };
                Stmt::BuiltinRuntimeCall(block.opcode.clone(), args)
            } else {
                Stmt::UnknownOpcode(block.opcode.clone())
            }
        }
    }

    fn parse_op_expr(&mut self, block: &Operand) -> Expr {
        if let Some(constant) = block.constant() {
            return Expr::Literal(constant.to_string());
        }

        if let Some(v) = block.opt_var() {
            return match self.fields.get(v) {
                Some(v) => Expr::GetField(*v),
                None => {
                    match self.globals.get(v) {
                        Some(v) => Expr::GetGlobal(*v),
                        _ => panic!("Undefined variable {}", v),
                    }
                }
            }
        }

        let block = self.target.blocks.get(block.unwrap_block()).unwrap();
        self.parse_expr(block)
    }

    fn parse_expr(&mut self, block: &Block) -> Expr {
        if let Some(op) = bin_op(&block.opcode) {  // TODO: make sure of left/right ordering
            let (lhs, rhs) = block.inputs.as_ref().unwrap().unwrap_pair();
            return Expr::Bin(op, Box::from(self.parse_op_expr(lhs)), Box::from(self.parse_op_expr(rhs)))
        }

        match block.opcode.as_str() {
            "operator_mathop" => unwrap_field!(block, Field::Op { OPERATOR } => {
                if let Operand::Var(name, _) = OPERATOR {
                    let op = match name.as_str() {
                        "ceiling" => "ceil",
                        "log" => "log10", // TODO: make sure right base
                        "e ^" => "exp",
                        "10 ^" => todo!("10 ^ not suffix"),
                        _ => name,  // TODO: don't assume valid input
                    };
                    let op = UnOp::SuffixCall(op.to_string());  // TODO: sad allocation noises
                    let e = Box::new(self.parse_op_expr(block.inputs.as_ref().unwrap().unwrap_one()));  // TODO: ugh
                    Expr::Un(op, e)
                } else {
                    panic!("Expected operator_mathop[OPERATOR]==Var(..) found {:?}", OPERATOR)
                }
            }),
            "argument_reporter_string_number" => unwrap_field!(block, Field::Val { VALUE } => {
                let v = self.args_by_name.get(VALUE.unwrap_var()).unwrap();
                Expr::GetArgument(*v)
            }),
            _ => Expr::BuiltinRuntimeGet(block.opcode.clone())  // TODO: should be checked
        }
    }

    fn parse_trigger(&mut self, block: &Block) -> Trigger {
        match block.opcode.as_str() {
            "event_whenflagclicked" => Trigger::FlagClicked,
            _ => todo!("Unknown trigger {}", block.opcode)
        }
    }
}

fn validate(target: &RawSprite) {
    assert!(!target.blocks.values().any(|v| {
        match &v.fields {
            Some(Field::Named(m)) => !m.is_empty(),
            _ => false
        }
    }));
}

/// These correspond to function definitions in the runtime. The argument types must match!
fn runtime_prototype(opcode: &str) -> Option<&'static [SType]> {
    match opcode {
        "pen_setPenColorToColor" => Some(&[SType::Colour]),
        "pen_setPenSizeTo" |
        "motion_changexby" |
        "motion_changeyby"|
        "motion_setx"|
        "motion_sety" => Some(&[SType::Number]),
        "pen_penUp" |
        "pen_penDown" => Some(&[]),
        _ => None
    }
}

// TODO: Somehow ive gone down the wrong path and this sucks
fn unwrap_arg_block<'src>(target: &'src RawSprite, block: &'src Block) -> &'src Block {
    target.blocks.get(block.inputs.as_ref().unwrap().unwrap_one().unwrap_block()).unwrap()
}

fn bin_op(opcode: &str) -> Option<BinOp> {
    use BinOp::*;
    match opcode {
        "operator_add" => Some(Add),
        "operator_subtract" => Some(Sub),
        "operator_multiply" => Some(Mul),
        "operator_divide" => Some(Div),
        "operator_and" => Some(And),
        "operator_or" => Some(Or),
        "operator_gt" => Some(GT),
        "operator_lt" => Some(LT),
        "operator_equals" => Some(EQ),
        "operator_random" => Some(Random),
        "operator_mathop" => None,  // TODO: block.fields[OPERATOR] == function name
        _ => None
    }
}

pub fn safe_str(name: &str) -> String {
    name.replace(&['-', ' ', '.'], "_")
}

impl Project {
    fn next_var(&mut self, name: &str) -> VarId {
        self.var_names.push(safe_str(name));
        VarId(self.var_names.len()-1)
    }
}
