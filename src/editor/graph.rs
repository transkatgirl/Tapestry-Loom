use eframe::egui::{Color32, Pos2, Rect, ScrollArea, Sense, Stroke, Ui};
use egui_notify::Toasts;
use flagset::FlagSet;
use rust_sugiyama::configure::{Config, CrossingMinimization, RankingType};
use tapestry_weave::ulid::Ulid;

use crate::{
    editor::shared::{
        SharedState, get_node_color,
        layout::{ArrangedWeave, WeaveLayout},
        weave::WeaveWrapper,
    },
    settings::{Settings, shortcuts::Shortcuts},
};

#[derive(Debug)]
pub struct GraphView {
    layout: WeaveLayout,
    arranged: ArrangedWeave,
}

impl Default for GraphView {
    fn default() -> Self {
        Self {
            layout: WeaveLayout::with_capacity(65535, 262144),
            arranged: ArrangedWeave::default(),
        }
    }
}

impl GraphView {
    pub fn reset(&mut self) {
        self.arranged = ArrangedWeave::default();
    }
    pub fn update(
        &mut self,
        weave: &mut WeaveWrapper,
        _settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
        _shortcuts: FlagSet<Shortcuts>,
    ) {
        if state.has_weave_layout_changed {
            self.arranged = ArrangedWeave::default();
        }
    }
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut WeaveWrapper,
        settings: &Settings,
        toasts: &mut Toasts,
        state: &mut SharedState,
        shortcuts: FlagSet<Shortcuts>,
    ) {
        if self.arranged.width == 0.0 && self.arranged.height == 0.0 {
            self.layout.load_weave(
                weave,
                weave
                    .dump_identifiers_u128()
                    .map(|id| (Ulid(id), (2.5, 2.5))),
            );
            self.arranged = self.layout.layout_weave(&Config {
                minimum_length: 1,
                vertex_spacing: 10.0,
                dummy_vertices: false,
                dummy_size: 5.0,
                ranking_type: RankingType::Up,
                c_minimization: CrossingMinimization::Barycenter,
                transpose: false,
            })
        }

        let default_color = ui.visuals().widgets.inactive.text_color();

        ScrollArea::both()
            .animated(false)
            .auto_shrink(false)
            .show(ui, |ui| {
                /*let painter = ui.painter();

                for (item, (x, y)) in self.arranged.positions.iter() {
                    let node = weave.get_node(item).unwrap();

                    if let Some((p_x, p_y)) = node
                        .from
                        .and_then(|id| self.arranged.positions.get(&Ulid(id)))
                    {
                        painter.line(
                            vec![
                                Pos2 {
                                    x: (p_x + 20.0) as f32,
                                    y: (p_y + 60.0) as f32,
                                },
                                Pos2 {
                                    x: (x + 20.0) as f32,
                                    y: (y + 60.0) as f32,
                                },
                            ],
                            Stroke {
                                width: 1.0,
                                color: default_color,
                            },
                        );
                    }

                    painter.circle(
                        Pos2 {
                            x: (x + 20.0) as f32,
                            y: (y + 60.0) as f32,
                        },
                        5.0,
                        get_node_color(node, settings).unwrap_or(default_color),
                        Stroke::NONE,
                    );
                }

                for (item, (x, y)) in self.arranged.positions.iter() {
                    let node = weave.get_node(item).unwrap();

                    painter.circle(
                        Pos2 {
                            x: (x + 20.0) as f32,
                            y: (y + 60.0) as f32,
                        },
                        5.0,
                        get_node_color(node, settings).unwrap_or(default_color),
                        Stroke::NONE,
                    );
                }*/
            });

        // ui.heading("Unimplemented");

        //println!("{:#?}", self.arranged);

        /*if shortcuts.contains(Shortcuts::FitToCursor) {
            // TODO
        }

        if shortcuts.contains(Shortcuts::FitToWeave) {
            // TODO
        }*/
    }
}
