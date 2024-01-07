use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::mem;
use crate::ast::{BinOp, Expr, Proc, Project, Scope, Sprite, Stmt, SType, Trigger, UnOp, VarId};
use crate::parse::{infer_type, runtime_prototype, safe_str};
use crate::{AssetPackaging, Target};
use crate::template;

// TODO: it would be more elegant if changing render backend was just a feature flag, not a src change. same for AssetPackaging but thats harder cause it only needs to copy the files there for embed
// TODO: codegen fetch (its easy)
pub fn emit_rust(project: &Project, backend: Target, _assets: AssetPackaging) -> String {
    let msgs: HashSet<Trigger> = project.targets.
        iter()
        .flat_map(|target|
            target.scripts.iter().map(|f| f.start))
        .filter(|t| matches!(t, Trigger::Message(_)))
        .collect();
    let msg_fields: HashSet<String> = msgs.iter().map(|t| {
        let name = match t {
            Trigger::Message(name) => name,
            _ => unreachable!(),
        };
        format!("{}, \n", trigger_msg_ident(project, *name))
    }).collect();
    assert_eq!(msg_fields.len(), msgs.len(), "lost some to mangling dup names");
    let msg_fields: String = msg_fields.into_iter().collect();
    let body: String = project.targets.iter().map(|target| Emit { project, target, triggers: HashMap::new(), current_is_async: false, current: None }.emit()).collect();
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
        format!("\"{}\"=>Msg::{}, \n", project.var_names[name.0].escape_default(), trigger_msg_ident(project, *name))
    }).collect();

    // TODO: move some of costume resolution into parse and dont just pass it through ast
    // TODO: dups? names need to be unique to the spite but make sure not to include same assets twice.
    let costumes: Vec<_> = project.targets
        .iter()
        .flat_map(|t| t.costumes.clone())
        .enumerate().collect();

    for (_, c) in &costumes {
        assert!(["png", "gif"].contains(&&*c.dataFormat), "TODO: Unsupported asset format for {:?} (expected png or gif)", c);
    }

    // assert_eq!(assets, AssetPackaging::Embed);
    let costume_names: String = costumes.iter().map(|(i, c)| format!("\"{}\" => Some({i}),\n", c.name.escape_default())).collect();
    let costume_includes: String = costumes.iter().map(|(_, c)| format!("ScratchAsset::Embed(include_bytes!(\"assets/{}\")),", c.md5ext)).collect();

    let backend_str = backend.code_name();
    // TODO: allow override?
    // TODO: fix redundant template syntax
    template!(
        "../data/main_rs",
        backend_str=backend_str,
        sprites=sprites,
        costume_includes=costume_includes,
        costume_names=costume_names,
        msg_fields=msg_fields,
        msg_names=msg_names,
        body=body
    )
}

fn trigger_msg_ident(project: &Project, v: VarId) -> String {
    format!("M{}_{}", v.0, safe_str(&project.var_names[v.0]))
}


fn format_trigger(project: &Project, value: &Trigger) -> String {
    match value {
        Trigger::FlagClicked => "Trigger::FlagClicked".to_string(),
        Trigger::Message(name) => format!("Trigger::Message(Msg::{})", trigger_msg_ident(project, *name)),
    }
}

struct Emit<'src> {
    project: &'src Project,
    target: &'src Sprite,
    triggers: HashMap<Trigger, Vec<RustStmt>>,
    current_is_async: bool,
    // This is used for loop closures cloning arguments. Scripts don't have arguments so its fine.
    current: Option<&'src Proc>,
}

