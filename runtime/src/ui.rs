use std::marker::PhantomData;
use crate::{HALF_SCREEN_WIDTH, List, Poly, RenderBackend, RunMode, ScratchProgram, Sprite, Str, Trigger, World};
use egui::{Button, CollapsingHeader, Direction, Grid, Layout, Separator, Ui};
use crate::backend::RenderHandle;

#[derive(Default)]
pub struct Debugger<S: ScratchProgram<R>, R: RenderBackend<S>> {
    _p: PhantomData<(S, R)>,
}

impl<S: ScratchProgram<R>, R: RenderBackend<S> + 'static> Debugger<S, R> {
    pub fn new() -> Self {
        Self {
            _p: Default::default(),
        }
    }

    pub fn frame(&mut self, world: &mut World<S, R>) {
        egui_macroquad::ui(|egui_ctx| {
            egui::Window::new("Sprites").hscroll(true).default_pos(((HALF_SCREEN_WIDTH * 2.0) as f32 + 20.0, 20.0))
                .show(egui_ctx, |ui| {
                    let sprites = world.bases.iter().zip(world.custom.iter()).enumerate();
                    Grid::new("vars")
                        .striped(true)
                        .show(ui, |ui| {
                            CollapsingHeader::new(format!("Globals")
                            ).show(ui, |ui| {
                                for (i, name) in world.globals.get_var_names().into_iter().enumerate() {
                                    ui.label(format!("{name} = {:?}", world.globals.var(i)));
                                    ui.end_row();
                                }
                            });
                            ui.end_row();
                            for (i, (base, user)) in sprites {
                                CollapsingHeader::new(format!("[{}] pos:({:.0}, {:.0}), dir:{:.0}, size:{:.0}% pen:({}, size:{}) costume:{}",
                                                              base._uid, base.x, base.y, base.direction, base.size_frac * 100.0, base.pen.active, base.pen.size, base.costume)
                                ).show(ui, |ui| {
                                    for (i, name) in user.get_var_names().into_iter().enumerate() {
                                        ui.label(format!("{name} = {:?}", user.var(i)));
                                        ui.end_row();
                                    }
                                });
                                ui.end_row();
                            }
                        });
                });

            egui::Window::new("Async")
                .hscroll(true).vscroll(true)
                .default_pos(((HALF_SCREEN_WIDTH * 2.0) as f32 + 20.0, 120.0))
                .show(egui_ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Flag").clicked() {
                            // TODO: is this supposed to stop old flag events
                            world.broadcast_toplevel_async(Trigger::FlagClicked);
                        }

                        if ui.button("Stop").clicked() {
                            world.scripts.clear();
                            world.mode = RunMode::Manual(false)
                        }

                        if ui.button("Hard Reset").clicked() {
                            world.restart();
                        }
                    });

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.radio_value(&mut world.mode, RunMode::Turbo, "Turbo");
                        ui.radio_value(&mut world.mode, RunMode::Compat, "Compat");
                        ui.radio_value(&mut world.mode, RunMode::Manual(false), "Manual");

                        let btn = ui.add_enabled(world.mode == RunMode::Manual(false), Button::new("Step"));
                        if btn.clicked() {
                            world.mode = RunMode::Manual(true);
                        }
                    });
                    ui.end_row();


                    ui.add_sized((400.0, 400.0), |ui: &mut Ui| {
                        Grid::new("Futures")
                            .striped(true)
                            .max_col_width(400.0)  // TODO: how do i let the user resize but not change based on content every frame? put really wide marker thing at the end feels dumb?
                            .min_col_width(400.0)
                            .show(ui, |ui| {
                                for script in world.scripts.iter() {
                                    ui.label(format!("Script of {}", script.owner));
                                    ui.end_row();
                                    for f in script.next.iter().rev() {
                                        // TODO: use their layout for indentation
                                        ui.label(format!("   {f:?}"));  // TODO: pretty concurrent
                                        ui.end_row();
                                    }
                                }
                            }).response
                    });

                });
        });

        egui_macroquad::draw();
    }
}

#[derive(Debug)]
pub enum VarBorrow<'a> {
    Num(&'a f64),  // TODO: these could be by value since its just a word but consistency is easier rn
    Bool(&'a bool),
    Str(&'a Str),
    Poly(&'a Poly),
    List(&'a List<Poly>),
    Fail
}

#[derive(Debug)]
pub enum VarBorrowMut<'a> {
    Num(&'a mut f64),
    Bool(&'a mut bool),
    Str(&'a mut Str),
    Poly(&'a mut Poly),
    List(&'a mut List<Poly>),
    Fail
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
