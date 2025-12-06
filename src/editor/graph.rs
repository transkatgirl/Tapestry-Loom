use eframe::egui::{Color32, Pos2, Rect, ScrollArea, Sense, Stroke, Ui, Vec2};
use egui_notify::Toasts;
use egui_plot::{Line, Plot, PlotPoint, PlotPoints, Polygon};
use flagset::FlagSet;
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
            self.arranged = self.layout.layout_weave(10.0);
        }

        let default_color = ui.visuals().widgets.inactive.text_color();

        let stroke = Stroke {
            width: ui.visuals().widgets.inactive.fg_stroke.width,
            color: ui.visuals().widgets.inactive.bg_fill,
        };

        let response = Plot::new([state.identifier.to_string(), "graph".to_string()])
            .show_x(false)
            .show_y(false)
            .invert_y(true)
            .show_background(false)
            .show_axes(false)
            .show_grid(false)
            .data_aspect(1.0)
            .show(ui, |ui| {
                // TODO: Only draw visible items

                for (item, (x, y)) in self.arranged.positions.iter() {
                    let node = weave.get_node(item).unwrap();

                    if let Some((p_x, p_y)) = node
                        .from
                        .and_then(|id| self.arranged.positions.get(&Ulid(id)))
                    {
                        ui.add(
                            Line::new(
                                "",
                                PlotPoints::Owned(vec![
                                    PlotPoint { x: *p_x, y: *p_y },
                                    PlotPoint { x: *x, y: *y },
                                ]),
                            )
                            .stroke(stroke),
                        );
                    }
                }

                for (item, (x, y)) in self.arranged.positions.iter() {
                    let node = weave.get_node(item).unwrap();

                    ui.add(
                        Polygon::new(
                            "",
                            PlotPoints::Owned(vec![
                                PlotPoint {
                                    x: x - 1.25,
                                    y: y - 1.25,
                                },
                                PlotPoint {
                                    x: x + 1.25,
                                    y: y - 1.25,
                                },
                                PlotPoint {
                                    x: x + 1.25,
                                    y: y + 1.25,
                                },
                                PlotPoint {
                                    x: x - 1.25,
                                    y: y + 1.25,
                                },
                            ]),
                        )
                        .fill_color(get_node_color(node, settings).unwrap_or(default_color)),
                    );
                }
            });

        /*ScrollArea::both()
        .animated(false)
        .auto_shrink(false)
        .show(ui, |ui| {
            let (response, painter) = ui.allocate_painter(
                Vec2 {
                    x: self.arranged.width as f32,
                    y: self.arranged.height as f32,
                },
                Sense::click_and_drag(),
            );
            let rect = response.rect;

            for (item, (x, y)) in self.arranged.positions.iter() {
                let node = weave.get_node(item).unwrap();

                if let Some((p_x, p_y)) = node
                    .from
                    .and_then(|id| self.arranged.positions.get(&Ulid(id)))
                {
                    painter.line(
                        vec![
                            Pos2 {
                                x: *p_x as f32 + rect.min.x,
                                y: *p_y as f32 + rect.min.y,
                            },
                            Pos2 {
                                x: *x as f32 + rect.min.x,
                                y: *y as f32 + rect.min.y,
                            },
                        ],
                        Stroke {
                            width: 1.0,
                            color: default_color,
                        },
                    );
                }
            }

            for (item, (x, y)) in self.arranged.positions.iter() {
                let node = weave.get_node(item).unwrap();

                painter.circle(
                    Pos2 {
                        x: *x as f32 + rect.min.x,
                        y: *y as f32 + rect.min.y,
                    },
                    2.5,
                    get_node_color(node, settings).unwrap_or(default_color),
                    Stroke::NONE,
                );
            }
        });*/

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