impl<'src> Emit<'src> {
    fn emit(&mut self) -> String {
        let mut var_names = String::new();
        let mut visit_vars = String::new();
        let mut visit_vars_mut = String::new();
        let mut fields = String::new();

        // TODO: factor out debug info stuff
        for (i, v) in self.target.fields.iter().enumerate() {
            let name = &self.project.var_names[v.0];
            fields += &format!("   {}: {},\n", name, self.inferred_type_name(*v));
            let constructor= &match self.inferred_type(*v) {
                SType::Number => format!("Num(&"),
                SType::Bool => format!("Bool(&"),
                SType::Str => format!("Str(&"),
                SType::ListPoly => format!("List(&"),
                SType::Poly => format!("Poly(&"),
            };
            var_names += &format!("\"{}\",", name.escape_default());
            visit_vars += &format!("{i} => V::{constructor}self.{name}),");   // TODO: debug_assert safe string
            visit_vars_mut += &format!("{i} => V::{constructor}mut self.{name}),");
        }

        let procs: String = self.target.procedures.iter().map(|t| self.emit_custom_proc(t)).collect();

        // For each entry point, push a RustStmt to target[Trigger]
        for func in &self.target.scripts {
            self.current_is_async = true;  // Scripts are always async.
            self.current = None;
            let body = self.emit_block(&func.body);
            // TODO: idk why im in a functional mood rn
            let handler = match self.triggers.remove(&func.start) {
                Some(mut prev) => {
                    prev.push(body);
                    prev
                },
                None => vec![body],
            };
            self.triggers.insert(func.start.clone(), handler);
        }

        let mut async_handlers = String::new();

        for (trigger, scripts) in &self.triggers {
            let script_ioactions: Vec<_> = scripts
                .iter()
                .map(|script| script.clone().to_src().coerce_closed())
                .collect();

            let action = if scripts.len() == 1 {
                format!("{}.done()", script_ioactions[0])
            } else {
                format!("IoAction::Concurrent(vec![{}]).done()", script_ioactions.join(","))
            };

            async_handlers.push_str(&format!(
                "{trigger} => {action},",
                trigger = format_trigger(&self.project, trigger),
            ));
        }

        // TODO: wrong? var defaults are in the json
        // TODO: override?
        template!(
            "../data/sprite_body",
            name=self.target.name,
            procs=procs,
            fields=fields,
            async_handlers=async_handlers,
            visit_vars=visit_vars,
            var_names=var_names,
            visit_vars_mut=visit_vars_mut
        )
    }

