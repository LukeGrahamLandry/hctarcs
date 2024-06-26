use std::collections::VecDeque;
use std::marker::PhantomData;
use crate::{HALF_SCREEN_HEIGHT, HALF_SCREEN_WIDTH, List, Poly, RenderBackend, RunMode, ScratchProgram, SEvent, Sprite, Str, Trigger, World};
use egui::{Button, CollapsingHeader, Color32, Context, Direction, Grid, Layout, Rgba, Separator, Ui};
use egui::plot::{Legend, Line, Plot, PlotBounds, PlotPoints, Points};
use macroquad::prelude::GREEN;
use crate::backend::RenderHandle;

#[derive(Default)]
pub struct Debugger<S: ScratchProgram<R>, R: RenderBackend<S>> {
    _p: PhantomData<(S, R)>,
    frame_time: VecDeque<f32>,
    actions: VecDeque<usize>,
    none_actions: VecDeque<usize>,
    answer: String,
}

const FRAME_COUNT: f64 = 500.0;

impl<S: ScratchProgram<R>, R: RenderBackend<S> + 'static> Debugger<S, R> {
    pub fn new() -> Self {
        Self {
            _p: Default::default(),
            frame_time: vec![0.0; FRAME_COUNT as usize].into(),
            actions: vec![0; FRAME_COUNT as usize].into(),
            none_actions: vec![0; FRAME_COUNT as usize].into(),
            answer: "".to_string(),
        }
    }

    // TODO: graphs for frame time and IoActions per frame
    pub fn frame(&mut self, egui_ctx: &Context, world: &mut World<S, R>) {
        if world.current_question.is_some() {
            egui::Window::new("Ask")
                .hscroll(true).vscroll(true)
                .default_pos((0.0, HALF_SCREEN_HEIGHT as f32))
                .show(egui_ctx, |ui| {
                    ui.label(world.current_question.as_ref().unwrap());
                    ui.add(egui::TextEdit::singleline(&mut self.answer));
                    if ui.button("Submit").clicked() {
                        assert!(world.last_answer.is_none());
                        world.last_answer = Some(self.answer.clone());
                        self.answer = String::new();
                        world.current_question = None;
                    }
                });
        }

        egui::Window::new("Variables")
            .vscroll(true).hscroll(true).default_open(false)
            .default_pos(((HALF_SCREEN_WIDTH * 2.0) as f32 + 20.0, 20.0))
            .show(egui_ctx, |ui| {
                let sprites = world.bases.iter().zip(world.custom.iter()).enumerate();
                ui.add_sized((700.0, 700.0), |ui: &mut Ui| {
                    Grid::new("vars")
                        .striped(true)
                        .min_col_width(700.0).max_col_width(700.0)
                        .show(ui, |ui| {
                            CollapsingHeader::new(format!("Globals"))
                                .show(ui, |ui| {
                                    for (i, name) in world.globals.get_var_names().into_iter().enumerate() {
                                        match world.globals.var(i) { // TODO: copy-n-paste
                                            VarBorrow::List(lst) => {
                                                CollapsingHeader::new(format!("{name} = List({len})", len=lst.len())
                                                ).show(ui, |ui| {
                                                    for (i, value) in lst.iter().enumerate() {
                                                        ui.label(format!("[{i}] {value:?}"));
                                                        ui.end_row();
                                                    }
                                                });
                                            }
                                            other => {
                                                ui.label(format!("{name} = {:?}", other));
                                            }
                                        }
                                    }
                                });
                            ui.end_row();
                            for (i, (base, user)) in sprites {
                                // println!("{i} {:?}",  user.get_var_names());
                                CollapsingHeader::new(format!("Sprite {}", base._uid))
                                    .show(ui, |ui| {
                                    if ui.button("Send Click").clicked() {
                                        world.events.push_back(SEvent::Click(i));
                                    }
                                    ui.end_row();
                                    ui.add_space(10.0);

                                    ui.label(format!("pos:({:.0}, {:.0}), dir:{:.0}, size:{:.0}% pen:({}, size:{}) costume:{}",
                                                     base.x, base.y, base.direction, base.size_frac * 100.0, base.pen.active, base.pen.size, base.costume));
                                    ui.end_row();
                                    for (i, name) in user.get_var_names().into_iter().enumerate() {
                                        match user.var(i) {
                                            VarBorrow::List(lst) => {
                                                CollapsingHeader::new(format!("{name} = List({len})", len=lst.len())
                                                ).show(ui, |ui| {
                                                    for (i, value) in lst.iter().enumerate() {
                                                        ui.label(format!("[{i}] {value:?}"));
                                                        ui.end_row();
                                                    }
                                                });
                                            }
                                            other => {
                                                ui.label(format!("{name} = {:?}", other));
                                            }
                                        }


                                        ui.end_row();
                                    }
                                });
                                ui.end_row();
                            }
                    }).response
                });
            });


        egui::Window::new("Settings")
            .hscroll(true).vscroll(true).default_open(true)
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
                    ui.radio_value(&mut world.mode, RunMode::Throttle, "Throttle");
                    ui.radio_value(&mut world.mode, RunMode::Manual(false), "Manual");

                    let btn = ui.add_enabled(world.mode == RunMode::Manual(false), Button::new("Step"));
                    if btn.clicked() {
                        world.mode = RunMode::Manual(true);
                    }
                });
                ui.end_row();

                ui.add_space(10.0);

                // TODO: completely unusable because the slider moves when you resize.
                let mut scale = egui_ctx.pixels_per_point();
                if ui.add(egui::Slider::new(&mut scale, 1.0..=4.0).text("Gui Scale")).changed() {
                    egui_ctx.set_pixels_per_point(scale);
                }
            });

        egui::Window::new("Callstack")
            .hscroll(true).vscroll(true).default_open(false)
            .default_pos(((HALF_SCREEN_WIDTH * 2.0) as f32 + 20.0, 240.0))
            .show(egui_ctx, |ui| {
                ui.add_sized((400.0, 400.0), |ui: &mut Ui| {
                    Grid::new("Futures")
                        .striped(true)
                        .show(ui, |ui| {
                            for script in world.scripts.iter() {
                                ui.add_sized((400.0, 400.0), |ui: &mut Ui| {
                                    CollapsingHeader::new(format!("Sprite {}: {:?}", script.owner, script.trigger))
                                        .show(ui, |ui|{
                                            ui.add_sized((400.0, 400.0), |ui: &mut Ui| {
                                                Grid::new("Futures inner")
                                                    .striped(true)
                                                    .max_col_width(400.0)  // TODO: how do i let the user resize but not change based on content every frame? put really wide marker thing at the end feels dumb?
                                                    .min_col_width(400.0)
                                                    .show(ui, |ui| {
                                                        for f in script.next.iter().rev() {
                                                            // TODO: use their layout for indentation
                                                            ui.label(format!("   {f:?}"));  // TODO: pretty concurrent
                                                            ui.end_row();
                                                        }
                                                    }).response
                                            });

                                        }).header_response
                                });
                            }
                        }).response
                });

            });

        egui::Window::new("Credits")
            .hscroll(true).vscroll(true).default_open(false)
            .default_pos(((HALF_SCREEN_WIDTH * 2.0) as f32 + 180.0, 20.0))
            .show(egui_ctx, |ui| {
                ui.label(S::get_credits());
            });

        let dt = egui_ctx.input(|input| input.stable_dt);
        self.frame_time.pop_front();
        self.frame_time.push_back(dt * 1000.0);

        self.actions.pop_front();
        self.actions.push_back(world.futs_this_frame);
        self.none_actions.pop_front();
        self.none_actions.push_back(world.none_futs_this_frame);

        egui::Window::new("Perf")
            .hscroll(true).vscroll(true).default_open(false)
            .default_pos(((HALF_SCREEN_WIDTH * 2.0) as f32 + 20.0, 340.0))
            .default_height(200.0)
            .show(egui_ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label("Frame Time");
                    ui.horizontal(|ui|{
                        Plot::new("frametime")
                            .auto_bounds_y().include_y(0.0).include_y(50.0)
                            .allow_drag(false).allow_zoom(false).allow_scroll(false)
                            .show(ui, |plot| {
                                let points: PlotPoints = self.frame_time
                                    .iter().enumerate()
                                    .map(|(i, v)| {
                                        [i as f64, *v as f64]
                                    }).collect();

                                plot.line(Line::new(points));
                            });
                    });

                    ui.label("Futures Resolved");

                    ui.horizontal(|ui|{
                        Plot::new("ioactions")
                            .auto_bounds_y().include_y(0.0).include_y(10.0)
                            .allow_drag(false).allow_zoom(false).allow_scroll(false)
                            .show(ui, |plot| {
                                let points: PlotPoints = self.actions
                                    .iter().enumerate()
                                    .map(|(i, v)| {
                                        [i as f64, *v as f64]
                                    }).collect();
                                let points2: PlotPoints = self.none_actions
                                    .iter().enumerate()
                                    .map(|(i, v)| {
                                        [i as f64, *v as f64]
                                    }).collect();

                                plot.line(Line::new(points).color(Color32::GREEN));
                                plot.line(Line::new(points2).color(Color32::RED));
                            });
                    });

                })

            });

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
