use std::{
    collections::{HashMap, HashSet},
    hash::BuildHasherDefault,
};

use eframe::egui::{
    Context, Pos2, Rect, TextStyle, Ui, Vec2, WidgetText,
    text::TextWrapping,
    widget_style::{Classes, WidgetState},
};
use tapestry_weave::{
    ShortId, TapestryNode,
    universal_weave::{Weave, layout::Spacing},
    util::RandomIdHasher,
    weave::layout::TapestryLayouter,
};

use crate::{
    common::view::View,
    editor::{
        EditorShared,
        shared::{
            scene::{Axes, EdgePainter, SceneCamera, SceneItem, SceneState},
            ui::{DocumentContextFlags, TextFlags, WeaveUi},
        },
    },
};

#[derive(Default, Debug, Clone, Copy, PartialEq)]
struct Metrics {
    pad: f32,
    width: f32,
    margin: Vec2,
}

impl Metrics {
    fn from_ui(ui: &Ui) -> Self {
        Self {
            pad: ui.text_style_height(&TextStyle::Monospace),
            width: ui.spacing().text_edit_width * 1.2,
            margin: ui
                .style()
                .button_style(&Classes::default(), WidgetState::Inactive)
                .frame
                .inner_margin
                .sum(),
        }
    }
    fn spacing(&self) -> Spacing {
        Spacing {
            node: self.pad,
            layer: 7.0 * self.pad,
            corridor: self.pad,
            edge: self.pad * 0.5,
        }
    }
    fn height(&self, ui: &Ui, weave_ui: &mut WeaveUi, node: &TapestryNode) -> f32 {
        let mut job = weave_ui.node_text(ui, node, TextFlags::EmptyNotice.into());
        job.wrap = TextWrapping::wrap_at_width(self.width - self.margin.x);

        (ui.painter().layout_job(job).size().y + self.margin.y).max(self.pad * 3.0)
    }
}

#[derive(Debug)]
pub struct CanvasView {
    scene: SceneState,
    camera: SceneCamera,
    sizes: HashMap<ShortId, (u64, f32), BuildHasherDefault<RandomIdHasher>>,
    visible: HashSet<ShortId, BuildHasherDefault<RandomIdHasher>>,
    visible_stale: bool,
    metrics: Metrics,
}

impl Default for CanvasView {
    fn default() -> Self {
        Self {
            scene: SceneState::new(Axes::Transposed, TapestryLayouter::default()),
            camera: SceneCamera::new(f32::EPSILON..=1.0, 0.9),
            sizes: HashMap::default(),
            visible: HashSet::default(),
            visible_stale: true,
            metrics: Metrics::default(),
        }
    }
}

impl View<EditorShared> for CanvasView {
    fn title(&self, _shared: &EditorShared) -> WidgetText {
        WidgetText::Text("\u{E125} Canvas".to_string())
    }
    fn logic(&mut self, shared: &mut EditorShared, _force_close: impl FnOnce(), _ctx: &Context) {
        if shared.weave_changed {
            self.scene.invalidate();
        }

        self.visible_stale |= shared.weave_changed || shared.ui.opened_changed();
    }
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {
        let Some(weave) = &mut shared.weave else {
            return;
        };

        let metrics = Metrics::from_ui(ui);

        if metrics != self.metrics {
            self.metrics = metrics;

            self.sizes.clear();
            *self.scene.spacing_mut() = metrics.spacing();
            self.scene.invalidate();
        }

        if !self.scene.is_arranged() {
            self.sizes.retain(|id, _| weave.contains(id));
        }

        let shift = self.scene.arrange(weave, shared.ui.cursor, |node| {
            let fingerprint = node.contents.content.fingerprint();

            Vec2 {
                x: metrics.width,
                y: match self.sizes.get(&node.id) {
                    Some((cached, height)) if cached == &fingerprint => *height,
                    _ => {
                        let height = metrics.height(ui, &mut shared.ui, node);
                        self.sizes.insert(node.id, (fingerprint, height));
                        height
                    }
                },
            }
        });

        if self.visible_stale {
            self.visible.clear();

            let mut stack: Vec<ShortId> = weave.roots().iter().copied().collect();

            while let Some(id) = stack.pop() {
                if self.visible.insert(id)
                    && shared.ui.is_open(&id)
                    && let Some(node) = weave.get(&id)
                {
                    stack.extend(node.to.iter().copied());
                }
            }

            self.visible_stale = false;
        }

        let mut weave_view =
            Rect::from_min_size(Pos2::ZERO, self.scene.size()).expand(self.metrics.pad);
        weave_view.max.x += 5.0 * self.metrics.pad;

        let response = self
            .camera
            .show(
                ui,
                &mut shared.ui,
                shift,
                weave_view,
                |id| {
                    if !self.visible.contains(id) {
                        return None;
                    }

                    Some(Rect::from_center_size(
                        self.scene.node_center(id)?,
                        Vec2 {
                            x: self.metrics.width,
                            y: self.sizes.get(id)?.1,
                        },
                    ))
                },
                |ui, weave_ui| {
                    let mut clip = ui.clip_rect().expand(self.metrics.pad);
                    clip.min.x -= 5.0 * self.metrics.pad;

                    let painter = ui.painter().clone();

                    let mut edges = EdgePainter::new(
                        &painter,
                        ui.visuals(),
                        ui.visuals().widgets.inactive.fg_stroke.width,
                    );

                    self.scene.view(clip, |item| match item {
                        SceneItem::Edge { from, to, segments } => {
                            if self.visible.contains(&from) && weave_ui.is_open(&from) {
                                edges.edge(weave, &from, &to, segments);
                            }
                        }
                        SceneItem::Node { id, rect } => {
                            if self.visible.contains(&id)
                                && let Some(node) = weave.get(&id).cloned()
                            {
                                weave_ui.canvas_node(weave, &node, ui, rect);
                            }
                        }
                    });

                    edges.finish();
                },
            )
            .response;

        response.context_menu(|ui| {
            shared
                .ui
                .document_context_menu(weave, ui, DocumentContextFlags::Roots.into());
        });

        if response.double_clicked() {
            self.camera.reset(ui, weave_view);
        }
    }
}