    fn emit_custom_proc(&mut self, t: &'src Proc) -> String {
        self.current_is_async = t.needs_async;
        self.current = Some(t);
        // println!("emit {} async={}", t.name, t.needs_async);
        let args = if t.args.is_empty() {
            "".to_string()
        } else {
            let args: Vec<_> = t.args.iter().map(|&v| {
                self.project.var_names[v.0].clone() + ": " + self.inferred_type_name(v)
            }).collect();
            format!(", {}", args.join(", "))
        };
        let body = self.emit_block(&t.body);
        match body.clone().to_sync() {
            None => {
                assert!(t.needs_async, "expected async fn {} \n{:?}", t.name, body);
                // TODO: list of reserved variable names.
                format!(r#"
                    fn {name}(&self{args}) -> IoAction<Stage, Backend> {{
                        IoAction::UserFnBody(Box::new(move |ctx, this| {{
                            let this: &mut Self = ctx.trusted_cast(this);
                            let __inner: IoAction<Stage, Backend> = {{ {src} }};
                            __inner.done()
                        }}), "{name}")
                    }}
        "#, name=t.name, src=body.to_src().coerce_open())
            },
            Some(src) => {
                assert!(!t.needs_async);
                format!("fn {}(&mut self, ctx: &mut Ctx{}){{\nlet this = self;\nnosuspend!({{ {src} }}) }}\n\n", t.name, args)
            }
        }
    }

    // Don't care about generating indentation, just run cargo fmt.
    fn emit_stmt(&mut self, stmt: &'src Stmt) -> RustStmt {
        // Implicit return a string of sync code if more complex, use explicit return from the match.
        RustStmt::sync(match stmt {
            Stmt::BuiltinRuntimeCall(name, args) => {
                let arg_types: Vec<_> = runtime_prototype(name).unwrap().iter().map(|t| Some(t.clone())).collect();
                format!("ctx.{}({});\n", name, self.emit_args(args, &arg_types))
            },
            Stmt::SetField(v, e) => {
                format!("this.{} = {};\n", self.project.var_names[v.0], self.emit_expr(e, self.project.expected_types[v.0].clone()))
            }
            Stmt::SetGlobal(v, e) => {
                format!("ctx.globals.{} = {};\n", self.project.var_names[v.0], self.emit_expr(e, self.project.expected_types[v.0].clone()))
            }
            Stmt::If(cond, body) => {
                return RustStmt::If {
                    cond: self.emit_expr(cond, Some(SType::Bool)).text,
                    if_true: Box::new(self.emit_block(body)),
                    if_false: None,
                };
            }
            Stmt::IfElse(cond, body, body2) => {
                return RustStmt::If {
                    cond: self.emit_expr(cond, Some(SType::Bool)).text,
                    if_true: Box::new(self.emit_block(body)),
                    if_false: Some(Box::new(self.emit_block(body2))),
                };
            }
            Stmt::RepeatUntil(cond, body) => { // TODO: is this supposed to be do while?
                return RustStmt::Loop {
                    init: format!(""),
                    body: Box::new(self.emit_block(body)),
                    end_cond: self.emit_expr(cond, Some(SType::Bool)),
                    inc_stmt: "".to_string(),
                    clone_args: self.clone_captured_args(),
                    after_loop: Box::new(RustStmt::Empty),
                };
            }
            Stmt::RepeatTimes(times, body) => {
                // There are no real locals so can't have name conflicts
                return RustStmt::Loop {  // TODO: check edge cases of the as usize in scratch
                    init: format!("let mut i = 0usize; let end = {} as usize;", self.emit_expr(times, Some(SType::Number))),
                    body: Box::new(self.emit_block(body)),
                    end_cond: rval(SType::Bool, "(i >= end)"),
                    inc_stmt: format!("i += 1;"),
                    clone_args: self.clone_captured_args(),
                    after_loop: Box::new(RustStmt::Empty),
                };
            }
            Stmt::RepeatTimesCapture(times, body, v, s) => {
                // There are no real locals so can't have name conflicts
                let var_ty = self.project.expected_types[v.0].clone().or(Some(SType::ListPoly)).unwrap();
                let iter_expr = rval(SType::Number, "((i + 1) as f64)".to_string()).coerce(&var_ty);

                return RustStmt::Loop {
                    init: format!("let mut i = 0usize; let end = {} as usize;", self.emit_expr(times, Some(SType::Number))),
                    body: Box::new(self.emit_block(body)),
                    end_cond: rval(SType::Bool, "(i >= end)"),
                    inc_stmt: format!("{} = {iter_expr}; i += 1;", self.ref_var(*s, *v, true)),
                    clone_args: self.clone_captured_args(),
                    after_loop: Box::new(RustStmt::Empty),
                };
            }
            Stmt::StopScript => {
                if self.current_is_async {
                    return RustStmt::IoAction("IoAction::StopCurrentScript".to_string());
                } else {
                    "return;".to_string()
                }
            }
            Stmt::CallCustom(name, args) => {
                let is_async = self.target.lookup_proc(name).unwrap().needs_async;
                let args = self.emit_args(args, &self.arg_types(name));
                if is_async {  // TODO: untested
                    return RustStmt::IoAction(format!("this.{name}({args})"));
                } else {
                    format!("nosuspend!(this.{name}(ctx, {args}));\n")
                }
            }
            Stmt::ListSet(s, v, i, item) => {
                let list = self.ref_var(*s, *v, true);
                let index = self.emit_expr(i, Some(SType::Number));
                let item = self.emit_expr(item, Some(SType::Poly));
                format!("let index = {index}; let item = {item}; {list}.replace(index, item);\n")
            },
            Stmt::ListPush(s, v, item) => format!("{}.push({});\n", self.ref_var(*s, *v, false), self.emit_expr(item, Some(SType::Poly))),
            Stmt::ListClear(s, v) => format!("{}.clear();\n", self.ref_var(*s, *v, true)),
            Stmt::ListRemoveIndex(s, v, i) =>
                format!("{}.remove({});\n", self.ref_var(*s, *v, true), self.emit_expr(i, Some(SType::Number))),  // TODO: what happens on OOB?
            Stmt::BroadcastWait(name) => {
                // TODO: do the conversion at comptime when possible. it feels important enough to have the check if param is a literal
                // TODO: happy path if self.target.is_singleton
                return RustStmt::IoAction(format!("IoAction::BroadcastWait(msg_of({}))", self.emit_expr(name, Some(SType::Str))));
            }
            Stmt::Exit => return RustStmt::IoAction(String::from("IoAction::StopAllScripts")),
            Stmt::WaitSeconds(seconds) => {
                return RustStmt::IoAction(format!("IoAction::SleepSecs({})", self.emit_expr(seconds, Some(SType::Number))))
            }
            _ => format!("todo!(r#\"{:?}\"#);\n", stmt)
        })
    }

    // Ugly solution to async loops being FnMut
    fn clone_captured_args(&self) -> String {
        match self.current {
            None => String::new(),  // Scripts don't have arguments.
            Some(proc) => {
                if !self.current_is_async {
                    return "unreachable!(\"sync proc so no loop clone args for capture\")".to_string();
                }
                proc.args.iter().map(|v| {
                    let ty = self.project.expected_types[v.0].unwrap_or(SType::Poly);
                    let name = &self.project.var_names[v.0];
                    match ty {
                        SType::Number | SType::Bool => format!(""),
                        SType::Str | SType::Poly => format!("let {name} = {name}.clone();\n"),
                        SType::ListPoly => unreachable!()
                    }
                }).collect()
            }
        }
    }

    fn arg_types(&self, proc_name: &str) -> Vec<Option<SType>> {
        // If I was being super fancy, we know they're sequential so could return a slice of the original vec with 'src.
        self.target.procedures.iter().find(|p| p.name == proc_name).unwrap()
            .args.iter().map(|v| self.project.expected_types[v.0].clone()).collect()
    }

    /// Comma seperated
    fn emit_args(&mut self, args: &'src [Expr], arg_types: &[Option<SType>]) -> String {
        // TODO: I shouldn't have to allocate the vec
        let args = args.iter().zip(arg_types.iter()).map(|(e, t)| self.emit_expr(e, t.clone()).text).collect::<Vec<_>>();
        args.join(", ")
    }

    fn emit_block(&mut self, args: &'src [Stmt]) -> RustStmt {
        let mut block = RustStmt::Empty;
        for s in args {
            let s = self.emit_stmt(s);
            if !self.current_is_async {
                assert!(s.is_sync(), "Cannot await in sync context\n {:?}", s);
            }
            block.push(s);
        }
        if !self.current_is_async {
            assert!(block.is_sync(), "failed to merge sync blocks\n {:?}", block)
        }
        block
    }

    // Allocating so many tiny strings but it makes the code look so simple.
    fn emit_expr(&mut self, expr: &'src Expr, t: Option<SType>) -> RustValue {
        let t = t.or(Some(SType::Poly));
        // TODO: this match could create a RustValue that then gets coerced again at the end.
        //       and each branch still has the hint in case it needs to do something with it (empty string)(
        let value = match expr {
            Expr::Bin(op, rhs, lhs) => {
                // TODO: clean up `[true/false literal] == [some bool expr]`
                let arg_t = match op {
                    BinOp::Pow |BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::GT | BinOp::Random | BinOp::LT | BinOp::Mod => Some(SType::Number),
                    BinOp::And | BinOp::Or => Some(SType::Bool),
                    BinOp::StrJoin => Some(SType::Str),
                    BinOp::EQ => None,
                };
                let out_t = match op {
                    BinOp::Pow | BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Random | BinOp::Mod => SType::Number,
                    BinOp::EQ | BinOp::GT | BinOp::LT | BinOp::And | BinOp::Or => SType::Bool,
                    BinOp::StrJoin => SType::Str,
                };

                // TODO: this is kinda icky
                let arg_t = if *op == BinOp::EQ {
                    let rhs_t = infer_type(&self.project, rhs);
                    let lhs_t = infer_type(&self.project, lhs);
                    let res = rhs_t.or(lhs_t).unwrap_or(SType::Poly);
                    match (lhs_t, rhs_t) {
                        (Some(lhs_t), Some(rhs_t)) => {
                            if lhs_t == rhs_t { lhs_t } else { SType::Poly}
                        }
                        _ => res
                    }
                } else {
                    arg_t.unwrap()
                };

                let (a, b) = (self.emit_expr(rhs, Some(arg_t)), self.emit_expr(lhs, Some(arg_t)));

                let text = match op {
                    BinOp::Random => format!("dyn_rand({}, {})", a, b),
                    BinOp::Pow => format!("{}.powf({})", a, b),
                    BinOp::StrJoin => format!("({}.join({}))", a, b),
                    _ => {
                        let infix = match op {
                            BinOp::Add => "+",
                            BinOp::Sub => "-",
                            BinOp::Mul => "*",
                            BinOp::Div => "/",
                            // TODO: do scratch and rust agree on mod edge cases (floats and negatives)?
                            BinOp::Mod => "%",
                            BinOp::GT => ">",
                            BinOp::LT => "<",
                            BinOp::EQ => "==",
                            BinOp::And => "&&",
                            BinOp::Or => "||",
                            _ => unreachable!()
                        };
                        format!("({} {} {})", a, infix, b)
                    }
                };

                rval(out_t, text)
            },
            Expr::Un(op, e) => {
                let (found, value) = match op {
                    UnOp::Not => (SType::Bool, format!("(!{})", self.emit_expr(e, Some(SType::Bool)))),
                    UnOp::SuffixCall(name) => (SType::Number, format!("({}.{name}())", self.emit_expr(e, Some(SType::Number)))),
                    UnOp::StrLen => (SType::Number, format!("{}.len()", self.emit_expr(e, Some(SType::Str)))),
                };
                rval(found, value)
            }
            Expr::GetField(v) => self.emit_var(Scope::Instance, *v),
            Expr::GetGlobal(v) => self.emit_var(Scope::Global, *v),
            Expr::GetArgument(v) => self.emit_var(Scope::Argument, *v),
            Expr::IsNum(e) => {
                rval(SType::Bool, format!("{}.is_num()", self.emit_expr(e, Some(SType::Poly))))
            }
            Expr::Literal(s) => {
                let (value, found) = match s.as_str() {
                    "true" | "false" => (s.parse::<bool>().unwrap().to_string(), SType::Bool),
                    "Infinity" => ("f64::INFINITY".to_string(), SType::Number),
                    "-Infinity" => ("f64::NEG_INFINITY".to_string(), SType::Number),
                    "" => unreachable!(),
                    _ => {
                        match s.parse::<f64>() {
                            Ok(v) => (format!("({}f64)", v), SType::Number),
                            Err(_) => (format!("Str::from(\"{}\")", s.escape_default()), SType::Str),
                        }

                    }  // Brackets because I'm not sure of precedence for negative literals
                };
                rval(found, value)
            },
            Expr::ListLen(s, v) => {
                let e = format!("{}.len()", self.ref_var(*s, *v, true));
                rval(SType::Number, e)
            },
            Expr::ListGet(s, v, i) => {
                let value = format!("{}[{}]", self.ref_var(*s, *v, true), self.emit_expr(i, Some(SType::Number)));
                rval(SType::Poly, value)
            },
            Expr::BuiltinRuntimeGet(name) => {
                let found = infer_type(self.project, expr).unwrap_or_else(|| panic!("Failed to infer return type of BuiltinRuntimeGet {name}"));
                rval(found, format!("ctx.{}()", name))
            },
            Expr::StringGetIndex(string, index) => {
                let value = format!("{}.get_index({})", self.emit_expr(string, Some(SType::Str)), self.emit_expr(index, Some(SType::Number)));
                rval(SType::Str, value)
            }
            Expr::Empty => rval(t.unwrap(), match t {
                None | Some(SType::Poly) => "Poly::Empty",
                Some(SType::Number) => "0.0f64",
                Some(SType::Str) => "Str::from(\"\")",
                Some(SType::Bool) => "false",
                Some(SType::ListPoly) => unreachable!("Null list."),
            }.to_string()),
            _ => rval(t.unwrap(), format!("todo!(r#\"{:?}\"#)", expr))
        };
        value.coerce_m(&t)
    }

    fn ref_var(&mut self, scope: Scope, v: VarId, place_expr: bool) -> String {
        let value = match scope {
            Scope::Instance => format!("this.{}", self.project.var_names[v.0]),
            Scope::Global => format!("ctx.globals.{}", self.project.var_names[v.0]),
            Scope::Argument => format!("{}", self.project.var_names[v.0]),
        };
        if place_expr {
            value
        } else {
            match &self.project.expected_types[v.0] {
                Some(SType::Str) | Some(SType::Poly) => format!("{value}.clone()"),
                _ => value
            }
        }
    }

    fn emit_var(&mut self, scope: Scope, v: VarId) -> RustValue {
        let ty = self.project.expected_types[v.0].unwrap_or(SType::Poly);
        rval(ty, self.ref_var(scope, v, false))
    }

    fn inferred_type_name(&self, v: VarId) -> &'static str {
        match &self.project.expected_types[v.0] {
            None => "Poly /* guess */",
            Some(t) => type_name(t.clone()),
        }
    }
    fn inferred_type(&self, v: VarId) -> SType {
        match &self.project.expected_types[v.0] {
            None => SType::Poly,
            Some(t) => *t,
        }
    }

}

