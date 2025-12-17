#![allow(clippy::too_many_arguments)]

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    hash::Hash,
    rc::Rc,
    time::Instant,
};

use eframe::{
    egui::{
        Align, Button, CollapsingHeader, Color32, Layout, Pos2, Rect, RichText, Scene, Stroke,
        StrokeKind, TextStyle, Tooltip, Ui, UiBuilder, Vec2,
    },
    epaint::{ColorMode, CubicBezierShape, PathStroke},
};
use egui_notify::Toasts;
use flagset::FlagSet;
use tapestry_weave::{ulid::Ulid, v0::InnerNodeContent};

use crate::{
    editor::{
        lists::{render_horizontal_node_label_buttons_ltr, render_node_context_menu},
        shared::{
            NodeIndex, SharedState,
            layout::{WeaveLayout, wire_bezier_3},
            render_node_metadata_tooltip, render_node_text_or_empty, render_token_tooltip,
            weave::WeaveWrapper,
        },
    },
    settings::{Settings, shortcuts::Shortcuts},
};

#[derive(Debug)]
pub struct CanvasView {
    rect: Rc<RefCell<Rect>>,
    layout: WeaveLayout,
    nodes: HashMap<Ulid, CanvasNode>,
    roots: Vec<Ulid>,
    active: HashSet<Ulid>,
    last_changed: Instant,
    new: bool,
}

#[derive(Debug)]
struct CanvasNode {
    rect: Rect,
    to: Vec<Ulid>,
    to_lines: Vec<([Pos2; 4], PathStroke)>,
    max_x: f32,
    button_rect: Rect,
    button_line: ([Pos2; 2], PathStroke),
}

impl Default for CanvasView {
    fn default() -> Self {
        Self {
            rect: Rc::new(RefCell::new(Rect::ZERO)),
            layout: WeaveLayout::with_capacity(65535, 131072),
            nodes: HashMap::with_capacity(65535),
            roots: Vec::with_capacity(128),
            active: HashSet::with_capacity(65535),
            last_changed: Instant::now(),
            new: true,
        }
    }
}

