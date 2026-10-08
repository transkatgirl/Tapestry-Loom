use std::mem;

use eframe::egui::{
    Color32, InnerResponse, Painter, Pos2, Rangef, Rect, Scene, Shape, Stroke, Ui, Vec2, Visuals,
    epaint::CubicBezierShape, layers::ShapeIdx,
};
use tapestry_weave::{
    ShortId, TapestryNode,
    universal_weave::{
        LayoutItem, Layouter, Weave, glam,
        layout::{Spacing, smooth},
        tinyvec::ArrayVec,
    },
    weave::{layout::TapestryLayouter, wrappers::LoggedTapestryWeave},
};

use crate::editor::shared::ui::{LayoutFit, WeaveUi};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axes {
    Identity,
    Transposed,
}

impl Axes {
    pub fn to_screen(self, vector: glam::Vec2) -> Vec2 {
        match self {
            Self::Identity => Vec2 {
                x: vector.x,
                y: vector.y,
            },
            Self::Transposed => Vec2 {
                x: vector.y,
                y: vector.x,
            },
        }
    }
    pub fn to_layout(self, vector: Vec2) -> glam::Vec2 {
        match self {
            Self::Identity => glam::Vec2 {
                x: vector.x,
                y: vector.y,
            },
            Self::Transposed => glam::Vec2 {
                x: vector.y,
                y: vector.x,
            },
        }
    }
}