const CALL_ACTION: &str = "IoAction::Loop(Box::new(move |ctx, this| { let this: &mut Self = ctx.trusted_cast(this);\n";
const CALL_ACTION_ONCE: &str = "IoAction::CallOnce(Box::new(move |ctx, this| { let this: &mut Self = ctx.trusted_cast(this);\n";

// TODO: this could also track borrow vs owned
#[derive(Clone, Debug)]
struct RustValue {
    ty: SType,
    text: String,
}

#[derive(Clone, Debug)]
enum RustStmt {
    Empty,
    Sync(String),
    Block(Vec<RustStmt>),
    IoAction(String),
    Loop {
        init: String,
        body: Box<RustStmt>,
        end_cond: RustValue,
        inc_stmt: String,
        /// You can't move out of an FnMut closure (which is needed because the loop will be called multiple times)
        /// This is not used if the loop is emitted as sync code.
        clone_args: String,
        after_loop: Box<RustStmt>
    },
    If {
        cond: String,
        if_true: Box<RustStmt>,
        if_false: Option<Box<RustStmt>>
    },
    // TODO: have an AsyncSeq?
}

impl RustValue {
    fn coerce_m(self: RustValue, want: &Option<SType>) -> RustValue {
        let want = want.as_ref().unwrap_or(&SType::Poly);
        self.coerce(want)
    }

