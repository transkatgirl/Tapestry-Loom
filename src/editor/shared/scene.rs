use std::mem;

use eframe::egui::{InnerResponse, Pos2, Rangef, Rect, Scene, Ui, Vec2};
use tapestry_weave::{
    ShortId, TapestryNode,
    universal_weave::{
        LayoutItem, Layouter, glam,
        layout::{Spacing, smooth},
        tinyvec::ArrayVec,
    },
    weave::{layout::TapestryLayouter, wrappers::LoggedTapestryWeave},
};

use crate::{
    common::ui::view_contains_pointer,
    editor::shared::ui::{LayoutFit, WeaveUi},
};

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
        active: bool,
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
    pub fn is_arranged(&self) -> bool {
        self.arranged
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
    pub fn view(&mut self, weave: &mut LoggedTapestryWeave, clip: Rect) -> Vec<SceneItem> {
        let mut items = Vec::new();

        self.layouter.view(
            weave,
            self.axes.to_layout(clip.min.to_vec2()),
            self.axes.to_layout(clip.max.to_vec2()),
            |item| {
                items.push(match item {
                    LayoutItem::Polyline {
                        from,
                        to,
                        active,
                        points,
                    } => SceneItem::Edge {
                        from,
                        to,
                        active,
                        segments: smooth(points)
                            .into_iter()
                            .map(|segment| {
                                segment.map(|point| self.axes.to_screen(point).to_pos2())
                            })
                            .collect(),
                    },
                    LayoutItem::Node { id, center, size } => SceneItem::Node {
                        id,
                        rect: Rect::from_center_size(
                            self.axes.to_screen(center).to_pos2(),
                            self.axes.to_screen(size),
                        ),
                    },
                });
            },
        );

        items
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
    #[allow(clippy::too_many_arguments, reason = "TODO")]
    pub fn show<T, R>(
        &mut self,
        ui: &mut Ui,
        weave_ui: &mut WeaveUi,
        shift: Option<Shift>,
        weave_view: Rect,
        passthrough: &mut T,
        node_bounds: impl Fn(&mut T, &ShortId) -> Option<Rect>,
        contents: impl FnOnce(&mut T, &mut Ui, &mut WeaveUi) -> R,
    ) -> InnerResponse<R> {
        self.outer = ui.available_size_before_wrap();

        let mut view = self.resolve(
            self.outer,
            weave_ui,
            shift,
            weave_view,
            |id| node_bounds(passthrough, id),
            view_contains_pointer(ui),
        );

        let response = Scene::new()
            .zoom_range(self.zoom_range)
            .show(ui, &mut view, |ui| contents(passthrough, ui, weave_ui));

        self.view = Some(view);

        response
    }
    fn resolve(
        &self,
        outer: Vec2,
        weave_ui: &WeaveUi,
        shift: Option<Shift>,
        weave_view: Rect,
        mut node_bounds: impl FnMut(&ShortId) -> Option<Rect>,
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
