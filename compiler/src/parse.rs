//! Converting a structure from scratch_schema to an AST.

use std::collections::HashMap;
use crate::ast::{BinOp, Expr, Func, Proc, Project, Scope, Sprite, Stmt, SType, Trigger, UnOp, VarId};
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
        let mut proj = Project { targets: vec![], var_names: vec![], expected_types: vec![] };

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
    let mut expand = | (_, v): (&String, &Operand)| {
        let name = v.unwrap_var();
        (name.to_string(), proj.next_var(name))
    };
    let mut a: HashMap<String, VarId> = target.variables.iter().map(&mut expand).collect();
    a.extend(target.lists.iter().map(&mut expand));
    a
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
        let entry = self.target.blocks.iter().filter(|(_, v)| v.opcode.starts_with("event_when"));
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
            name: self.target.name.clone(),
            is_stage: self.target.isStage,
            is_singleton: true,
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

    fn parse_t(&mut self, e: &Operand, t: SType) -> Expr {
        let e = self.parse_op_expr(e);
        self.expect_type(&e, t);
        e
    }

    fn parse_stmt(&mut self, block: &'src Block) -> Stmt {
        match block.opcode.as_str() {
            "control_if_else" => unwrap_input!(block, Input::Branch2 { CONDITION, SUBSTACK, SUBSTACK2 } => {
                Stmt::IfElse(self.parse_t(CONDITION, SType::Bool), self.parse_body(SUBSTACK.opt_block()), self.parse_body(SUBSTACK2.opt_block()))
            }),
            "control_if" => unwrap_input!(block, Input::Branch1 { CONDITION, SUBSTACK } => {
                Stmt::If(self.parse_t(CONDITION, SType::Bool), self.parse_body(SUBSTACK.opt_block()))
            }),
            "control_repeat" => unwrap_input!(block, Input::ForLoop { TIMES, SUBSTACK } => {
                Stmt::RepeatTimes(self.parse_t(TIMES, SType::Number), self.parse_body(SUBSTACK.opt_block()))
            }),
            "control_stop" => {
                match block.fields.as_ref().unwrap().unwrap_stop() {
                    StopOp::ThisScript => Stmt::StopScript,
                    StopOp::All => Stmt::Exit
                }
            },
            "data_setvariableto" => unwrap_field!(block, Field::Var { VARIABLE } => {
                let value = self.parse_op_expr(block.inputs.as_ref().unwrap().unwrap_one());
                let val_t = self.infer_type(&value);
                let (v, scope) = self.resolve(VARIABLE);
                if let Some(val_t) = val_t {
                    self.project.expect_type(v, val_t);
                }
                println!("Set {:?} = {:?}", v, value);
                match scope {
                    Scope::Instance => Stmt::SetField(v, value),
                    Scope::Global => Stmt::SetGlobal(v, value)
                }
            }),
            "data_changevariableby" => unwrap_field!(block, Field::Var { VARIABLE } => {  // TODO: this could have a new ast node and use prettier +=
                let value = self.parse_op_expr(block.inputs.as_ref().unwrap().unwrap_one());
                self.expect_type(&value, SType::Number);
                match self.fields.get(VARIABLE.unwrap_var()) {
                    Some(&v) => {
                        self.project.expect_type(v, SType::Number);
                        Stmt::SetField(v, Expr::Bin(BinOp::Add, Box::new(Expr::GetField(v)), Box::new(value)))
                    },
                    None => {
                        let v = *self.globals.get(VARIABLE.unwrap_var()).unwrap();
                        self.project.expect_type(v, SType::Number);
                        Stmt::SetGlobal(v, Expr::Bin(BinOp::Add, Box::new(Expr::GetGlobal(v)), Box::new(value)))
                    }
                }
            }),
            "data_deletealloflist" => unwrap_field!(block, Field::List { LIST } => {
                let (v, scope) = self.resolve(LIST);
                self.project.expect_type(v, SType::ListPolymorphic);
                Stmt::ListClear(scope, v)
            }),
            "control_for_each" => unwrap_field!(block, Field::Var { VARIABLE } => {
                unwrap_input!(block, Input::SecretForLoop { SUBSTACK, VALUE } => {
                    let (v, s) = self.resolve(VARIABLE);
                    self.project.expect_type(v, SType::Number);
                    Stmt::RepeatTimesCapture(self.parse_t(VALUE, SType::Number), self.parse_body(SUBSTACK.opt_block()), v, s)
                })
            }),
            "procedures_call" => unwrap_input!(block, Input::Named(args) => {
                let proto = block.mutation.as_ref().unwrap();
                let args: Vec<_> = proto.arg_ids().iter()
                    .map(|id| args.get(id).unwrap())
                    .map(|o| self.parse_op_expr(o))
                    .collect();

                let types: Vec<_> = args.iter().map(|e| self.infer_type(e)).collect();
                // let arg_vars = pro


                Stmt::CallCustom(safe_str(proto.name()), args)
            }),
            "data_replaceitemoflist" => unwrap_field!(block, Field::List { LIST } => {
                unwrap_input!(block, Input::ListBoth { INDEX, ITEM } => {
                    let (v, scope) = self.resolve(LIST);
                    let mut i = self.parse_op_expr(INDEX);

                    if let Expr::Literal(s) = &i {
                        if s == "last" {
                            i = Expr::Bin(BinOp::Sub, Box::new(Expr::ListLen(scope, v)), Box::new(Expr::Literal("1".into())));
                        }
                    }
                    self.expect_type(&i, SType::Number);
                    let val = self.parse_op_expr(ITEM);

                    self.maybe_expect_list(v, &val);
                    Stmt::ListSet(scope, v, i, val)
                })
            }),
            "data_addtolist" => unwrap_field!(block, Field::List { LIST } => {
                unwrap_input!(block, Input::ListItem { ITEM } => {
                    let (v, scope) = self.resolve(LIST);
                    let val = self.parse_op_expr(ITEM);
                    self.maybe_expect_list(v, &val);
                    Stmt::ListPush(scope, v, val)
                })
            }),
            "event_broadcastandwait" => unwrap_input!(block, Input::Broadcast { BROADCAST_INPUT } => {
                let event = self.parse_t(BROADCAST_INPUT, SType::Str);
                Stmt::BroadcastWait(event)
            }),
            _ => if let Some(proto) = runtime_prototype(block.opcode.as_str()) {
                let args = match proto {
                    &[] => vec![],
                    &[SType::Number] => vec![self.parse_t(block.inputs.as_ref().unwrap().unwrap_one(), SType::Number)],
                    &[SType::Number, SType::Number] => {
                        let (a, b) = block.inputs.as_ref().unwrap().unwrap_pair();
                        let (a, b) = (self.parse_t(a, SType::Number), self.parse_t(b, SType::Number));
                        vec![a, b]
                    }

                    _ => vec![Expr::UnknownExpr(format!("args::{:?}", proto))],
                };
                Stmt::BuiltinRuntimeCall(block.opcode.clone(), args)
            } else {
                Stmt::UnknownOpcode(block.opcode.clone())
            }
        }
    }

    fn resolve(&mut self, name: &Operand) -> (VarId, Scope) {
        match self.fields.get(name.unwrap_var()) {
            Some(&v) => (v, Scope::Instance),
            None => {
                let v = *self.globals.get(name.unwrap_var()).unwrap();
                (v, Scope::Instance)
            }
        }
    }

    fn maybe_expect_list(&mut self, list: VarId, item: &Expr) {
        println!("expect list {:?} -> {}", item, self.project.var_names[list.0]);
        let val_t = self.infer_type(&item);
        self.project.expect_type(list, SType::ListPolymorphic);

        if let Some(val_t) = val_t {
            match val_t {
                SType::Number | SType::Str | SType::Bool  => {}, // TODO: why are bools ending up here
                SType::ListPolymorphic => panic!("Expected list item found {:?} {:?} for {}", val_t, item, self.project.var_names[list.0])
            };
        }
    }

    // TODO: could replace this with parse_t since you probably always want that
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

        let block = self.target.blocks.get(block.opt_block().unwrap()).unwrap();
        self.parse_expr(block)
    }

    fn coerce_number(e: Expr) -> Expr {
        match &e {
            Expr::Literal(s) => if s == "" {
                Expr::Literal("0.0".to_string())
            } else {
                e
            }
            _ => e
        }
    }

    fn parse_op_num(&mut self, block: &Operand) -> Expr {
        Parser::coerce_number(self.parse_op_expr(block))
    }

    fn parse_expr(&mut self, block: &Block) -> Expr {
        if let Some(op) = bin_op(&block.opcode) {  // TODO: make sure of left/right ordering
            let (lhs, rhs) = block.inputs.as_ref().unwrap().unwrap_pair();
            return Expr::Bin(op, Box::from(self.parse_op_num(lhs)), Box::from(self.parse_op_num(rhs)))
        }

        match block.opcode.as_str() {
            "operator_not" => unwrap_input!(block, Input::Un { OPERAND } => {
                Expr::Un(UnOp::Not, Box::new(self.parse_t(OPERAND, SType::Bool)))
            }),
            "operator_mathop" => unwrap_field!(block, Field::Op { OPERATOR } => {
                if let Operand::Var(name, _) = OPERATOR {
                    let e = Box::new(self.parse_op_num(block.inputs.as_ref().unwrap().unwrap_one()));  // TODO: ugh
                    self.expect_type(&e, SType::Number);

                    if name == "10 ^" {
                        return Expr::Bin(BinOp::Pow, Box::new(Expr::Literal("10".into())), e);
                    }

                    let op = match name.as_str() {  // TODO: sad allocation noises
                        "ceiling" => "ceil".to_string(),
                        "log" => "log10".to_string(), // TODO: make sure right base
                        "e ^" => "exp".to_string(),
                        "sin" | "cos" | "tan"
                            => format!("to_degrees().{}", name),
                        "asin" | "acos" | "atan"
                            => format!("{}().to_degrees", name),
                        _ => name.to_string(),  // TODO: don't assume valid input
                    };
                    let op = UnOp::SuffixCall(op);

                    Expr::Un(op, e)
                } else {
                    panic!("Expected operator_mathop[OPERATOR]==Var(..) found {:?}", OPERATOR)
                }
            }),
            "data_itemoflist" => unwrap_field!(block, Field::List { LIST } => {
                unwrap_input!(block, Input::ListIndex { INDEX } => {
                    let (v, scope) = self.resolve(LIST);
                    let mut i = self.parse_op_expr(INDEX);

                    if let Expr::Literal(s) = &i {
                        if s == "last" {
                            i = Expr::Bin(BinOp::Sub, Box::new(Expr::ListLen(scope, v)), Box::new(Expr::Literal("1".into())));
                        }
                    }
                    self.expect_type(&i, SType::Number);
                    self.project.expect_type(v, SType::ListPolymorphic);
                    Expr::ListGet(scope, v, Box::new(i))
                })
            }),
            "data_lengthoflist" => unwrap_field!(block, Field::List { LIST } => {
                let (v, scope) = self.resolve(LIST);
                self.project.expect_type(v, SType::ListPolymorphic);
                Expr::ListLen(scope, v)
            }),
            "argument_reporter_string_number" => unwrap_field!(block, Field::Val { VALUE } => {
                let v = self.args_by_name.get(VALUE.unwrap_var()).unwrap();
                Expr::GetArgument(*v)
            }),
            "operator_letter_of" => unwrap_input!(block, Input::CharStr { LETTER, STRING } => {
                let index = Box::new(self.parse_t(LETTER, SType::Number));
                let string = Box::new(self.parse_t(STRING, SType::Str));
                Expr::StringGetIndex(string, index)
            }),
            "operator_join" => unwrap_input!(block, Input::StrPair { STRING1, STRING2 } => {
                // Not expecting SType::Str for args because numbers coerce
                Expr::Bin(BinOp::StrJoin, Box::new(self.parse_op_expr(STRING1)), Box::new(self.parse_op_expr(STRING2)))
            }),
            _ => Expr::BuiltinRuntimeGet(block.opcode.clone())  // TODO: should be checked
        }
    }

    // The type checking is really to propagate the inference.
    // Input should always be valid since its generated by scratch.
    // A panic here is probably a bug.
    fn expect_type(&mut self, e: &Expr, t: SType) {
        println!("expect_type {:?} {:?}", t, e);
        match e {
            Expr::GetField(v) |
            Expr::GetGlobal(v) |
            Expr::GetArgument(v)
                => self.project.expect_type(*v, t),
            Expr::Bin(op, rhs, lhs) => {
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Random | BinOp::Pow => {
                        assert_eq!(t, SType::Number);
                        self.expect_type(rhs, SType::Number);
                        self.expect_type(lhs, SType::Number);
                    }
                    BinOp::GT | BinOp::LT => {
                        assert_eq!(t, SType::Bool);
                        self.expect_type(rhs, SType::Number);
                        self.expect_type(lhs, SType::Number);
                    }
                    BinOp::EQ => {
                        assert_eq!(t, SType::Bool);
                    },
                    BinOp::And | BinOp::Or => {
                        assert_eq!(t, SType::Bool);
                        self.expect_type(rhs, SType::Bool);
                        self.expect_type(lhs, SType::Bool);
                    }
                    BinOp::StrJoin => {
                        assert_eq!(t, SType::Str);
                        self.expect_type(rhs, SType::Str);
                        self.expect_type(lhs, SType::Str);
                    }
                }
            }
            Expr::Un(op, v) => {
                match op {
                    UnOp::Not => {
                        assert_eq!(t, SType::Bool);
                        self.expect_type(v, SType::Bool);
                    }
                    UnOp::SuffixCall(_) => {
                        assert_eq!(t, SType::Number);
                        self.expect_type(v, SType::Number);
                    }
                }
            }
            Expr::Literal(s) => {
                match s.as_str() {  // TODO: really need to parse this in one place
                    "true" | "false" => assert_eq!(t, SType::Bool),
                    "Infinity" | "-Infinity" => assert_eq!(t, SType::Number),
                    "" => assert!(t == SType::Number || t == SType::Str),
                    _ => match s.parse::<f64>() {
                        Ok(_) => assert_eq!(t, SType::Number),
                        Err(_) => assert_eq!(t, SType::Str),
                    }
                }
            }
            _ => {}
        }
    }

    fn infer_type(&mut self, e: &Expr) -> Option<SType> {
        infer_type(&self.project, e)
    }

    fn parse_trigger(&mut self, block: &Block) -> Trigger {
        match block.opcode.as_str() {
            "event_whenflagclicked" => Trigger::FlagClicked,
            "event_whenbroadcastreceived" => unwrap_field!(block, Field::Msg { BROADCAST_OPTION } => {
                let target = BROADCAST_OPTION.unwrap_var();
                Trigger::Message(target.to_string())
            }),
            _ => todo!("Unknown trigger {}", block.opcode)
        }
    }
}

