use std::collections::HashSet;

use eframe::egui::{Color32, Stroke, Tooltip, Ui, Vec2};
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
        if state.has_weave_changed || state.has_theme_changed {
            self.items.clear();
        }
    }
    fn update_plot_cache(&mut self, weave: &mut WeaveWrapper, ui: &Ui, settings: &Settings) {
        let active: HashSet<Ulid> = weave.get_active_thread().collect();

        self.items.clear();
        self.context_menu_node = None;

        let default_color = ui.visuals().widgets.inactive.text_color();

        let stroke_color = ui.visuals().widgets.inactive.bg_fill;
        let active_stroke_color = ui.visuals().widgets.noninteractive.fg_stroke.color;

        let icon_color = ui.visuals().panel_fill;

        for (item, (x, y)) in self.arranged.positions.iter() {
            if !active.contains(item)
                && let Some(node) = weave.get_node(item)
                && let Some((p_x, p_y)) = node
                    .from
                    .and_then(|id| self.arranged.positions.get(&Ulid(id)))
            {
                self.items.push(PrecalculatedItem::Edge(
                    [PlotPoint { x: *p_x, y: *p_y }, PlotPoint { x: *x, y: *y }],
                    stroke_color,
                ));
            }
        }

        for (item, (x, y)) in self.arranged.positions.iter() {
            if active.contains(item)
                && let Some(node) = weave.get_node(item)
                && let Some((p_x, p_y)) = node
                    .from
                    .and_then(|id| self.arranged.positions.get(&Ulid(id)))
            {
                self.items.push(PrecalculatedItem::Edge(
                    [PlotPoint { x: *p_x, y: *p_y }, PlotPoint { x: *x, y: *y }],
                    active_stroke_color,
                ));
            }
        }

        for (item, (x, y)) in self.arranged.positions.iter() {
            if let Some(node) = weave.get_node(item) {
                self.items.push(PrecalculatedItem::Node(
                    *item,
                    vec![
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
                    PlotPoint { x: *x, y: *y },
                    get_node_color(node, settings).unwrap_or(default_color),
                ));

                if node.bookmarked {
                    self.items.push(PrecalculatedItem::Shape(
                        vec![
                            PlotPoint {
                                x: x - 0.25,
                                y: y - 0.35,
                            },
                            PlotPoint {
                                x: x + 0.25,
                                y: y - 0.35,
                            },
                            PlotPoint {
                                x: x + 0.25,
                                y: y + 0.35,
                            },
                            PlotPoint { x: *x, y: y + 0.2 },
                            PlotPoint {
                                x: x - 0.25,
                                y: y + 0.35,
                            },
                            PlotPoint {
                                x: x - 0.25,
                                y: y - 0.35,
                            },
                        ],
                        icon_color,
                    ));
                }
            }
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
        let mut fitting_node = None;

        if self.arranged.width == 0.0 && self.arranged.height == 0.0 {
            // TODO: Perform layout on a background thread
            self.layout.load_weave(
                weave,
                weave
                    .dump_identifiers_ordered_u128()
                    .into_iter()
                    .map(|id| (Ulid(id), (1.0, 1.0))),
            );
            self.arranged = self.layout.layout_weave(1.5);
            self.update_plot_cache(weave, ui, settings);
            fitting_node = state.get_cursor_node().into_node();
        } else if self.items.is_empty() {
            self.update_plot_cache(weave, ui, settings);
            fitting_node = state.get_cursor_node().into_node();
        }

        let mut pointer_node = None;

        let hover_stroke = Stroke {
            color: ui.visuals().widgets.noninteractive.fg_stroke.color, // Same as active_stroke_color in update_plot_cache()
            width: 2.0,
        };
        let hovered_node = state.get_hovered_node().into_node();

        let cursor_stroke = Stroke {
            color: ui.visuals().widgets.inactive.bg_fill, // Same as stroke_color in update_plot_cache()
            width: hover_stroke.width,
        };
        let cursor_node = state.get_cursor_node().into_node();

        let changed_node = state.get_changed_node();

        if shortcuts.contains(Shortcuts::FitToCursor) {
            fitting_node = cursor_node;
            println!("{:?}", cursor_node);
        }

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

                let screen_area = ui.response().rect;
                let fitting_area = Vec2 {
                    x: screen_area.width() / 15.0,
                    y: screen_area.height() / 15.0,
                };

                for item in self.items.iter() {
                    match item {
                        PrecalculatedItem::Edge(points, color) => {
                            ui.add(
                                Line::new("", PlotPoints::Borrowed(points))
                                    .color(*color)
                                    .width(2.0)
                                    .allow_hover(false),
                            );
                        }
                        PrecalculatedItem::Node(id, points, location, color) => {
                            let mut polygon = Polygon::new("", PlotPoints::Borrowed(points))
                                .fill_color(*color)
                                .allow_hover(false);
                            let bounds = polygon.bounds();
                            let id = *id;

                            if let Some(pointer) = pointer
                                && ((bounds.min()[0])..=(bounds.max()[0])).contains(&pointer.x)
                                && ((bounds.min()[1])..=(bounds.max()[1])).contains(&pointer.y)
                            {
                                pointer_node = Some(id);
                                polygon = polygon.stroke(hover_stroke);
                            } else if Some(id) == hovered_node {
                                polygon = polygon.stroke(hover_stroke);
                            } else if Some(id) == cursor_node {
                                polygon = polygon.stroke(cursor_stroke);
                            }

                            if (settings.interface.auto_scroll
                                && pointer.is_none()
                                && fitting_node.is_none()
                                && Some(id) == changed_node)
                                || (Some(id) == fitting_node)
                            {
                                let mut bounds = ui.plot_bounds();
                                bounds.set_x_center_width(location.x, fitting_area.x as f64);
                                bounds.set_y_center_height(location.y, fitting_area.y as f64);
                                ui.set_plot_bounds(bounds);
                            }

                            ui.add(polygon);
                        }
                        PrecalculatedItem::Shape(points, color) => {
                            ui.add(
                                Polygon::new("", PlotPoints::Borrowed(points))
                                    .fill_color(*color)
                                    .allow_hover(false),
                            );
                        }
                    }
                }

                if shortcuts.contains(Shortcuts::FitToWeave) {
                    ui.set_auto_bounds(true);
                }

                fitting_node = None;
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
    }
}

#[derive(Debug, Clone)]
enum PrecalculatedItem {
    Edge([PlotPoint; 2], Color32),
    Node(Ulid, Vec<PlotPoint>, PlotPoint, Color32),
    Shape(Vec<PlotPoint>, Color32),
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
