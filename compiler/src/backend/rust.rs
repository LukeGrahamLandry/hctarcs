use crate::ast::{BinOp, Expr, Project, Sprite, Stmt};

pub fn emit_rust(project: &Project) -> String {
    project.targets.iter().map(|target| Emit { project, target }.emit()).collect()
}

struct Emit<'src> {
    project: &'src Project,
    target: &'src Sprite
}

impl<'src> Emit<'src> {
    fn emit(&mut self) -> String {
        let fields: String = self.target.fields.iter().map(|v| {
            format!("   {}: f64,\n", self.project.var_names[v.0])  // TODO: types
        }).collect();
        let procs: String = self.target.procedures.iter().map(|t| {
            format!("fn {}(&mut self, sprite: &mut SpriteBase, globals: &mut Stage){{\n{}}}\n\n", t.name, self.emit_block(&t.body))
        }).collect();
        format!("pub struct {0} {{\n{fields}}}\nimpl {0} {{\n{procs}\n}}\n\n", self.target.name)
    }

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
            _ => format!("todo!(r#\"{:?}\"#);\n", stmt)
        }
    }

    /// Comma seperated
    fn emit_args(&mut self, args: &'src [Expr]) -> String {
        // TODO: I shouldn't have to allocate the vec
        let args = args.iter().map(|e| self.emit_expr(e)).collect::<Vec<_>>();
        args.join(",")
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
                    BinOp::Mod => Some("%"),
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
                format!("todo!(r#\"{:?}\"#)", expr)
            }
            Expr::GetField(v) => format!("self.{}", self.project.var_names[v.0]),
            Expr::GetGlobal(v) => format!("globals.{}", self.project.var_names[v.0]),
            Expr::Literal(s) => format!("{}f64", s),  // TODO: strings and bools
            _ => format!("todo!(r#\"{:?}\"#)", expr)
        }
    }
}