impl CanvasView {
    /*pub fn reset(&mut self) {
        *self.rect.borrow_mut() = Rect::ZERO;
        self.roots.clear();
        self.new = true;
    }*/
    pub fn update(
        &mut self,
        _weave: &mut WeaveWrapper,
        _settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
        _shortcuts: FlagSet<Shortcuts>,
    ) {
        if state.has_weave_changed || state.has_theme_changed {
            self.roots.clear();
        }
    }
    fn update_layout(
        &mut self,
        ui: &mut Ui,
        weave: &mut WeaveWrapper,
        settings: &Settings,
        state: &mut SharedState,
    ) {
        let padding_base = ui.text_style_height(&TextStyle::Monospace) as f64;

        let active = HashSet::new();

        let identifiers = weave.dump_identifiers_ordered_u128_rev();

        let sizes: Vec<_> = identifiers
            .iter()
            .copied()
            .map(|id| {
                let size = calculate_size(ui, id, |ui| {
                    render_node(
                        ui,
                        weave,
                        &active,
                        settings,
                        state,
                        &Ulid(id),
                        Stroke {
                            width: ui.visuals().widgets.inactive.fg_stroke.width * 1.5,
                            color: ui.visuals().widgets.inactive.bg_fill,
                        },
                        false,
                    );
                });

                (
                    Ulid(id),
                    (size.y as f64, size.x as f64 + (padding_base * 2.0)),
                )
            })
            .collect();

        self.layout.load_weave(weave, sizes.into_iter());
        let mut arranged = self.layout.layout_weave(padding_base * 3.0);
        self.last_changed = Instant::now();

        for (_, rect) in arranged.rects.iter_mut() {
            *rect = Rect {
                min: Pos2 {
                    x: rect.min.y,
                    y: rect.min.x,
                },
                max: Pos2 {
                    x: rect.max.y,
                    y: rect.max.x,
                },
            };
        }

        self.nodes.clear();
        self.roots.extend(weave.get_roots());

        self.active = weave.get_active_thread().collect();

        let stroke_width = ui.visuals().widgets.inactive.fg_stroke.width;
        let stroke_color = ui.visuals().widgets.inactive.bg_fill;
        let active_stroke_color = ui.visuals().widgets.active.fg_stroke.color;

        let padding_base = padding_base as f32;

        for item in identifiers.into_iter().map(Ulid) {
            let rect = *arranged.rects.get(&item).unwrap();

            if let Some(node) = weave.get_node(&item) {
                let rect = Rect {
                    min: Pos2 {
                        x: rect.min.x + padding_base,
                        y: rect.min.y,
                    },
                    max: Pos2 {
                        x: rect.max.x - padding_base,
                        y: rect.max.y,
                    },
                };
                let button_rect = Rect {
                    min: Pos2 {
                        x: rect.max.x + padding_base,
                        y: rect.min.y,
                    },
                    max: Pos2 {
                        x: rect.max.x + (padding_base * 5.0),
                        y: rect.max.y,
                    },
                };

                self.nodes.insert(
                    item,
                    CanvasNode {
                        rect,
                        to: node.to.iter().copied().map(Ulid).collect(),
                        to_lines: Vec::with_capacity(node.to.len()),
                        max_x: rect.max.x + (padding_base * 4.0),
                        button_rect,
                        button_line: (
                            [
                                Pos2 {
                                    x: rect.max.x,
                                    y: (rect.min.y + (rect.max.y - rect.min.y) / 2.0),
                                },
                                Pos2 {
                                    x: button_rect.min.x,
                                    y: (button_rect.min.y
                                        + (button_rect.max.y - button_rect.min.y) / 2.0),
                                },
                            ],
                            PathStroke {
                                width: stroke_width,
                                color: ColorMode::Solid(stroke_color),
                                kind: StrokeKind::Middle,
                            },
                        ),
                    },
                );

                if let Some(parent) = node.from.map(Ulid) {
                    let p_node = self.nodes.get_mut(&parent).unwrap();
                    let p_rect = *arranged.rects.get(&parent).unwrap();

                    p_node.max_x = p_node.max_x.max(rect.min.x + padding_base);
                    p_node.to_lines.push((
                        wire_bezier_3(
                            ui.style().spacing.interact_size.y * 2.0,
                            Pos2 {
                                x: p_rect.max.x - padding_base,
                                y: (p_rect.min.y + (p_rect.max.y - p_rect.min.y) / 2.0),
                            },
                            Pos2 {
                                x: rect.min.x,
                                y: (rect.min.y + (rect.max.y - rect.min.y) / 2.0),
                            },
                        ),
                        PathStroke {
                            width: stroke_width,
                            color: ColorMode::Solid(if !self.active.contains(&item) {
                                stroke_color
                            } else {
                                active_stroke_color
                            }),
                            kind: StrokeKind::Middle,
                        },
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
        let mut changed_node = if settings.interface.auto_scroll {
            state.get_changed_node()
        } else {
            None
        };

        if self.roots.is_empty() {
            self.update_layout(ui, weave, settings, state);

            if self.new {
                changed_node = state.get_cursor_node().into_node();
                self.new = false;
            }
        }

        let mut focus: Option<Rect> = None;

        let last_rect = *self.rect.borrow();

        let clip_rect = ui.clip_rect();
        let outer_rect = ui.available_rect_before_wrap();

        let inactive_stroke = Stroke {
            width: ui.visuals().widgets.inactive.fg_stroke.width * 1.5,
            color: ui.visuals().widgets.inactive.bg_fill,
        };
        let active_stroke = Stroke {
            width: ui.visuals().widgets.inactive.fg_stroke.width * 1.5,
            color: ui.visuals().widgets.active.fg_stroke.color,
        };

        let scene_rect = self.rect.clone();

        Scene::new().show(ui, &mut scene_rect.borrow_mut(), |ui| {
            if clip_rect.contains(ui.ctx().pointer_hover_pos().unwrap_or_default())
                && last_rect != Rect::ZERO
            {
                changed_node = None;
            }

            if shortcuts.contains(Shortcuts::FitToCursor) {
                changed_node = state.get_cursor_node().into_node();
            }

            if ui.is_visible() {
                let show_tooltip = self.last_changed.elapsed().as_secs_f32()
                    >= ui.style().interaction.tooltip_delay;

                for root in &self.roots {
                    self.traverse_and_focus(root, &mut focus, &outer_rect, changed_node, state);
                    self.traverse_and_paint(
                        ui,
                        root,
                        &active_stroke,
                        &inactive_stroke,
                        show_tooltip,
                        &mut (weave, settings, state),
                        last_rect == Rect::ZERO,
                    );
                }
            }
        });

        if let Some(focus) = focus {
            *self.rect.borrow_mut() = focus;
        }

        if shortcuts.contains(Shortcuts::FitToWeave) {
            *self.rect.borrow_mut() = Rect::ZERO;
        }

        if *self.rect.borrow() != last_rect {
            self.last_changed = Instant::now();
        }
    }
    fn traverse_and_focus(
        &self,
        node: &Ulid,
        focus: &mut Option<Rect>,
        outer_rect: &Rect,
        changed_node: Option<Ulid>,
        state: &SharedState,
    ) {
        let canvas_node = self.nodes.get(node).unwrap();

        if Some(*node) == changed_node {
            let rect = canvas_node.rect;
            let scale = (outer_rect.size() / rect.size()).min_elem();

            if scale > 0.9 {
                *focus = Some(rect.scale_from_center(scale * (1.0 / 0.9)));
            } else {
                *focus = Some(rect);
            }
        }

        if state.is_open(node) {
            for child in &canvas_node.to {
                self.traverse_and_focus(child, focus, outer_rect, changed_node, state);
            }
        }
    }
    fn traverse_and_paint(
        &self,
        ui: &mut Ui,
        node: &Ulid,
        active_stroke: &Stroke,
        inactive_stroke: &Stroke,
        show_tooltip: bool,
        render_state: &mut (&mut WeaveWrapper, &Settings, &mut SharedState),
        disable_culling: bool,
    ) {
        let canvas_node = self.nodes.get(node).unwrap();

        if ui.clip_rect().min.x > canvas_node.max_x && !disable_culling {
            for child in &canvas_node.to {
                self.traverse_and_paint(
                    ui,
                    child,
                    active_stroke,
                    inactive_stroke,
                    show_tooltip,
                    render_state,
                    disable_culling,
                );
            }
        } else {
            if ui.clip_rect().max.x >= canvas_node.rect.max.x || disable_culling {
                if render_state.2.is_open(node) {
                    let painter = ui.painter();

                    for (points, stroke) in canvas_node.to_lines.iter().cloned() {
                        painter.add(CubicBezierShape {
                            points,
                            closed: false,
                            fill: Color32::TRANSPARENT,
                            stroke,
                        });
                    }

                    for child in &canvas_node.to {
                        self.traverse_and_paint(
                            ui,
                            child,
                            active_stroke,
                            inactive_stroke,
                            show_tooltip,
                            render_state,
                            disable_culling,
                        );
                    }
                } else if (ui.is_rect_visible(canvas_node.button_rect) || disable_culling)
                    && should_render_expand_button(node, render_state.0)
                {
                    let painter = ui.painter();

                    painter.line(
                        canvas_node.button_line.0.to_vec(),
                        canvas_node.button_line.1.clone(),
                    );

                    ui.scope_builder(
                        UiBuilder::new()
                            .max_rect(canvas_node.button_rect)
                            .layout(Layout::left_to_right(Align::Center)),
                        |ui| {
                            render_expand_button(
                                ui,
                                render_state.0,
                                render_state.2,
                                node,
                                *inactive_stroke,
                            );
                        },
                    );
                }
            }

            ui.scope_builder(UiBuilder::new().max_rect(canvas_node.rect), |ui| {
                if ui.is_rect_visible(canvas_node.rect) || disable_culling {
                    render_node(
                        ui,
                        render_state.0,
                        &self.active,
                        render_state.1,
                        render_state.2,
                        node,
                        if self.active.contains(node) {
                            *active_stroke
                        } else {
                            *inactive_stroke
                        },
                        show_tooltip,
                    );
                }
            });
        }
    }
}

fn render_node(
    ui: &mut Ui,
    weave: &mut WeaveWrapper,
    active: &HashSet<Ulid>,
    settings: &Settings,
    state: &mut SharedState,
    node: &Ulid,
    mut stroke: Stroke,
    show_tooltip: bool,
) {
    let hovered_node = state.get_hovered_node().into_node();
    let cursor_node = state.get_cursor_node().into_node();

    ui.set_max_width(ui.spacing().text_edit_width * 1.2);

    if let Some(node) = weave.get_node(node).cloned() {
        if node.bookmarked {
            if active.contains(&Ulid(node.id)) {
                stroke.color = ui.visuals().selection.stroke.color;
            } else {
                stroke.color = ui.visuals().selection.bg_fill;
            }
        }

        if cursor_node == Some(Ulid(node.id)) {
            stroke.width *= 2.0;
        }

        let mut button = Button::new(render_node_text_or_empty(ui, &node, settings, None))
            .fill(Color32::TRANSPARENT)
            .stroke(stroke)
            .min_size(Vec2 {
                x: ui.spacing().text_edit_width * 1.2,
                y: ui.text_style_height(&TextStyle::Monospace) * 3.0,
            })
            .wrap();

        if hovered_node == Some(Ulid(node.id)) {
            button = button.fill(ui.style().visuals.widgets.hovered.weak_bg_fill);
        }

        let response = ui.add(button);

        if ui.is_sizing_pass() {
            return;
        }

        response.context_menu(|ui| {
            render_node_context_menu(ui, settings, state, weave, &node, true);
        });

        if response.contains_pointer() {
            state.set_hovered_node(NodeIndex::Node(Ulid(node.id)));
        }

        if response.clicked() {
            weave.set_node_active_status_u128(&node.id, true);
            state.set_cursor_node(NodeIndex::Node(Ulid(node.id)));
        }

        let mut tooltip = Tooltip::for_enabled(&response);

        tooltip.popup = tooltip.popup.open(
            (response.contains_pointer() || Tooltip::should_show_tooltip(&response, true))
                && show_tooltip,
        );

        tooltip.show(|ui| {
            ui.horizontal(|ui| {
                render_horizontal_node_label_buttons_ltr(ui, settings, state, weave, &node);
                if !node.to.is_empty() {
                    render_collapsing_button(ui, state, &Ulid(node.id));
                }
            });

            if let InnerNodeContent::Tokens(tokens) = &node.contents.content
                && tokens.len() == 1
                && let Some(token) = tokens.first()
            {
                ui.add_space(ui.spacing().menu_spacing);
                render_token_tooltip(ui, &token.0, &token.1);
            }

            ui.separator();

            CollapsingHeader::new("Node Information").show_unindented(ui, |ui| {
                render_node_metadata_tooltip(ui, &node);
            });
        });
    }
}

fn should_render_expand_button(node: &Ulid, weave: &WeaveWrapper) -> bool {
    if let Some(node) = weave.get_node(node).cloned()
        && !node.to.is_empty()
    {
        true
    } else {
        false
    }
}

fn render_expand_button(
    ui: &mut Ui,
    weave: &mut WeaveWrapper,
    state: &mut SharedState,
    node: &Ulid,
    stroke: Stroke,
) {
    if let Some(weave_node) = weave.get_node(node).cloned()
        && let Some(hover_node) = weave_node.to.first().copied().map(Ulid)
    {
        let is_hovered = state.get_hovered_node() == NodeIndex::Node(hover_node);

        let response = ui.add(
            Button::new(RichText::new("...").size(ui.text_style_height(&TextStyle::Monospace)))
                .min_size(Vec2 {
                    x: ui.text_style_height(&TextStyle::Monospace) * 1.75,
                    y: ui.text_style_height(&TextStyle::Monospace) * 1.75,
                })
                .fill(if !is_hovered {
                    Color32::TRANSPARENT
                } else {
                    ui.style().visuals.widgets.hovered.weak_bg_fill
                })
                .stroke(stroke),
        );

        if response.contains_pointer() {
            state.set_hovered_node(NodeIndex::Node(hover_node));
        }

        if response.clicked() {
            state.set_open(*node, true);
        }

        ui.set_max_width(ui.min_rect().width());
    }
}

fn calculate_size(ui: &Ui, hash: impl Hash, contents: impl FnOnce(&mut Ui)) -> Vec2 {
    let mut ui = Ui::new(
        ui.ctx().clone(),
        ui.id().with(hash),
        UiBuilder::new().invisible().sizing_pass(),
    );
    contents(&mut ui);
    ui.min_size()
}

fn render_collapsing_button(ui: &mut Ui, state: &mut SharedState, node: &Ulid) {
    let is_open = state.is_open(node);

    let label = if is_open { "\u{E43C}" } else { "\u{E43E}" };
    let hover_text = if is_open {
        "Collapse node"
    } else {
        "Expand node"
    };
    if ui.button(label).on_hover_text(hover_text).clicked() {
        state.toggle_open(*node);
    };
}
