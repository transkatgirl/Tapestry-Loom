use eframe::{
    egui::{
        Color32, Context, InnerResponse, Pos2, Rect, Shape, Stroke, StrokeKind, Tooltip, Ui, Vec2,
        WidgetText, epaint::RectShape,
    },
    epaint::CubicBezierShape,
};
use tapestry_weave::{
    ShortId,
    universal_weave::{Weave, layout::Spacing},
    weave::layout::TapestryLayouter,
};

use crate::{
    common::view::View,
    editor::{
        EditorShared,
        shared::{
            scene::{Axes, SceneCamera, SceneItem, SceneState},
            ui::{DocumentContextFlags, TextFlags, TokenTooltipFlags},
        },
    },
};

#[derive(Debug)]
pub struct GraphView {
    scene: SceneState,
    camera: SceneCamera,
    context_node: Option<ShortId>,
}

impl Default for GraphView {
    fn default() -> Self {
        Self {
            scene: SceneState::new(
                Axes::Identity,
                TapestryLayouter::new(Spacing {
                    node: 1.0,
                    layer: 2.0,
                    corridor: 0.0,
                    edge: 0.25,
                }),
            ),
            camera: SceneCamera::new(f32::EPSILON..=240.0, 15.0),
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
            self.scene.invalidate();
        }
    }
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {
        let Some(weave) = &mut shared.weave else {
            return;
        };

        let shift = self
            .scene
            .arrange(weave, shared.ui.cursor, |_| Vec2::splat(1.0));

        let weave_view =
            Rect::from_min_size(Pos2::ZERO, self.scene.size()).expand2(self.scene.size() * 0.025);

        let InnerResponse {
            response,
            inner: painter,
        } = self.camera.show(
            ui,
            &mut shared.ui,
            shift,
            weave_view,
            &mut self.scene,
            |scene, id: &ShortId| {
                scene
                    .node_center(id)
                    .map(|center| Rect::from_center_size(center, Vec2::splat(1.0)))
            },
            |_, ui, _| ui.painter().clone(),
        );

        let zoom = painter
            .ctx()
            .layer_transform_to_global(painter.layer_id())
            .map_or(1.0, |transform| transform.scaling);

        let default_color = ui.visuals().widgets.inactive.text_color();
        let icon_color = ui.visuals().panel_fill;

        let hover_stroke = Stroke::new(
            2.75 / zoom,
            ui.visuals().widgets.noninteractive.fg_stroke.color,
        );
        let cursor_stroke = Stroke::new(hover_stroke.width, ui.visuals().widgets.inactive.bg_fill);

        let pointer = response.hover_pos().filter(|_| !response.dragged());

        let mut pointer_node = None;

        let width = 2.0 / zoom;
        let active_stroke = Stroke::new(width, ui.visuals().widgets.noninteractive.fg_stroke.color);
        let inactive_stroke = Stroke::new(width, ui.visuals().widgets.inactive.bg_fill);
        let tolerance = 0.25
            / painter
                .ctx()
                .layer_transform_to_global(painter.layer_id())
                .map_or(1.0, |transform| transform.scaling);

        for item in self
            .scene
            .view(weave, painter.clip_rect().expand(hover_stroke.width))
        {
            match item {
                SceneItem::Edge {
                    from: _,
                    to: _,
                    active,
                    segments,
                } => {
                    painter.extend(segments.into_iter().flat_map(|segment| {
                        CubicBezierShape::from_points_stroke(
                            segment,
                            false,
                            Color32::TRANSPARENT,
                            if active {
                                active_stroke
                            } else {
                                inactive_stroke
                            },
                        )
                        .to_path_shapes(Some(tolerance), None)
                        .into_iter()
                        .map(Shape::Path)
                    }));
                }
                SceneItem::Node { id, rect: bounds } => {
                    let Some(node) = weave.get(&id) else {
                        return;
                    };

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
                                    Vec2 { x: 0.25, y: 0.15 },
                                    Vec2 { x: 0.75, y: 0.15 },
                                    Vec2 { x: 0.75, y: 0.85 },
                                    Vec2 { x: 0.5, y: 0.7 },
                                    Vec2 { x: 0.25, y: 0.85 },
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
            }
        }

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
                shared.ui.activate_node(weave, id, &response, true);
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
                            None,
                            TokenTooltipFlags::WarnModified.into(),
                        );
                    }
                });
                shared.ui.set_hovered(id);
            }
        } else if response.double_clicked() {
            self.camera.reset(ui, weave_view);
        }
    }
}
