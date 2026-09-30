use std::mem;

use eframe::egui::{
    self, Color32, Context, InnerResponse, NumExt, Rect, Scene, Shape, Stroke, StrokeKind, Tooltip,
    Ui, WidgetText,
    epaint::{CubicBezierShape, RectShape},
    vec2,
};
use tapestry_weave::{
    ShortId,
    universal_weave::{
        LayoutItem, Layouter, Weave,
        glam::Vec2,
        layout::{Spacing, smooth},
    },
    weave::layout::TapestryLayouter,
};

use crate::{
    common::view::View,
    editor::{
        EditorShared,
        shared::ui::{DocumentContextFlags, LayoutFit, TextFlags, TokenTooltipFlags},
    },
};

#[derive(Debug)]
pub struct GraphView {
    layouter: TapestryLayouter,
    arranged: bool,
    view: Option<Rect>,
    fit_weave: bool,
    context_node: Option<ShortId>,
}

impl Default for GraphView {
    fn default() -> Self {
        Self {
            layouter: TapestryLayouter::new(Spacing {
                node: 1.0,
                layer: 2.0,
                corridor: 0.0,
                edge: 0.25,
            }),
            arranged: false,
            view: None,
            fit_weave: false,
            context_node: None,
        }
    }
}

impl View<EditorShared> for GraphView {
    fn title(&self, _shared: &EditorShared) -> WidgetText {
        WidgetText::Text("\u{E52E} Graph".to_string())
    }
    fn logic(&mut self, shared: &mut EditorShared, _force_close: impl FnOnce(), _ctx: &Context) {
        if shared.weave_changed {
            self.arranged = false;
        }
    }
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {
        let Some(weave) = &mut shared.weave else {
            return;
        };

        let view_size = ui.available_size().at_least(egui::Vec2::splat(64.0)) / 15.0;
        let contains_pointer = ui.rect_contains_pointer(ui.clip_rect());

        let anchor = if mem::replace(&mut self.arranged, true) {
            None
        } else {
            let anchor = shared.ui.cursor.and_then(|cursor| {
                self.layouter
                    .center(&cursor)
                    .map(|position| position.to_array().into())
            });
            self.layouter.layout(weave, |_| Vec2::ONE);
            anchor
        };

        let cursor_position = shared.ui.cursor.and_then(|cursor| {
            self.layouter
                .center(&cursor)
                .map(|position| position.to_array().into())
        });
        let weave_view = || {
            let size = egui::Vec2::from(self.layouter.size().to_array());
            let scale = (size / view_size).max_elem().max(1.0) * 1.05;

            Rect::from_center_size((size * 0.5).to_pos2(), view_size * scale)
        };

        let mut view = if mem::take(&mut self.fit_weave) || shared.ui.fit == LayoutFit::Weave {
            weave_view()
        } else if (shared.ui.fit == LayoutFit::Cursor || self.view.is_none())
            && let Some(position) = cursor_position
        {
            Rect::from_center_size(position, view_size)
        } else if let Some(mut view) = self.view {
            let visible = |view: Rect| {
                let scale = (view.size() / view_size).max_elem();
                Rect::from_center_size(view.center(), view_size * scale)
            };

            if let Some(anchor) = anchor
                && let Some(position) = cursor_position
                && visible(view).contains(anchor)
            {
                view = view.translate(position - anchor);
            }

            if let Some(node) = shared.ui.autoscroll_target()
                && let Some(position) = self
                    .layouter
                    .center(&node)
                    .map(|position| position.to_array().into())
                && (!contains_pointer
                    || (shared.ui.cursor == Some(node) && !visible(view).contains(position)))
            {
                view = Rect::from_center_size(position, view_size);
            }

            view
        } else {
            weave_view()
        };

        let InnerResponse {
            response,
            inner: painter,
        } = Scene::new()
            .zoom_range(f32::EPSILON..=240.0)
            .show(ui, &mut view, |ui| ui.painter().clone());

        self.view = Some(view);

        let zoom = ui
            .ctx()
            .layer_transform_to_global(painter.layer_id())
            .map_or(1.0, |transform| transform.scaling);

        let default_color = ui.visuals().widgets.inactive.text_color();
        let stroke_color = ui.visuals().widgets.inactive.bg_fill;
        let active_stroke_color = ui.visuals().widgets.noninteractive.fg_stroke.color;
        let icon_color = ui.visuals().panel_fill;

        let edge_width = 2.0 / zoom;
        let hover_stroke = Stroke::new(2.75 / zoom, active_stroke_color);
        let cursor_stroke = Stroke::new(hover_stroke.width, stroke_color);

        let view = painter.clip_rect().expand(hover_stroke.width);
        let pointer = response.hover_pos().filter(|_| !response.dragged());

        let mut pointer_node = None;

        let inactive_edges_idx = painter.add(Shape::Noop);
        let mut inactive_edges = Vec::new();

        self.layouter.view(
            Vec2::from_array(view.min.into()),
            Vec2::from_array(view.max.into()),
            |item| match item {
                LayoutItem::Polyline { from, to, points } => {
                    let smoothed = smooth(points)
                        .into_iter()
                        .map(|segment| segment.map(|segment| segment.to_array().into()));

                    if weave.contains_active(&from) && weave.contains_active(&to) {
                        painter.extend(smoothed.flat_map(|segment| {
                            CubicBezierShape::from_points_stroke(
                                segment,
                                false,
                                Color32::TRANSPARENT,
                                Stroke::new(edge_width, active_stroke_color),
                            )
                            .to_path_shapes(Some(0.25 / zoom), None)
                            .into_iter()
                            .map(Shape::Path)
                        }));
                    } else {
                        inactive_edges.extend(smoothed.flat_map(|segment| {
                            CubicBezierShape::from_points_stroke(
                                segment,
                                false,
                                Color32::TRANSPARENT,
                                Stroke::new(edge_width, stroke_color),
                            )
                            .to_path_shapes(Some(0.25 / zoom), None)
                            .into_iter()
                            .map(Shape::Path)
                        }));
                    }
                }
                LayoutItem::Node { id, center, size } => {
                    let node = weave.get(&id).unwrap();

                    let bounds =
                        Rect::from_center_size(center.to_array().into(), size.to_array().into());

                    let hovered = if pointer.is_some_and(|pointer| bounds.contains(pointer)) {
                        pointer_node = Some(id);
                        true
                    } else {
                        shared.ui.is_hovered(&id)
                    };

                    let shape = Shape::Rect(
                        RectShape::new(
                            bounds,
                            0.0,
                            shared.ui.node_color(node).unwrap_or(default_color),
                            if hovered {
                                hover_stroke
                            } else if shared.ui.cursor == Some(id) {
                                cursor_stroke
                            } else {
                                Stroke::NONE
                            },
                            StrokeKind::Middle,
                        )
                        .with_round_to_pixels(false),
                    );

                    if node.bookmarked {
                        painter.extend([
                            shape,
                            Shape::convex_polygon(
                                [
                                    vec2(0.25, 0.15),
                                    vec2(0.75, 0.15),
                                    vec2(0.75, 0.85),
                                    vec2(0.5, 0.7),
                                    vec2(0.25, 0.85),
                                ]
                                .into_iter()
                                .map(|point| bounds.lerp_inside(point))
                                .collect(),
                                icon_color,
                                Stroke::NONE,
                            ),
                        ]);
                    } else {
                        painter.add(shape);
                    }
                }
            },
        );

        painter.set(inactive_edges_idx, Shape::Vec(inactive_edges));

        if response
            .context_menu(|ui| {
                if let Some(node) = self.context_node {
                    if let Some(node) = weave.get(&node).cloned() {
                        shared.ui.node_context_menu(weave, &node, ui, false);
                    }
                } else {
                    shared
                        .ui
                        .document_context_menu(weave, ui, DocumentContextFlags::Roots.into());
                }
            })
            .is_none()
        {
            self.context_node = pointer_node;
        }

        if let Some(id) = pointer_node {
            if response.clicked() {
                shared.ui.activate_node(weave, id, &response, false);
            }

            if !response.context_menu_opened() {
                Tooltip::for_widget(&response).at_pointer().show(|ui| {
                    if let Some(node) = weave.get(&id) {
                        ui.label(shared.ui.node_text(
                            ui,
                            node,
                            TextFlags::EmptyNotice | TextFlags::FirstTokenBytes,
                        ));
                        ui.separator();
                        shared.ui.node_hover_tooltip(
                            ui,
                            node,
                            TokenTooltipFlags::WarnModified.into(),
                        );
                    }
                });
                shared.ui.set_hovered(id);
            }
        } else if response.double_clicked() {
            self.fit_weave = true;
            ui.ctx().request_repaint();
        }
    }
}
