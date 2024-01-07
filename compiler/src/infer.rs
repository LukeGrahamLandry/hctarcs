use crate::ast::{Expr, Project, Stmt, SType};
use crate::parse::{expect_type, infer_type};

pub fn run_infer(project: &mut Project) {
    // This feels sad and slow but it seems to not that big a difference and does improve inference coverage.
    let mut count = 0;
    let mut last = false;
    loop {
        let mut infer = Infer {
            project,
            dirty: 0,
            current_fn: None,
        };
        infer.run();
        // TODO: track dirty for types. currently just tracking for aysnc.
        if infer.dirty == 0 {
            if last {
                break;
            }
            last = true;

        } else {
            last = false;
        }
        count += 1;
        if count > 10 {
            println!("Infer round {count}");
        }
    }
}

struct Infer<'a> {
    project: &'a mut Project,
    dirty: usize,
    // None if in a script since those are always async. In proc, (sprite_index, func_index)
    current_fn: Option<(usize, usize)>
}

impl<'a> Infer<'a> {
    fn run(&mut self) {
        for i in 0..self.project.targets.len() {
            for j in 0..self.project.targets[i].procedures.len() {
                let proc = &self.project.targets[i].procedures[j];
                let block = proc.body.clone();
                self.current_fn = Some((i, j));
                self.infer_block(block);
                self.current_fn = None;
            }
        }

        for i in 0..self.project.targets.len() {
            for j in 0..self.project.targets[i].scripts.len() {
                let block = self.project.targets[i].scripts[j].body.clone();
                self.current_fn = None;
                self.infer_block(block);
            }
        }
    }

    fn mark_async(&mut self) {
        if let Some((target_i, proc_i)) = self.current_fn {
            let proc = &mut self.project.targets[target_i].procedures[proc_i];
            if !proc.needs_async {
                println!("mark_async {}", proc.name);
                proc.needs_async = true;
                self.dirty += 1;
            }
        }
    }

    fn infer_expr(&mut self, expr: Expr) {
        if let Some(t) = infer_type(self.project, &expr) {
            expect_type(self.project, &expr, t);
        }
    }

    fn infer_block(&mut self, s: Vec<Stmt>) {
        for s in s {
            self.infer_stmt(s)
        }
    }

    fn infer_stmt(&mut self, stmt: Stmt) {
        match stmt {
            Stmt::RepeatTimes(e, s) |
            Stmt::If(e, s) |
            Stmt::RepeatUntil(e, s) => {
                self.infer_expr(e);
                self.infer_block(s);
            }
            Stmt::IfElse(e, s, s1) => {
                self.infer_expr(e);
                self.infer_block(s);
                self.infer_block(s1);
            }

            Stmt::RepeatTimesCapture(e, s, v, scope) => {
                self.infer_expr(e);
                self.infer_block(s);
                self.project.expect_type(v, SType::Number);
            }
            Stmt::SetField(v, e) |
            Stmt::SetGlobal(v, e) => {
                self.infer_expr(e.clone());
                if let Some(t) = infer_type(self.project, &e) {
                    self.project.expect_type(v, t);
                }
            }
            Stmt::ListSet(_, _, _, _) => {}
            Stmt::ListPush(_, _, _) => {}
            Stmt::ListClear(_, _) => {}
            Stmt::ListRemoveIndex(_, _, _) => {}
            Stmt::BuiltinRuntimeCall(_, _) => {}
            Stmt::CallCustom(name, _) => {
                if let Some((sprite_id, _)) = self.current_fn {
                    let func = self.project.targets[sprite_id].lookup_proc(&name);
                    if func.unwrap().needs_async {
                        self.mark_async();
                    }
                }
            }
            Stmt::UnknownOpcode(_) => {}
            Stmt::CloneMyself => {}
            Stmt::WaitSeconds(e) => {
                self.infer_expr(e);
                self.mark_async();
            }
            Stmt::StopScript => {
                // This is only async if fn is already async so dont mark here.
                // TODO: if i incorrectly do this, linrays does 700k futs/frame and is really slow.
                //       use that as a test to optimise when only real ioaction is deep inner function.
                //       allow functions that optionally return an ioaction. not sure i can really get that to work for that case tho.
                // TODO: comment out when done testing
                self.mark_async();
            }
            Stmt::BroadcastWait(_) | Stmt::Exit => {
                self.mark_async();
            }
        }
    }

}