    fn coerce(self, want: &SType) -> RustValue {
        if want == &self.ty {
            return if want == &SType::Poly {
                // TODO: rethink stuff to avoid redundant clones (im sure rustc would fix but looks ugly).
                //       but i dont want to actually change behaviour based on hackily analyzing the generated string.
                //       problem is we dont distinguish between direct var reads and newly computed things
                // assert!(!value.ends_with(".clone()"));
                rval(*want, format!("{}.clone()", self.text))
            } else {
                self
            }
        }
        if want == &SType::Poly {
            assert!(!self.text.starts_with("Poly::from"));
            return rval(*want, match &self.ty {
                &SType::Number | &SType::Bool => format!("Poly::from({})", self.text),
                &SType::Str => format!("Poly::from({}.clone())", self.text),
                _ => return self,
            });
        } else if self.ty == SType::Poly {
            assert!(!self.text.ends_with(".as_num()"));
            assert!(!self.text.ends_with(".as_str()"));
            assert!(!self.text.ends_with(".as_bool()"));
            return rval(*want, match want {
                &SType::Number => format!("{}.as_num()", self.text),
                &SType::Str => format!("{}.as_str()", self.text),
                &SType::Bool => format!("{}.as_bool()", self.text),
                _ => return self,
            })
        } else if want == &SType::Str && &self.ty == &SType::Number {
            // TODO: this is only valid in string concat, otherwise probably an inference bug?
            return rval(*want, format!("Poly::from({}).as_str()", self.text))
        } else if want == &SType::Number && &self.ty == &SType::Str {
            panic!("Poly::from({self:?}).as_num()");
        } else {
            panic!("coerce want {:?} but found {self:?}", want);
        }
    }
}

