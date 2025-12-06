use std::collections::HashSet;

use eframe::egui::{Color32, Tooltip, Ui};
use egui_notify::Toasts;
use egui_plot::{Line, Plot, PlotItem, PlotPoint, PlotPoints, Polygon};
use flagset::FlagSet;
use tapestry_weave::ulid::Ulid;

use crate::{
    editor::{
        lists::render_node_context_menu,
        shared::{
            NodeIndex, SharedState, get_node_color,
            layout::{ArrangedWeave, WeaveLayout},
            render_node_metadata_tooltip, render_node_text,
            weave::WeaveWrapper,
        },
    },
    settings::{Settings, shortcuts::Shortcuts},
};

#[derive(Debug)]
pub struct GraphView {
    layout: WeaveLayout,
    items: Vec<PrecalculatedItem>,
    arranged: ArrangedWeave,
    context_menu_node: Option<Ulid>,
}

impl Default for GraphView {
    fn default() -> Self {
        Self {
            layout: WeaveLayout::with_capacity(65535, 131072),
            items: Vec::with_capacity(65535 + 131072),
            arranged: ArrangedWeave::default(),
            context_menu_node: None,
        }
    }
}

impl GraphView {
    /*pub fn reset(&mut self) {
        self.arranged = ArrangedWeave::default();
    }*/
    pub fn update(
        &mut self,
        _weave: &mut WeaveWrapper,
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
        self.context_menu_node = None;

        let default_color = ui.visuals().widgets.inactive.text_color();

        let stroke_color = ui.visuals().widgets.inactive.bg_fill;
        let active_stroke_color = ui.visuals().widgets.active.fg_stroke.color;

        for (item, (x, y)) in self.arranged.positions.iter() {
            if !active.contains(item) {
                let node = weave.get_node(item).unwrap();

                if let Some((p_x, p_y)) = node
                    .from
                    .and_then(|id| self.arranged.positions.get(&Ulid(id)))
                {
                    self.items.push(PrecalculatedItem::Edge(
                        [PlotPoint { x: *p_x, y: *p_y }, PlotPoint { x: *x, y: *y }],
                        stroke_color,
                    ));
                }
            }
        }

        for (item, (x, y)) in self.arranged.positions.iter() {
            if active.contains(item) {
                let node = weave.get_node(item).unwrap();

                if let Some((p_x, p_y)) = node
                    .from
                    .and_then(|id| self.arranged.positions.get(&Ulid(id)))
                {
                    self.items.push(PrecalculatedItem::Edge(
                        [PlotPoint { x: *p_x, y: *p_y }, PlotPoint { x: *x, y: *y }],
                        active_stroke_color,
                    ));
                }
            }
        }

        for (item, (x, y)) in self.arranged.positions.iter() {
            let node = weave.get_node(item).unwrap();

            self.items.push(PrecalculatedItem::Node(
                *item,
                [
                    PlotPoint {
                        x: x - 0.5,
                        y: y - 0.5,
                    },
                    PlotPoint {
                        x: x + 0.5,
                        y: y - 0.5,
                    },
                    PlotPoint {
                        x: x + 0.5,
                        y: y + 0.5,
                    },
                    PlotPoint {
                        x: x - 0.5,
                        y: y + 0.5,
                    },
                ],
                get_node_color(node, settings).unwrap_or(default_color),
            ));
        }
    }
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut WeaveWrapper,
        settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
        shortcuts: FlagSet<Shortcuts>,
    ) {
        if self.arranged.width == 0.0 && self.arranged.height == 0.0 {
            self.layout.load_weave(
                weave,
                weave
                    .dump_identifiers_ordered_u128()
                    .into_iter()
                    .map(|id| (Ulid(id), (1.0, 1.0))),
            );
            self.arranged = self.layout.layout_weave(1.5);
            self.update_plot_cache(weave, ui, settings);
        } else if self.items.is_empty() {
            self.update_plot_cache(weave, ui, settings);
        }

        let mut pointer_node = None;

        let response = Plot::new([state.identifier.to_string(), "graph".to_string()])
            .show_x(false)
            .show_y(false)
            .invert_x(true)
            .invert_y(true)
            .show_background(false)
            .show_axes(false)
            .show_grid(false)
            .data_aspect(1.0)
            .show(ui, |ui| {
                let bounds = ui.plot_bounds();
                let pointer = ui.pointer_coordinate().filter(|pointer| {
                    ((bounds.min()[0])..=(bounds.max()[0])).contains(&pointer.x)
                        && ((bounds.min()[1])..=(bounds.max()[1])).contains(&pointer.y)
                        && !ui.response().dragged()
                });

                for item in self.items.iter().copied() {
                    match item {
                        PrecalculatedItem::Edge(points, color) => {
                            ui.add(
                                Line::new("", PlotPoints::Owned(points.to_vec()))
                                    .color(color)
                                    .allow_hover(false),
                            );
                        }
                        PrecalculatedItem::Node(id, points, color) => {
                            let polygon = Polygon::new("", PlotPoints::Owned(points.to_vec()))
                                .fill_color(color)
                                .allow_hover(true);
                            let bounds = polygon.bounds();

                            if let Some(pointer) = pointer
                                && ((bounds.min()[0])..=(bounds.max()[0])).contains(&pointer.x)
                                && ((bounds.min()[1])..=(bounds.max()[1])).contains(&pointer.y)
                            {
                                pointer_node = Some(id);
                            }

                            ui.add(polygon);
                        }
                    }
                }
            });

        if response.response.context_menu_opened() {
            if let Some(id) = self.context_menu_node {
                response.response.context_menu(|ui| {
                    render_context_menu(ui, weave, &id, settings, state);
                });
            }
        } else {
            self.context_menu_node = None;
        }

        if let Some(id) = pointer_node {
            if response.response.clicked() {
                weave.set_node_active_status(&id, true);
                state.set_cursor_node(NodeIndex::Node(id));
            }

            if self.context_menu_node.is_none() {
                response.response.context_menu(|ui| {
                    render_context_menu(ui, weave, &id, settings, state);
                    self.context_menu_node = Some(id);
                });
            }

            if !response.response.context_menu_opened() {
                Tooltip::for_widget(&response.response)
                    .at_pointer()
                    .show(|ui| {
                        render_tooltip(ui, weave, &id, settings);
                    });
                state.set_hovered_node(NodeIndex::Node(id));
            }
        }

        if shortcuts.contains(Shortcuts::FitToCursor) {
            // TODO
        }

        if shortcuts.contains(Shortcuts::FitToWeave) {
            // TODO
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum PrecalculatedItem {
    Edge([PlotPoint; 2], Color32),
    Node(Ulid, [PlotPoint; 4], Color32),
}

fn render_context_menu(
    ui: &mut Ui,
    weave: &mut WeaveWrapper,
    node: &Ulid,
    settings: &Settings,
    state: &mut SharedState,
) {
    if let Some(node) = weave.get_node(node).cloned() {
        render_node_context_menu(ui, settings, state, weave, &node);
    }
}

fn render_tooltip(ui: &mut Ui, weave: &mut WeaveWrapper, node: &Ulid, settings: &Settings) {
    if let Some(node) = weave.get_node(node) {
        ui.label(render_node_text(ui, node, settings, None));

        ui.separator();

        render_node_metadata_tooltip(ui, node);
    }
}
