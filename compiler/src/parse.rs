//! Converting a structure from scratch_schema to an AST.

use std::collections::HashMap;
use crate::ast::{BinOp, Expr, Func, Proc, Project, Scope, Sprite, Stmt, SType, Trigger, UnOp, VarId};
use crate::infer::run_infer;
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
        let mut proj = Project { targets: vec![], var_names: vec![], expected_types: vec![], triggers_by_name: HashMap::new(), any_async: false };

        let mut stages = value.targets.iter().filter(|t| t.isStage);
        let stage = stages.next().unwrap();
        assert!(stages.next().is_none());
        let globals_vars = get_vars(&mut proj, stage);
        let globals = globals_vars.iter().map(|(k, v, _)| (k.clone(), *v)).collect();

        // TODO: using globals_vars this way is ten billion allocations for no reason
        for target in &value.targets {
            let vars = if target.isStage {  // TODO: ehhhh idk about this
                globals_vars.clone()
            } else {
                get_vars(&mut proj, target)
            };
            let fields = vars.iter().map(|(k, v, _)| (k.clone(), *v)).collect();
            let field_defaults = vars.iter().map(|(_, k, v)| (*k, v.clone())).collect();
            let result = Parser { project: &mut proj, target, fields, field_defaults, globals: &globals, args_by_name: HashMap::new(), procedures: HashMap::new(), needs_async: false }.parse();
            proj.targets.push(result);
        }

        run_infer(&mut proj);
        proj
    }
}

fn get_vars(proj: &mut Project, target: &RawSprite) -> Vec<(String, VarId, Option<Expr>)> {
    let mut expand = | (_, v): (&String, &Operand)| {
        let name = v.unwrap_var();
        let val = v.var_default_opt();
        (name.to_string(), proj.next_var(name, true), val)
    };
    let mut a: Vec<_> = target.variables.iter().map(&mut expand).collect();
    a.extend(target.lists.iter().map(&mut expand));
    a
}

struct Parser<'src> {
    project: &'src mut Project,
    target: &'src RawSprite,
    fields: HashMap<String, VarId>,
    field_defaults: HashMap<VarId, Option<Expr>>, // TODO: merge with fields
    globals: &'src HashMap<String, VarId>,
    args_by_name: HashMap<String, VarId>,
    procedures: HashMap<String, ProcProto<'src>>,
    needs_async: bool
}

struct ProcProto<'src> {
    params: Vec<VarId>,
    args_by_name: HashMap<String, VarId>,
    block: &'src Block
}

impl<'src> Parser<'src> {
    fn parse(mut self) -> Sprite {
        // println!("Parse Sprite {}", self.target.name);
        validate(self.target);
        let mut any_async = false;

        // Need to make two passes over the procedures.
        // Declare parameter vars for type inference then emit the body.
        let procedure_defs: Vec<(&String, &Block)> = self.target.blocks.iter().filter(|(_, v)| v.opcode == "procedures_definition").collect();
        self.procedures = procedure_defs
            .iter()
            .map(|(_, block)| block)
            .map(|block| {
                assert!(matches!(block.inputs, Some(Input::Custom { .. })));
                let proto = unwrap_arg_block(self.target, block);
                assert_eq!(proto.opcode, "procedures_prototype");
                let proto = proto.mutation.as_ref().unwrap();
                // TODO: arg names are not globally unique
                let args: Vec<_> = proto.arg_names().iter().map(|n| self.project.next_var(n, true)).collect();
                //println!("Decl proc {}", proto.name());
                (proto.name().to_string(), ProcProto {
                    args_by_name: proto.arg_names().iter().zip(args.iter()).map(|(k, v)| (k.clone(), *v)).collect(),
                    params: args,
                    block,
                })
            })
            .collect();

        let mut procedures = vec![];
        let procs: Vec<_> = self.procedures.keys().cloned().collect();
        for name in &procs {
            let proc = self.procedures.get(name).unwrap();
            //println!("Parse Proc {name}");
            let args = proc.params.clone();
            self.args_by_name = proc.args_by_name.clone();
            procedures.push(Proc {
                name: safe_str(name),
                body: self.parse_body(proc.block.next.as_deref()),
                args,
                needs_async: self.needs_async,
            });
            self.project.any_async |= self.needs_async;
            any_async |= self.needs_async;
            self.needs_async = false;
            self.args_by_name.clear();
        }

        let mut functions = vec![];
        let entry = self.target.blocks.iter().filter(|(_, v)| v.opcode.starts_with("event_when"));
        for (_, block) in entry {
            //println!("Parse Func {name}");
            let start = self.parse_trigger(block);
            functions.push(Func {
                start,
                body: self.parse_body(block.next.as_deref()),
                needs_async: self.needs_async,
            });
            self.project.any_async |= self.needs_async;
            any_async |= self.needs_async;
            self.needs_async = false;
        }

        let mut fields = vec![];
        let mut field_defaults = vec![];
        for v in self.fields.values().copied() {
            fields.push(v);
            field_defaults.push(self.field_defaults.get(&v).unwrap().clone());
        }

        Sprite {
            scripts: functions,
            procedures,
            fields,
            field_defaults,
            name: self.target.name.clone(),
            is_stage: self.target.isStage,
            is_singleton: true,
            costumes: self.target.costumes.clone(),
            any_async,
        }
    }