fn rval(ty: SType, text: impl ToString) -> RustValue {
    RustValue { ty, text: text.to_string() }
}

impl Display for RustValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

impl RustStmt {
    fn sync(src: String) -> Self {
        RustStmt::Sync(src)
    }

    fn push(&mut self, mut other: Self) {
        if !matches!(self, RustStmt::Sync(_) | RustStmt::Empty) && self.is_sync() {
            let mut this = RustStmt::Empty;
            mem::swap(&mut this, self);
            *self = RustStmt::Sync(this.to_sync().unwrap());
        }
        match self {
            RustStmt::Sync(s) => {
                if other.is_sync() {
                    *s += &other.to_sync().unwrap();
                    return;
                }
                match &mut other {
                    RustStmt::Sync(_) | RustStmt::Empty => unreachable!(),
                    RustStmt::Loop { init, .. } => {
                        *s += init;
                        mem::swap(s, init);
                        mem::swap(self, &mut other);
                        return;
                    }
                    _ => {}
                }
            }
            RustStmt::Block(body) => {
                match other {
                    RustStmt::Block(others) => body.extend(others.into_iter()),
                    s => body.push(s),
                }
                return;
            }
            RustStmt::IoAction(_) => {}
            RustStmt::Loop { after_loop, .. } => {
                after_loop.push(other);
                return;
            }
            RustStmt::If { .. } => {}
            RustStmt::Empty => {
                *self = other;
                return;
            }
        }

        let mut this = RustStmt::Empty;
        mem::swap(&mut this, self);
        *self = RustStmt::Block(vec![this, other])
        // change strategy depending if a box would need to allocate
        // still need to handle opaque io actions like sleep. i think sequence is always better than append.
    }