fn validate(_target: &RawSprite) {
    // TODO: bring back when lists are parsed
    // assert!(!target.blocks.values().any(|v| {
    //     match &v.fields {
    //         Some(Field::Named(m)) => !m.is_empty(),
    //         _ => false
    //     }
    // }));
}

/// These correspond to function definitions in the runtime. The argument types must match!
fn runtime_prototype(opcode: &str) -> Option<&'static [SType]> {
    match opcode {
        "pen_setPenColorToColor" |
        "pen_setPenSizeTo" |
        "motion_changexby" |
        "motion_changeyby"|
        "motion_setx"|
        "motion_sety" => Some(&[SType::Number]),
        "pen_penUp" |
        "pen_penDown" => Some(&[]),
        "motion_gotoxy" => Some(&[SType::Number, SType::Number]),
        _ => None
    }
}

// TODO: Somehow ive gone down the wrong path and this sucks
fn unwrap_arg_block<'src>(target: &'src RawSprite, block: &'src Block) -> &'src Block {
    target.blocks.get(block.inputs.as_ref().unwrap().unwrap_one().opt_block().unwrap()).unwrap()
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
    // TODO: be more rigorous than just hard coding the ones ive seen
    name.replace(&['-', ' ', '.', '^', '*', '@', '=', '!', '>', '+', '-', '<', '/'], "_")
}