    fn parse_body_or_empty(&mut self, arg: &'src Option<Operand>) -> Vec<Stmt> {
        match arg {
            None => vec![],
            Some(arg) => self.parse_body(arg.opt_block()),
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
            "control_if_else" => unwrap_input!(block, Input::Branch { CONDITION, SUBSTACK, SUBSTACK2 } => {
                Stmt::IfElse(self.parse_t(CONDITION, SType::Bool), self.parse_body_or_empty(SUBSTACK), self.parse_body_or_empty(SUBSTACK2))
            }),
            "control_if" => unwrap_input!(block, Input::Branch { CONDITION, SUBSTACK, SUBSTACK2 } => {
                assert!(SUBSTACK2.is_none());
                Stmt::If(self.parse_t(CONDITION, SType::Bool), self.parse_body_or_empty(SUBSTACK))
            }),
            "control_repeat_until" => unwrap_input!(block, Input::Branch { CONDITION, SUBSTACK, SUBSTACK2 } => {
                assert!(SUBSTACK2.is_none());
                Stmt::RepeatUntil(self.parse_t(CONDITION, SType::Bool), self.parse_body_or_empty(SUBSTACK))
            }),
            "control_while" => unwrap_input!(block, Input::Branch { CONDITION, SUBSTACK, SUBSTACK2 } => {
                // Secret block that turbowarp knows about?
                // TODO: make sure its just flip of until
                assert!(SUBSTACK2.is_none());
                Stmt::RepeatUntil(Expr::Un(UnOp::Not, Box::new(self.parse_t(CONDITION, SType::Bool))), self.parse_body_or_empty(SUBSTACK))
            }),
            "control_repeat" => unwrap_input!(block, Input::ForLoop { TIMES, SUBSTACK } => {
                Stmt::RepeatTimes(self.parse_t(TIMES, SType::Number), self.parse_body(SUBSTACK.opt_block()))
            }),
            "control_forever" => unwrap_input!(block, Input::Forever { SUBSTACK } => {
                let const_false = Expr::Bin(BinOp::EQ, Box::new(Expr::Literal(String::from("0"))), Box::new(Expr::Literal(String::from("1"))));
                Stmt::RepeatUntil(const_false, self.parse_body(SUBSTACK.opt_block()))
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
                } else if let Some(var_t) = &self.project.expected_types[v.0] {
                     self.expect_type(&value, var_t.clone());
                }
                //println!("Set {:?} = {:?}", v, value);
                match scope {
                    Scope::Instance => Stmt::SetField(v, value),
                    Scope::Global => Stmt::SetGlobal(v, value),
                    Scope::Argument => unreachable!(),
                }
            }),
            "data_hidelist" => Stmt::Empty, // TODO
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
                self.project.expect_type(v, SType::ListPoly);
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
                //println!("Call {}", proto.name());
                let args: Vec<_> = proto.arg_ids().iter()
                    .map(|id| args.get(id).unwrap())
                    .map(|o| self.parse_op_expr(o))
                    .collect();

                let arg_types: Vec<_> = args.iter().map(|e| self.infer_type(e)).collect();
                let param_count = self.procedures.get(proto.name()).unwrap().params.len();
                for i in 0..param_count {
                    let id = self.procedures.get(proto.name()).unwrap().params[i];
                    if let Some(t) = &arg_types[i] {
                        self.project.expect_type(id, t.clone());
                    }
                }
                // TODO: need to know if callee needs_async
                Stmt::CallCustom(safe_str(proto.name()), args)
            }),
            "data_replaceitemoflist" => unwrap_field!(block, Field::List { LIST } => {
                unwrap_input!(block, Input::ListBoth { INDEX, ITEM } => {
                    let (v, scope) = self.resolve(LIST);
                    let mut i = self.parse_op_expr(INDEX);

                    if let Expr::Literal(s) = &i {
                        if s == "last" {  // One indexed!
                            i = Expr::ListLen(scope, v);
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
            "data_deleteoflist"  => unwrap_field!(block, Field::List { LIST } => {
                unwrap_input!(block, Input::ListIndex { INDEX } => {
                    let (v, scope) = self.resolve(LIST);
                    let mut i = self.parse_op_expr(INDEX);

                    if let Expr::Literal(s) = &i {
                        if s == "last" {  // One indexed!
                            i = Expr::ListLen(scope, v);
                        }
                    }
                    self.expect_type(&i, SType::Number);
                    Stmt::ListRemoveIndex(scope, v, i)
                })
            }),
            "event_broadcastandwait" => unwrap_input!(block, Input::Broadcast { BROADCAST_INPUT } => {
                let event = self.parse_t(BROADCAST_INPUT, SType::Str);
                // TODO: impl this properly
                // self.needs_async = true;
                Stmt::BroadcastWait(event)
            }),
            "control_create_clone_of" => unwrap_input!(block, Input::Clone { CLONE_OPTION } => {
                let value = self.target.blocks.get(CLONE_OPTION.opt_block().unwrap()).unwrap();
                assert_eq!(value.opcode, "control_create_clone_of_menu");
                unwrap_field!(value, Field::Clone { CLONE_OPTION } => {
                    assert_eq!(CLONE_OPTION.unwrap_var(), "_myself_");
                    self.needs_async = true;
                    Stmt::CloneMyself
                })
            }),
            "control_wait" => unwrap_input!(block, Input::Time { DURATION } => {
                let s = self.parse_t(DURATION, SType::Number);
                Stmt::WaitSeconds(s)
            }),
            "sensing_askandwait" =>  unwrap_input!(block, Input::Ask { QUESTION } => {
                let s = self.parse_t(QUESTION, SType::Str);
                Stmt::AskAndWait(s)
            }),
            _ => if let Some(proto) = runtime_prototype(block.opcode.as_str()) {
                let args = match proto {
                    &[] => vec![],
                    [arg_t] => vec![self.parse_t(block.inputs.as_ref().unwrap().unwrap_one(), arg_t.clone())],
                    [a_t, b_t] => {
                        let (a, b) = block.inputs.as_ref().unwrap().unwrap_pair();
                        let (a, b) = (self.parse_t(a, a_t.clone()), self.parse_t(b, b_t.clone()));
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
                (v, Scope::Global)
            }
        }
    }

    fn maybe_expect_list(&mut self, list: VarId, item: &Expr) {
        //println!("expect list {:?} -> {}", item, self.project.var_names[list.0]);
        let val_t = self.infer_type(&item);
        self.project.expect_type(list, SType::ListPoly);

        if let Some(val_t) = val_t {
            match val_t {
                SType::Number | SType::Str | SType::Bool | SType::Poly => {}, // TODO: why are bools ending up here
                SType::ListPoly => panic!("Expected list item found {:?} {:?} for {}", val_t, item, self.project.var_names[list.0])
            };
        }
    }

    // TODO: could replace this with parse_t since you probably always want that
    fn parse_op_expr(&mut self, block: &Operand) -> Expr {
        if let Some(constant) = block.constant() {
            return if constant.is_empty() {
                Expr::Empty
            } else {
                Expr::Literal(constant.to_string())
            };
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
            let lhs = Box::from(self.parse_op_num(lhs));
            let rhs = Box::from(self.parse_op_num(rhs));

            let mut expr = Expr::Bin(op.clone(), lhs.clone(), rhs.clone());

            // TODO: HACK!!!!
            if op == BinOp::EQ {
                if let Some(left) = get_read_var(&lhs) {
                    if let Expr::Bin(BinOp::Add, a, b) = rhs.as_ref() {
                        if let Some(right) = get_read_var(b) {
                            if matches!(a.as_ref(), Expr::Empty) && left == right {
                                expr = Expr::IsNum(lhs);
                            }
                        }
                    }
                }
            }

            // TODO: this is a bit silly. infer_type knows the output types and expect_type knows the input types and i dont want to be redundant.
            let out_t = infer_type(self.project, &expr);
            if let Some(out_t) = out_t {
                self.expect_type(&expr, out_t)
            }
            return expr;
        }

        match block.opcode.as_str() {
            "operator_not" => unwrap_input!(block, Input::Un { OPERAND } => {
                Expr::Un(UnOp::Not, Box::new(self.parse_t(OPERAND, SType::Bool)))
            }),
            "operator_length" => unwrap_input!(block, Input::Str { STRING } => {
                Expr::Un(UnOp::StrLen, Box::new(self.parse_t(STRING, SType::Str)))
            }),
            "operator_mathop" => unwrap_field!(block, Field::Op { OPERATOR } => {
                if let Operand::Var(name, _) = OPERATOR {
                    let e = Box::new(self.parse_op_num(block.inputs.as_ref().unwrap().unwrap_one()));  // TODO: ugh
                    self.expect_type(&e, SType::Number);

                    if name == "10 ^" {
                        return Expr::Bin(BinOp::Pow, Box::new(Expr::Literal("10".into())), e);
                    }

                    // TODO: make these into unique UnOp varients, clearly its backend dependent
                    let op = match name.as_str() {  // TODO: sad allocation noises
                        "ceiling" => "ceil".to_string(),
                        "log" => "log10".to_string(), // TODO: make sure right base
                        "e ^" => "exp".to_string(),
                        "sin" | "cos" | "tan"
                            => format!("to_radians().{}", name),
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
                    // TODO: this parse_index logic is in 3 places. condense it and add support for list["random"]
                    let mut i = self.parse_op_expr(INDEX);
                    if let Expr::Literal(s) = &i {
                        if s == "last" {  // One indexed!
                            i = Expr::ListLen(scope, v);
                        }
                    }
                    self.expect_type(&i, SType::Number);
                    self.project.expect_type(v, SType::ListPoly);
                    Expr::ListGet(scope, v, Box::new(i))
                })
            }),
            "data_lengthoflist" => unwrap_field!(block, Field::List { LIST } => {
                let (v, scope) = self.resolve(LIST);
                self.project.expect_type(v, SType::ListPoly);
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
            "looks_costume" => unwrap_field!(block, Field::Costume { COSTUME } => {
                // TODO: there should be an SType::Costume so the id lookup can be constant folded
                Expr::Literal(COSTUME.unwrap_var().to_string())
            }),
            "sensing_dayssince2000" => Expr::BuiltinRuntimeGet(format!("sensing_dayssince2000")),
            _ => Expr::BuiltinRuntimeGet(block.opcode.clone())  // TODO: should be checked
        }
    }

    // The type checking is really to propagate the inference.
    // Input should always be valid since its generated by scratch.
    // A panic here is probably a bug.
    fn expect_type(&mut self, e: &Expr, t: SType) {
        expect_type(self.project, e, t);
    }

    fn infer_type(&mut self, e: &Expr) -> Option<SType> {
        infer_type(&self.project, e)
    }

    fn parse_trigger(&mut self, block: &Block) -> Trigger {
        match block.opcode.as_str() {
            "event_whenflagclicked" => Trigger::FlagClicked,
            "event_whenbroadcastreceived" => unwrap_field!(block, Field::Msg { BROADCAST_OPTION } => {
                let target = BROADCAST_OPTION.unwrap_var();
                let v = match self.project.triggers_by_name.get(target) {
                    Some(&v) => v,
                    None => {
                        let v = self.project.next_var(target, false);
                        self.project.triggers_by_name.insert(target.to_string(), v);
                        v
                    }
                };
                Trigger::Message(v)
            }),
            "event_whenthisspriteclicked" => Trigger::SpriteClicked,
            _ => todo!("Unknown trigger {}", block.opcode)
        }
    }
}

fn validate(target: &RawSprite) {
    assert!(target.blocks.values().all(|v| {
        match &v.fields {
            Some(Field::Named(m)) => m.is_empty(),
            _ => true
        }
    }));
}

/// These correspond to function definitions in the runtime. The argument types must match!
pub fn runtime_prototype(opcode: &str) -> Option<&'static [SType]> {
    match opcode {
        "pen_setPenColorToColor" | "pen_setPenSizeTo" | "motion_changexby" | "motion_changeyby" | "motion_setx"
        | "motion_sety" | "looks_setsizeto"
        => Some(&[SType::Number]),
        "pen_penUp" | "pen_stamp" | "looks_hide" | "pen_clear" | "pen_penDown"
            => Some(&[]),
        "motion_gotoxy" => Some(&[SType::Number, SType::Number]),
        "looks_switchcostumeto" | "looks_say" => Some(&[SType::Str]),
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
        "operator_mod" => Some(Mod),
        "operator_equals" => Some(EQ),
        "operator_random" => Some(Random),
        "operator_mathop" => None,  // TODO: block.fields[OPERATOR] == function name
        _ => None
    }
}

pub fn safe_str(name: &str) -> String {
    // TODO: be more rigorous than just hard coding the ones ive seen
    // TODO: BROKEN! if the special char was only difference we loose. need to mangle
    name.replace(&['-', ' ', '.', '^', '*', '@', '=', '!', '>', '+', '-', '<', '/', '?'], "_")
}

impl Project {
    // safe_str is used by message triggers
    fn next_var(&mut self, name: &str, make_safe: bool) -> VarId {
        self.var_names.push(if make_safe { safe_str(name) } else { name.to_string() });
        self.expected_types.push(None);
        VarId(self.var_names.len()-1)
    }

    /// returns "did type change?"
    pub fn expect_type(&mut self, v: VarId, t: SType) -> bool {
        match &self.expected_types[v.0] {
            None => {
                self.expected_types[v.0] = Some(t);
                true
            }
            Some(prev) => {
                let changed = !types_match(prev, &t);
                if changed {
                    // Lists are not first class values in scratch, they can't be in a Poly
                    // TODO: this will be handled differently when i track list element types
                    assert_ne!(t, SType::ListPoly);
                    assert_ne!(*prev, SType::ListPoly);

                    // TODO: what happens if someone else already inferred their type based on our old incorrect guess? maybe it just works out.
                    println!("WARNING: type mismatch: was {:?} but now {:?} for var {}", prev, &t, self.var_names[v.0]);
                    self.expected_types[v.0] = Some(SType::Poly);
                }
                changed
            }
        }
    }
}

pub fn types_match(a: &SType, b: &SType) -> bool {
    a == b
        || ((a == &SType::Str || a == &SType::Number) && b == &SType::Poly)
        || ((b == &SType::Str || b == &SType::Number) && a == &SType::Poly)
}

pub fn infer_type(project: &Project, e: &Expr) -> Option<SType> {
    match e {
        Expr::GetField(v) | Expr::GetGlobal(v) | Expr::GetArgument(v)
        => project.expected_types[v.0].clone(),
        Expr::Bin(op, ..) => {
            match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Random | BinOp::Pow | BinOp::Mod => Some(SType::Number),
                BinOp::GT | BinOp::LT | BinOp::EQ | BinOp::And | BinOp::Or => Some(SType::Bool),
                BinOp::StrJoin => Some(SType::Str),
            }
        }
        Expr::Un(op, _) =>  match op {
            UnOp::Not => Some(SType::Bool),
            UnOp::SuffixCall(_) |
            UnOp::StrLen => Some(SType::Number),
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
        Expr::StringGetIndex(_, _) => Some(SType::Str),
        Expr::BuiltinRuntimeGet(s) => {
            match s.as_ref() {
                "sensing_answer" => Some(SType::Str),
                "motion_xposition" | "motion_yposition" | "sensing_dayssince2000" => Some(SType::Number),

                _ => None,
            }
        }
        Expr::ListGet(_, _, _) => Some(SType::Poly),
        Expr::ListLen(_, _) => Some(SType::Number),
        _ => None
    }
}

// This is hard to call because the expr is often in the project
/// returns "did type change?"
pub(crate) fn expect_type(project: &mut Project, e: &Expr, t: SType) -> bool {
    //println!("expect_type {:?} {:?}", t, e);
    match e {
        Expr::GetField(v) |
        Expr::GetGlobal(v) |
        Expr::GetArgument(v)
        => project.expect_type(*v, t),
        Expr::Bin(op, rhs, lhs) => {
            match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Random | BinOp::Pow | BinOp::Mod => {
                    assert!(matches!(t, SType::Number | SType::Poly));
                    expect_type(project, rhs, SType::Number) ||
                    expect_type(project, lhs, SType::Number)
                }
                BinOp::GT | BinOp::LT => {
                    assert!(matches!(t, SType::Number | SType::Bool));
                    expect_type(project, rhs, SType::Number) ||
                    expect_type(project, lhs, SType::Number)
                }
                BinOp::EQ => {
                    assert_eq!(t, SType::Bool);
                    let a = infer_type(project, lhs);
                    let b = infer_type(project, rhs);
                    if a != b {
                        // TODO: HACK for better noticing var == bool literal in ray tracer
                        if matches!(lhs.as_ref(), &Expr::Literal(_)) {
                            expect_type(project, rhs, a.unwrap())
                        } else if matches!(rhs.as_ref(), &Expr::Literal(_)) {
                            expect_type(project, lhs, b.unwrap())
                        } else {
                            expect_type(project, lhs, SType::Poly) ||
                            expect_type(project, rhs, SType::Poly)
                        }
                    } else {
                        false
                    }
                },
                BinOp::And | BinOp::Or => {
                    assert_eq!(t, SType::Bool);
                    expect_type(project, rhs, SType::Bool) ||
                    expect_type(project, lhs, SType::Bool)
                }
                BinOp::StrJoin => {
                    assert!(matches!(t, SType::Str | SType::Poly));
                    expect_type(project, rhs, SType::Str) ||
                    expect_type(project, lhs, SType::Str)
                }
            }
        }
        Expr::Un(op, v) => {
            match op {
                UnOp::Not => {
                    assert_eq!(t, SType::Bool);
                    expect_type(project, v, SType::Bool)
                }
                UnOp::SuffixCall(_) => {
                    assert_eq!(t, SType::Number);
                    expect_type(project, v, SType::Number)
                }
                UnOp::StrLen => {
                    assert_eq!(t, SType::Number);
                    expect_type(project, v, SType::Str)
                }
            }
        }
        Expr::Literal(s) => {
            if t != SType::Poly {
                match s.as_str() {  // TODO: really need to parse this in one place
                    "true" | "false" => assert!(matches!(t, SType::Bool | SType::Str)),
                    "Infinity" | "-Infinity" => assert!(matches!(t, SType::Number)),
                    "" => assert!(matches!(t, SType::Number | SType::Str)),
                    _ => match s.parse::<f64>() {
                        Ok(_) => assert_eq!(t, SType::Number),
                        Err(_) => assert_eq!(t, SType::Str),
                    }
                }
            }
            false
        }
        Expr::ListGet(_, v, i) => {
            assert!(!matches!(t, SType::ListPoly));
            project.expect_type(*v, SType::ListPoly) ||
            expect_type(project, i, SType::Number)
        }
        Expr::ListLen(_, v) => {
            assert!(matches!(t, SType::Number | SType::Poly));
            project.expect_type(*v, SType::ListPoly)
        }
        _ => false
    }
}

fn get_read_var(expr: &Expr) -> Option<VarId> {
    match expr {
        Expr::GetField(v) | Expr::GetGlobal(v) | Expr::GetArgument(v) => Some(*v),
        _ => None
    }
}