    // TODO: !! check version that doesnt require clone. i call this so often.
    //       OR result that returns the original if not?
    /// None if self contains any await points.
    fn to_sync(self) -> Option<String> {
        let sync_block = |stmts: Vec<RustStmt>|
            if stmts.is_empty() { Some(String::from("")) } else {
                stmts.into_iter().map(Self::to_sync).reduce(
                    |a, b| match (a, b) {
                        (Some(mut a), Some(b)) => {
                            a.push_str(&b);
                            Some(a)
                        },
                        _ => None,
                    }).flatten()
            };

        match self {
            RustStmt::Sync(s) => Some(s),
            RustStmt::Block(stmts) => sync_block(stmts),
            RustStmt::IoAction(_) => None,
            RustStmt::Loop { init, body, end_cond, inc_stmt, after_loop, .. } => {
                // TODO: test that fails if you forget after_loop here
                match (body.to_sync(), after_loop.to_sync()) {
                    (Some(body), Some(after)) => Some(format!("{init}\n while !({end_cond}) {{ {inc_stmt} {body} }} {after}")),
                    _ => None
                }
            }
            RustStmt::If { cond, if_true, if_false } => {
                // TODO: copy-n-paste
                if let (Some(if_true), Some(if_false)) = (if_true.clone().to_sync(), if_false.clone().unwrap_or_else(|| Box::new(RustStmt::Sync(String::new()))).to_sync()) {
                    Some(format!("if {cond} {{\n {if_true} \n}} else {{\n {if_false} \n}}"))
                } else {
                    None
                }
            }
            RustStmt::Empty => Some(format!("")),
        }
    }

    fn is_sync(&self) -> bool {
        let sync_block = |stmts: &Vec<RustStmt>| stmts.is_empty() || stmts.iter().map(Self::is_sync).all(|b| b);

        match self {
            RustStmt::Sync(_) => true,
            RustStmt::Block(stmts) => sync_block(stmts),
            RustStmt::IoAction(_) => false,
            RustStmt::Loop { body, after_loop, .. } => body.is_sync() && after_loop.is_sync(),
            RustStmt::If { if_true, if_false, .. } => {
                if_true.is_sync() && (if_false.is_none() || if_false.as_ref().unwrap().is_sync())
            }
            RustStmt::Empty => true,
        }
    }

