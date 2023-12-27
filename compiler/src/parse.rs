use std::collections::HashMap;
use crate::ast::{BinOp, Expr, Func, Proc, Project, Sprite, Stmt, Trigger, VarId};
use crate::scratch_schema::{Block, Field, Input, Operand, RawSprite, ScratchProject};

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
        let mut proj = Project { targets: vec![] };

        let mut stages = value.targets.iter().filter(|t| t.isStage);
        let stage = stages.next().unwrap();
        assert!(stages.next().is_none());
        let globals = get_vars(stage);

        for target in &value.targets {
            let fields = get_vars(target);
            proj.targets.push(Parser { target, fields, globals: &globals}.parse());
        }

        proj

    }
}

fn get_vars(target: &RawSprite) -> HashMap<String, VarId> {
    target.variables.iter().enumerate().map(|(i, (k, v))| {
        (v.unwrap_var().to_string(), VarId(i))
    }).collect()
}

struct Parser<'src> {
    target: &'src RawSprite,
    fields: HashMap<String, VarId>,
    globals: &'src HashMap<String, VarId>,
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
            procedures.push(Proc {
                body: self.parse_body(block.next.as_deref()),
            });
        }
        Sprite {
            functions,
            procedures,
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
            _ => Stmt::UnknownOpcode(block.opcode.clone())
        }
    }

    fn parse_op_expr(&mut self, block: &Operand) -> Expr {
        if let Some(constant) = block.constant() {
            return Expr::Literal(constant.to_string());
        }

        if let Operand::ArgRef(..) = block {
            return Expr::UnknownExpr(format!("{:?}", block));  // TODO
        }

        let block = self.target.blocks.get(block.unwrap_block()).unwrap();
        self.parse_expr(block)
    }

    fn parse_expr(&mut self, block: &Block) -> Expr {
        if let Some(op) = bin_op(&block.opcode) {  // TODO: make sure of left/right ordering
            let (lhs, rhs) = block.inputs.as_ref().unwrap().unwrap_pair();
            return Expr::Bin(op, Box::from(self.parse_op_expr(lhs)), Box::from(self.parse_op_expr(rhs)))
        }

        match &block.opcode {
            _ => Expr::UnknownExpr(block.opcode.clone())
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
        "operator_mathop" => None,  // TODO: block.fields[OPERATOR] == function name
        _ => None
    }
}