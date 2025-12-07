use std::{collections::HashSet, hash::Hash, time::Instant};

use eframe::{
    egui::{
        Button, Color32, Pos2, Rect, Scene, Stroke, StrokeKind, TextStyle, Tooltip, Ui, UiBuilder,
        Vec2,
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
            layout::{ArrangedWeave, WeaveLayout, wire_bezier_3},
            render_node_metadata_tooltip, render_node_text, render_token_metadata_tooltip,
            weave::WeaveWrapper,
        },
    },
    settings::{Settings, shortcuts::Shortcuts},
};

#[derive(Debug)]
pub struct CanvasView {
    rect: Rect,
    layout: WeaveLayout,
    arranged: ArrangedWeave,
    lines: Vec<([Pos2; 4], PathStroke)>,
    last_changed: Instant,
}

impl Default for CanvasView {
    fn default() -> Self {
        Self {
            rect: Rect::ZERO,
            layout: WeaveLayout::with_capacity(65535, 131072),
            arranged: ArrangedWeave::default(),
            lines: Vec::with_capacity(131072),
            last_changed: Instant::now(),
        }
    }
}

impl CanvasView {
    pub fn reset(&mut self) {
        self.rect = Rect::ZERO;
        self.arranged = ArrangedWeave::default();
    }
    pub fn update(
        &mut self,
        _weave: &mut WeaveWrapper,
        _settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
        _shortcuts: FlagSet<Shortcuts>,
    ) {
        if state.has_weave_changed || state.has_theme_changed {
            self.arranged = ArrangedWeave::default();
        }
    }
    fn update_layout(
        &mut self,
        ui: &mut Ui,
        weave: &mut WeaveWrapper,
        settings: &Settings,
        state: &mut SharedState,
    ) {
        let sizes: Vec<_> = weave
            .dump_identifiers_ordered_u128_rev()
            .into_iter()
            .map(|id| {
                let size = calculate_size(ui, id, |ui| {
                    render_node(ui, weave, settings, state, &Ulid(id), false);
                });

                (Ulid(id), (size.y as f64, size.x as f64))
            })
            .collect();

        self.layout.load_weave(weave, sizes.into_iter());
        self.arranged = self
            .layout
            .layout_weave(ui.text_style_height(&TextStyle::Monospace) as f64 * 4.0);
        self.last_changed = Instant::now();

        for (_, rect) in self.arranged.rects.iter_mut() {
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

        self.lines.clear();

        let active: HashSet<Ulid> = weave.get_active_thread().collect();

        let stroke_width = ui.visuals().widgets.noninteractive.fg_stroke.width;
        let stroke_color = ui.visuals().widgets.inactive.bg_fill;
        let active_stroke_color = ui.visuals().widgets.noninteractive.fg_stroke.color;

        for (item, rect) in self.arranged.rects.iter() {
            if !active.contains(item)
                && let Some(node) = weave.get_node(item)
                && let Some(p_rect) = node.from.and_then(|id| self.arranged.rects.get(&Ulid(id)))
            {
                self.lines.push((
                    wire_bezier_3(
                        ui.style().spacing.interact_size.y * 2.0,
                        Pos2 {
                            x: p_rect.max.x,
                            y: (p_rect.min.y + (p_rect.max.y - p_rect.min.y) / 2.0),
                        },
                        Pos2 {
                            x: rect.min.x,
                            y: (rect.min.y + (rect.max.y - rect.min.y) / 2.0),
                        },
                    ),
                    PathStroke {
                        width: stroke_width,
                        color: ColorMode::Solid(stroke_color),
                        kind: StrokeKind::Middle,
                    },
                ));
            }
        }

        for (item, rect) in self.arranged.rects.iter() {
            if active.contains(item)
                && let Some(node) = weave.get_node(item)
                && let Some(p_rect) = node.from.and_then(|id| self.arranged.rects.get(&Ulid(id)))
            {
                self.lines.push((
                    wire_bezier_3(
                        ui.style().spacing.interact_size.y * 2.0,
                        Pos2 {
                            x: p_rect.max.x,
                            y: (p_rect.min.y + (p_rect.max.y - p_rect.min.y) / 2.0),
                        },
                        Pos2 {
                            x: rect.min.x,
                            y: (rect.min.y + (rect.max.y - rect.min.y) / 2.0),
                        },
                    ),
                    PathStroke {
                        width: stroke_width,
                        color: ColorMode::Solid(active_stroke_color),
                        kind: StrokeKind::Middle,
                    },
                ));
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
        if self.arranged.width == 0.0 && self.arranged.height == 0.0 {
            self.update_layout(ui, weave, settings, state);
        }

        let mut focus: Option<Rect> = None;

        let mut changed_node = if settings.interface.auto_scroll {
            state.get_changed_node()
        } else {
            None
        };

        let last_rect = self.rect;

        Scene::new().show(ui, &mut self.rect, |ui| {
            let painter = ui.painter();

            if ui.response().contains_pointer() {
                changed_node = None;
            }

            if shortcuts.contains(Shortcuts::FitToCursor) {
                changed_node = state.get_cursor_node().into_node();
            }

            for (points, stroke) in self.lines.iter().cloned() {
                painter.add(CubicBezierShape {
                    points,
                    closed: false,
                    fill: Color32::TRANSPARENT,
                    stroke,
                });
            }

            for (node, rect) in &self.arranged.rects {
                ui.scope_builder(UiBuilder::new().max_rect(*rect), |ui| {
                    render_node(
                        ui,
                        weave,
                        settings,
                        state,
                        node,
                        self.last_changed.elapsed().as_secs_f32()
                            >= ui.style().interaction.tooltip_delay,
                    );
                });

                if Some(*node) == changed_node {
                    focus = Some(*rect);
                }
            }
        });

        if let Some(focus) = focus {
            self.rect = focus;
        }

        if shortcuts.contains(Shortcuts::FitToWeave) {
            self.rect = Rect::ZERO;
        }

        if self.rect != last_rect {
            self.last_changed = Instant::now();
        }
    }
}

fn render_node(
    ui: &mut Ui,
    weave: &mut WeaveWrapper,
    settings: &Settings,
    state: &mut SharedState,
    node: &Ulid,
    show_tooltip: bool,
) {
    let hovered_node = state.get_hovered_node().into_node();
    let cursor_node = state.get_cursor_node().into_node();

    let stroke_width = ui.visuals().widgets.noninteractive.fg_stroke.width * 1.5;

    if let Some(node) = weave.get_node(node).cloned() {
        let mut button = Button::new(render_node_text(ui, &node, settings, None))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke {
                width: stroke_width,
                color: ui.visuals().widgets.noninteractive.bg_stroke.color,
            })
            .min_size(Vec2 {
                x: ui.spacing().text_edit_width,
                y: ui.text_style_height(&TextStyle::Monospace) * 3.0,
            });

        if cursor_node == Some(Ulid(node.id)) {
            button = button
                .stroke(Stroke {
                    width: stroke_width,
                    color: ui.visuals().widgets.noninteractive.fg_stroke.color,
                })
                .selected(true);
        }

        if hovered_node == Some(Ulid(node.id)) {
            button = button.fill(ui.style().visuals.widgets.hovered.weak_bg_fill);
        }

        if node.bookmarked {
            button = button.stroke(Stroke {
                width: stroke_width,
                color: ui.visuals().selection.bg_fill,
            });
        }

        let response = ui.add(button);

        response.context_menu(|ui| {
            render_node_context_menu(ui, settings, state, weave, &node);
        });

        if response.clicked() {
            weave.set_node_active_status_u128(&node.id, true);
            state.set_cursor_node(NodeIndex::Node(Ulid(node.id)));
        }

        if response.contains_pointer() {
            state.set_hovered_node(NodeIndex::Node(Ulid(node.id)));
        }

        let mut tooltip = Tooltip::for_enabled(&response);

        tooltip.popup = tooltip.popup.open(
            (response.contains_pointer() || Tooltip::should_show_tooltip(&response, true))
                && show_tooltip,
        );

        tooltip.show(|ui| {
            ui.horizontal(|ui| {
                render_horizontal_node_label_buttons_ltr(ui, settings, state, weave, &node);
            });

            ui.separator();

            ui.collapsing("Node Information", |ui| {
                if let InnerNodeContent::Tokens(tokens) = &node.contents.content
                    && tokens.len() == 1
                    && let Some(token) = tokens.first()
                {
                    render_token_metadata_tooltip(ui, token.0.len(), &token.1);

                    ui.separator();
                }

                render_node_metadata_tooltip(ui, &node);
            });
        });
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