#[derive(Debug)]
pub enum SceneItem {
    Edge {
        from: ShortId,
        to: ShortId,
        segments: ArrayVec<[[Pos2; 4]; 5]>,
    },
    Node {
        id: ShortId,
        rect: Rect,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shift {
    pub from: Pos2,
    pub to: Pos2,
}

#[derive(Debug)]
pub struct SceneState {
    layouter: TapestryLayouter,
    axes: Axes,
    arranged: bool,
}

impl SceneState {
    pub fn new(axes: Axes, layouter: TapestryLayouter) -> Self {
        Self {
            layouter,
            axes,
            arranged: false,
        }
    }
    pub fn invalidate(&mut self) {
        self.arranged = false;
    }
    pub fn spacing_mut(&mut self) -> &mut Spacing {
        self.layouter.spacing_mut()
    }
    pub fn size(&self) -> Vec2 {
        self.axes.to_screen(self.layouter.size())
    }
    pub fn node_center(&self, id: &ShortId) -> Option<Pos2> {
        self.layouter
            .center(id)
            .map(|center| self.axes.to_screen(center).to_pos2())
    }
    pub fn arrange(
        &mut self,
        weave: &mut LoggedTapestryWeave,
        cursor: Option<ShortId>,
        mut sizes: impl FnMut(&TapestryNode) -> Vec2,
    ) -> Option<Shift> {
        if mem::replace(&mut self.arranged, true) {
            return None;
        }

        let from = cursor.and_then(|cursor| self.node_center(&cursor));

        self.layouter
            .layout(weave, |node| self.axes.to_layout(sizes(node)));

        let to = cursor.and_then(|cursor| self.node_center(&cursor));

        Some(Shift {
            from: from?,
            to: to?,
        })
    }
    pub fn view(&self, clip: Rect, mut f: impl FnMut(SceneItem)) {
        let axes = self.axes;

        self.layouter.view(
            axes.to_layout(clip.min.to_vec2()),
            axes.to_layout(clip.max.to_vec2()),
            |item| {
                f(match item {
                    LayoutItem::Polyline { from, to, points } => SceneItem::Edge {
                        from,
                        to,
                        segments: smooth(points)
                            .into_iter()
                            .map(|segment| segment.map(|point| axes.to_screen(point).to_pos2()))
                            .collect(),
                    },
                    LayoutItem::Node { id, center, size } => SceneItem::Node {
                        id,
                        rect: Rect::from_center_size(
                            axes.to_screen(center).to_pos2(),
                            axes.to_screen(size),
                        ),
                    },
                });
            },
        );
    }
}

#[derive(Debug)]
pub struct SceneCamera {
    view: Option<Rect>,
    outer: Vec2,
    zoom_range: Rangef,
    focus_zoom: f32,
}

impl SceneCamera {
    pub fn new(zoom_range: impl Into<Rangef>, focus_zoom: f32) -> Self {
        let zoom_range = zoom_range.into();

        debug_assert!(zoom_range.contains(focus_zoom));

        Self {
            view: None,
            outer: Vec2::ZERO,
            zoom_range,
            focus_zoom,
        }
    }
    pub fn show<R>(
        &mut self,
        ui: &mut Ui,
        weave_ui: &mut WeaveUi,
        shift: Option<Shift>,
        weave_view: Rect,
        node_bounds: impl Fn(&ShortId) -> Option<Rect>,
        contents: impl FnOnce(&mut Ui, &mut WeaveUi) -> R,
    ) -> InnerResponse<R> {
        self.outer = ui.available_size_before_wrap();

        let mut view = self.resolve(
            self.outer,
            weave_ui,
            shift,
            weave_view,
            node_bounds,
            ui.rect_contains_pointer(ui.clip_rect()),
        );

        let response = Scene::new()
            .zoom_range(self.zoom_range)
            .show(ui, &mut view, |ui| contents(ui, weave_ui));

        self.view = Some(view);

        response
    }
    fn resolve(
        &self,
        outer: Vec2,
        weave_ui: &WeaveUi,
        shift: Option<Shift>,
        weave_view: Rect,
        node_bounds: impl Fn(&ShortId) -> Option<Rect>,
        contains_pointer: bool,
    ) -> Rect {
        let focus = |outer: Vec2, bounds: Rect| {
            Rect::from_center_size(bounds.center(), bounds.size().max(outer / self.focus_zoom))
        };
        let visible = |outer: Vec2, view: Rect| {
            Rect::from_center_size(
                view.center(),
                outer / self.zoom_range.clamp((outer / view.size()).min_elem()),
            )
        };

        if weave_ui.fit == LayoutFit::Weave {
            return focus(outer, weave_view);
        }

        if (weave_ui.fit == LayoutFit::Cursor || self.view.is_none())
            && let Some(bounds) = weave_ui.cursor.and_then(|cursor| node_bounds(&cursor))
        {
            return focus(outer, bounds);
        }

        let Some(mut view) = self.view else {
            return focus(outer, weave_view);
        };

        if let Some(shift) = shift
            && visible(outer, view).contains(shift.from)
        {
            view = view.translate(shift.to - shift.from);
        }

        if let Some(target) = weave_ui.autoscroll_target()
            && let Some(bounds) = node_bounds(&target)
            && (!contains_pointer
                || (weave_ui.cursor == Some(target) && !visible(outer, view).contains_rect(bounds)))
        {
            view = focus(outer, bounds);
        }

        view
    }
    pub fn reset(&mut self, ui: &Ui, bounds: Rect) {
        self.view = Some(Rect::from_center_size(
            bounds.center(),
            bounds.size().max(self.outer / self.focus_zoom),
        ));
        ui.ctx().request_repaint();
    }
}

pub struct EdgePainter<'a> {
    painter: &'a Painter,
    inactive_idx: ShapeIdx,
    inactive: Vec<Shape>,
    active_stroke: Stroke,
    inactive_stroke: Stroke,
    tolerance: f32,
}

impl<'a> EdgePainter<'a> {
    pub fn new(painter: &'a Painter, visuals: &Visuals, width: f32) -> Self {
        Self {
            painter,
            inactive_idx: painter.add(Shape::Noop),
            inactive: Vec::new(),
            active_stroke: Stroke::new(width, visuals.widgets.noninteractive.fg_stroke.color),
            inactive_stroke: Stroke::new(width, visuals.widgets.inactive.bg_fill),
            tolerance: 0.25
                / painter
                    .ctx()
                    .layer_transform_to_global(painter.layer_id())
                    .map_or(1.0, |transform| transform.scaling),
        }
    }
    pub fn edge(
        &mut self,
        weave: &LoggedTapestryWeave,
        from: &ShortId,
        to: &ShortId,
        segments: impl IntoIterator<Item = [Pos2; 4]>,
    ) {
        let active = weave.contains_active(from) && weave.contains_active(to);

        let shapes = segments.into_iter().flat_map(|segment| {
            CubicBezierShape::from_points_stroke(
                segment,
                false,
                Color32::TRANSPARENT,
                if active {
                    self.active_stroke
                } else {
                    self.inactive_stroke
                },
            )
            .to_path_shapes(Some(self.tolerance), None)
            .into_iter()
            .map(Shape::Path)
        });

        if active {
            self.painter.extend(shapes);
        } else {
            self.inactive.extend(shapes);
        }
    }
    pub fn finish(self) {
        self.painter
            .set(self.inactive_idx, Shape::Vec(self.inactive));
    }
}