impl Project {
    fn next_var(&mut self, name: &str) -> VarId {
        self.var_names.push(safe_str(name));
        self.expected_types.push(None);
        VarId(self.var_names.len()-1)
    }

    fn expect_type(&mut self, v: VarId, t: SType) {
        match &self.expected_types[v.0] {
            None => {
                self.expected_types[v.0] = Some(t);
            }
            Some(prev) => if prev != &t {
                println!("WARNING: type mismatch: was {:?} but now {:?} for var {}", prev, &t, self.var_names[v.0])
            }
        }
    }
}

pub fn infer_type(project: &Project, e: &Expr) -> Option<SType> {
    match e {
        Expr::GetField(v) | Expr::GetGlobal(v) | Expr::GetArgument(v)
        => project.expected_types[v.0].clone(),
        Expr::Bin(op, ..) => {
            match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Random | BinOp::Pow => Some(SType::Number),
                BinOp::GT | BinOp::LT | BinOp::EQ | BinOp::And | BinOp::Or => Some(SType::Bool),
                BinOp::StrJoin => Some(SType::Str),
            }
        }
        Expr::Un(op, _) =>  match op {
            UnOp::Not => Some(SType::Bool),
            UnOp::SuffixCall(_) => Some(SType::Number),
        },
        Expr::Literal(s) => match s.as_str() {  // TODO: really need to parse this in one place
            "true" | "false" => Some(SType::Bool),
            "Infinity" | "-Infinity" => Some(SType::Number),
            "" => None,
            _ => match s.parse::<f64>() {
                Ok(_) => Some(SType::Number),
                Err(_) => Some(SType::Str),
            }
        },
        _ => None
    }
}
