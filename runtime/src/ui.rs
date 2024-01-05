use std::marker::PhantomData;
use crate::{Poly, RenderBackend, ScratchProgram, Str, World};
use egui_macroquad::egui;
use egui_macroquad::egui::Grid;
// TODO: dont care whose giving me egui, depend on it directly

#[derive(Default)]
pub struct Debugger<S: ScratchProgram<R>, R: RenderBackend<S>> {
    _p: PhantomData<(S, R)>,
}

impl<S: ScratchProgram<R>, R: RenderBackend<S>> Debugger<S, R> {
    pub fn new() -> Self {
        Self {
            _p: Default::default(),
        }
    }
    pub fn frame(&mut self, world: &World<S, R>) {
        egui_macroquad::ui(|egui_ctx| {
            egui::Window::new("Sprites").hscroll(true)
                .show(egui_ctx, |ui| {
                    let sprites = world.bases.iter().zip(world.custom.iter()).enumerate();
                    Grid::new("Sprites")
                        .striped(true)
                        .show(ui, |ui| {
                            for (i, (base, user)) in sprites {
                                ui.label(format!("[{}] pos:({:.0}, {:.0}), dir:{:.0}, size:{:.0}% pen:({}, size:{}) costume:{}", base._uid, base.x, base.y, base.direction, base.size_frac * 100.0, base.pen.active, base.pen.size, base.costume));
                                ui.label(format!("{:?}", user));
                                ui.end_row();
                            }
                        });
                });
        });

        egui_macroquad::draw();
    }
}

pub enum VarBorrow<'a> {
    Num(f64),
    Bool(bool),
    Str(&'a Str),
    Poly(&'a Poly),
    List(&'a [Poly]),
}

pub enum VarBorrowMut<'a> {
    Num(&'a mut f64),
    Bool(&'a mut bool),
    Str(&'a mut Str),
    Poly(&'a mut Poly),
    List(&'a mut [Poly]),
}

pub struct DebugInfo {
    functions: Vec<FuncInfo>,
    actions: Vec<ActionInfo>
}

pub struct ActionInfo {
    display: &'static str,
    id: usize,
    func_id: usize,
}

pub struct FuncInfo {
    sign: &'static str,
}
