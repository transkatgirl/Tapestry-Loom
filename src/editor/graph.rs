use std::collections::HashSet;

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
    items: Vec<PrecalculatedItem>,
    arranged: ArrangedWeave,
}

impl Default for GraphView {
    fn default() -> Self {
        Self {
            layout: WeaveLayout::with_capacity(65535, 262144),
            items: Vec::with_capacity(65535 + 262144),
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
        if state.has_weave_changed {
            self.items.clear();
        }
    }
    fn update_plot_cache(&mut self, weave: &mut WeaveWrapper, ui: &Ui, settings: &Settings) {
        let active: HashSet<Ulid> = weave.get_active_thread().collect();

        self.items.clear();

        let default_color = ui.visuals().widgets.inactive.text_color();

        let stroke_color = ui.visuals().widgets.inactive.bg_fill;
        let active_stroke_color = ui.visuals().selection.bg_fill;

        for (item, (x, y)) in self.arranged.positions.iter() {
            let node = weave.get_node(item).unwrap();

            if let Some((p_x, p_y)) = node
                .from
                .and_then(|id| self.arranged.positions.get(&Ulid(id)))
            {
                self.items.push(PrecalculatedItem::Edge(
                    [PlotPoint { x: *p_x, y: *p_y }, PlotPoint { x: *x, y: *y }],
                    if active.contains(item) {
                        active_stroke_color
                    } else {
                        stroke_color
                    },
                ));
            }
        }

        for (item, (x, y)) in self.arranged.positions.iter() {
            let node = weave.get_node(item).unwrap();

            self.items.push(PrecalculatedItem::Node(
                *item,
                PlotPoint { x: *x, y: *y },
                get_node_color(node, settings).unwrap_or(default_color),
            ));
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
                    .map(|id| (Ulid(id), (1.0, 1.0))),
            );
            self.arranged = self.layout.layout_weave(2.0);
            self.update_plot_cache(weave, ui, settings);
        } else if self.items.is_empty() {
            self.update_plot_cache(weave, ui, settings);
        }

        let response = Plot::new([state.identifier.to_string(), "graph".to_string()])
            .show_x(false)
            .show_y(false)
            .invert_y(true)
            .show_background(false)
            .show_axes(false)
            .show_grid(false)
            .data_aspect(1.0)
            .show(ui, |ui| {
                for item in self.items.iter().copied() {
                    match item {
                        PrecalculatedItem::Edge(points, color) => {
                            ui.add(
                                Line::new("", PlotPoints::Owned(points.to_vec()))
                                    .color(color)
                                    .allow_hover(false),
                            );
                        }
                        PrecalculatedItem::Node(id, point, color) => {
                            ui.add(
                                Polygon::new(
                                    "",
                                    PlotPoints::Owned(vec![
                                        PlotPoint {
                                            x: point.x - 0.5,
                                            y: point.y - 0.5,
                                        },
                                        PlotPoint {
                                            x: point.x + 0.5,
                                            y: point.y - 0.5,
                                        },
                                        PlotPoint {
                                            x: point.x + 0.5,
                                            y: point.y + 0.5,
                                        },
                                        PlotPoint {
                                            x: point.x - 0.5,
                                            y: point.y + 0.5,
                                        },
                                    ]),
                                )
                                .id(id.to_string())
                                .fill_color(color),
                            );
                        }
                    }
                }
            });

        /*if shortcuts.contains(Shortcuts::FitToCursor) {
            // TODO
        }

        if shortcuts.contains(Shortcuts::FitToWeave) {
            // TODO
        }*/
    }
}

#[derive(Debug, Clone, Copy)]
enum PrecalculatedItem {
    Edge([PlotPoint; 2], Color32),
    Node(Ulid, PlotPoint, Color32),
}
