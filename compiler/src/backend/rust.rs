use crate::ast::{BinOp, Expr, Project};

pub fn emit_rust(project: &Project) -> String {
    project.targets.iter().map(|target| Emit { project }.emit()).collect()
}

struct Emit<'src> {
    project: &'src Project
}

impl<'src> Emit<'src> {
    fn emit(&mut self) -> String {
        "".to_string()
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
                    return format!("{} {} {}", self.emit_expr(rhs), infix, self.emit_expr(lhs))
                }
                format!("todo!(r#\"{:?}\"#)", expr)
            }
            Expr::GetField(v) => format!("self.{}", self.project.var_names[v.0]),
            Expr::GetGlobal(v) => format!("globals.{}", self.project.var_names[v.0]),
            _ => format!("todo!(r#\"{:?}\"#)", expr)
        }
    }
}