    fn to_src(self) -> RsAct {
        if matches!(self, RustStmt::Empty) {
            return RsAct::Empty;
        }
        if self.is_sync() {
            return RsAct::Sync(self.to_sync().unwrap());
        }

        match self {
            RustStmt::Sync(_) => unreachable!(),
            RustStmt::Block(body) => {  // TODO: fold them into eachotehr when possible
                let new_stmts = collapse_sync_runs(body);
                let actions: Vec<_> = new_stmts.into_iter().map(|s| s.to_src().coerce_closed()).collect();
                RsAct::Closed(format!("IoAction::Sequential(vec![{}])", actions.join(",\n")))
            },
            RustStmt::IoAction(a) => RsAct::Closed(a),
            RustStmt::Loop { init, body, end_cond, inc_stmt, clone_args, after_loop } => {
                let body_src = body.to_src().coerce_open();
                RsAct::Open(format!(r#"
                    {init}
                    {CALL_ACTION}
                    {clone_args}
                    if {end_cond} {{
                        LoopRes::Break({{ {after} }})
                    }} else {{
                        {inc_stmt}
                        let _next = {{ {body_src} }};
                        LoopRes::Continue(_next)
                    }}
                }}))"#, after=after_loop.to_src().coerce_open()))
            },
            RustStmt::If { cond, if_true, if_false } => {
                let t = if_true.to_src().coerce_open();
                let f = if_false.map(|a| a.to_src().coerce_open()).unwrap_or_else(|| String::from("IoAction::None"));
                let src = format!(r#"(if {cond} {{ {t} }} else {{ {f} }})"#);
                RsAct::Open(src)
            }
            RustStmt::Empty => unreachable!()
        }
    }
}


// TODO: remove the old version of this
fn collapse_sync_runs(body: Vec<RustStmt>) -> Vec<RustStmt> {
    let mut new_stmt = RustStmt::Empty;
    for stmt in body {
        new_stmt.push(stmt);
    }
    match new_stmt {
        RustStmt::Block(contents) => contents,
        _ => vec![new_stmt],
    }
}

enum RsAct {
    /// A block that returns nothing.
    Sync(String),
    /// A block that returns an IoAction. There may be sync code at the front that has side effects before producing the action.
    Open(String),
    /// An expression that returns an IoAction. Evaluating the expression has no side effects.
    Closed(String),
    /// Do nothing.
    Empty
}

impl RsAct {
    fn coerce_open(self) -> String {
        match self {
            RsAct::Sync(s) => format!("nosuspend!({{ {s} }});\nIoAction::None"),
            RsAct::Open(s) => s,
            RsAct::Closed(s) => s,
            RsAct::Empty => String::from("IoAction::None")
        }
    }

    /// Useful for function bodies where you want a list of futures but not early eval those later in the list.
    fn coerce_closed(self) -> String {
        match self {
            RsAct::Sync(s) => format!("({CALL_ACTION_ONCE}nosuspend!({{ {s} }});\nIoAction::None.done() }})))"),
            RsAct::Open(s) => format!("({CALL_ACTION_ONCE}{s}.done() }})))"),
            RsAct::Closed(s) => s,
            RsAct::Empty => String::from("IoAction::None")
        }
    }
}

fn collapse_if_sync(stmts: Vec<RustStmt>) -> Vec<RustStmt> {
    match RustStmt::Block(stmts.clone()).to_sync() {  // TODO: separate is_sync so dont have to clone
        None => stmts,  // TODO: can do better, collapse any runs of sync stmts
        Some(s) => vec![RustStmt::Sync(s)],
    }
}

#[derive(Debug)]
struct IoAction(String, usize);


impl Display for IoAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn type_name(t: SType) -> &'static str {
    match t {
        SType::Number => "f64",
        SType::ListPoly => "List<Poly>",
        SType::Bool => "bool",
        SType::Str => "Str",
        SType::Poly => "Poly"
    }
}
