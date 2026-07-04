use eframe::egui::{Context, Id, ScrollArea, Ui, WidgetText, collapsing_header::CollapsingState};
use tapestry_weave::v1::dependent::{TapestryNode, TapestryWeave};
use ulid::Ulid;

use crate::{
    common::{
        ui::{label_separator, listing},
        view::View,
    },
    editor::{
        EditorShared,
        shared::ui::{AutoscrollData, ButtonFlags, DEFAULT_OPEN, LabelOptions, WeaveUi},
    },
};

pub const LABEL_SEPARATOR_OPACITY: f32 = 0.35;

#[derive(Default, Debug)]
pub struct TreeListView {}

impl TreeListView {
    #[allow(clippy::too_many_arguments)]
    fn render_row(
        &mut self,
        weave: &mut TapestryWeave,
        nodes: impl Iterator<Item = u64>,
        ui: &mut Ui,
        shared: &mut WeaveUi,
        autoscroll: Option<AutoscrollData>,
        editor_id: Ulid,
        indent_level: usize,
    ) {
        for (index, node) in nodes.enumerate() {
            if let Some(node) = weave.get_node(&node).cloned() {
                if indent_level != 0 || index != 0 {
                    label_separator(ui, LABEL_SEPARATOR_OPACITY);
                }

                let id = Id::new((editor_id.0, node.id));
                let mut collapsing =
                    CollapsingState::load_with_default_open(ui.ctx(), id, DEFAULT_OPEN);
                if let Some(opened) = shared.opened.get(&node.id).copied() {
                    collapsing.set_open(opened);
                }

                let mut render_label = |ui: &mut Ui| {
                    ui.horizontal_wrapped(|ui| {
                        if node.to.is_empty() {
                            ui.add_space(ui.spacing().icon_width + ui.spacing().icon_spacing);
                        }

                        shared.horizontal_node_label(
                            weave,
                            &node,
                            ui,
                            &LabelOptions {
                                buttons: if indent_level == 0 && index == 0 && node.from.is_some() {
                                    ButtonFlags::Hoist
                                        | ButtonFlags::Merge
                                        | ButtonFlags::Generate
                                        | ButtonFlags::Add
                                        | ButtonFlags::Bookmark
                                        | ButtonFlags::Delete
                                } else {
                                    ButtonFlags::Merge
                                        | ButtonFlags::Generate
                                        | ButtonFlags::Add
                                        | ButtonFlags::Bookmark
                                        | ButtonFlags::Delete
                                },
                                collapsing: false,
                                show_info: true,
                                autoscroll,
                            },
                            &None,
                        );
                    });
                };

                if node.to.is_empty() {
                    render_label(ui)
                } else {
                    let collapsing_response = collapsing
                        .show_header(ui, |ui| {
                            render_label(ui);
                        })
                        .body(|ui| {
                            if indent_level > 3 && ui.available_size_before_wrap().x < 150.0 {
                                // TODO
                            } else {
                                let nodes: Vec<u64> = node.to.iter().copied().collect();

                                self.render_row(
                                    weave,
                                    nodes.into_iter(),
                                    ui,
                                    shared,
                                    autoscroll,
                                    editor_id,
                                    indent_level + 1,
                                );
                            }
                        });

                    if collapsing_response.0.clicked() {
                        shared.opened.insert(
                            node.id,
                            CollapsingState::load_with_default_open(ui.ctx(), id, DEFAULT_OPEN)
                                .is_open(),
                        );
                    }
                }

                // TODO
            }
        }
    }
}

impl View<EditorShared> for TreeListView {
    fn title(&self, _shared: &EditorShared) -> WidgetText {
        WidgetText::Text("\u{E408} Tree".to_string())
    }
    fn logic(&mut self, shared: &mut EditorShared, _force_close: impl FnOnce(), ctx: &Context) {}
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {
        ScrollArea::vertical()
            .auto_shrink(false)
            .animated(false)
            .show(ui, |ui| {
                listing(ui, |ui| {
                    if let Some(weave) = &mut shared.weave {
                        let autoscroll = shared.ui.calculate_autoscroll(ui);

                        let roots: Vec<u64> = if let Some(cursor) = shared.ui.cursor
                            && let Some(cursor_node) = weave.get_node(&cursor)
                            && let Some(cursor_parent) = &cursor_node.from
                            && let Some(Some(cursor_parent_parent)) =
                                weave.get_node_parent(cursor_parent)
                        {
                            if !cursor_node.to.is_empty() {
                                vec![*cursor_parent]
                            } else {
                                vec![*cursor_parent_parent]
                            }
                        } else {
                            weave.roots().iter().copied().collect()
                        };

                        self.render_row(
                            weave,
                            roots.into_iter(),
                            ui,
                            &mut shared.ui,
                            autoscroll,
                            shared.id,
                            0,
                        );
                    }
                })
            });
    }
}

#[derive(Default, Debug)]
pub struct ListView {}

impl ListView {
    fn render_item(
        &mut self,
        weave: &mut TapestryWeave,
        node: &TapestryNode,
        ui: &mut Ui,
        shared: &mut WeaveUi,
        autoscroll: Option<AutoscrollData>,
        is_start: bool,
    ) {
        if !is_start {
            label_separator(ui, LABEL_SEPARATOR_OPACITY);
        }

        ui.horizontal_wrapped(|ui| {
            ui.add_space(ui.spacing().icon_spacing);
            shared.horizontal_node_label(
                weave,
                node,
                ui,
                &LabelOptions {
                    buttons: ButtonFlags::Merge
                        | ButtonFlags::Generate
                        | ButtonFlags::Add
                        | ButtonFlags::Bookmark
                        | ButtonFlags::Delete,
                    collapsing: false,
                    show_info: true,
                    autoscroll,
                },
                &None,
            );
        });
    }
}

impl View<EditorShared> for ListView {
    fn title(&self, _shared: &EditorShared) -> WidgetText {
        WidgetText::Text("\u{E106} List".to_string())
    }
    fn logic(&mut self, shared: &mut EditorShared, _force_close: impl FnOnce(), ctx: &Context) {}
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {
        ScrollArea::vertical()
            .auto_shrink(false)
            .animated(false)
            .show(ui, |ui| {
                listing(ui, |ui| {
                    if let Some(weave) = &mut shared.weave {
                        let autoscroll = shared.ui.calculate_autoscroll(ui);

                        let items: Vec<u64> = if let Some(cursor) = shared.ui.cursor {
                            weave
                                .get_node_children(&cursor)
                                .map(|c| c.iter().copied().collect())
                                .unwrap_or_default()
                        } else {
                            weave.roots().iter().copied().collect()
                        };

                        for (index, item) in items.into_iter().enumerate() {
                            if let Some(node) = weave.get_node(&item).cloned() {
                                self.render_item(
                                    weave,
                                    &node,
                                    ui,
                                    &mut shared.ui,
                                    autoscroll,
                                    index == 0,
                                );
                            }
                        }
                    }
                })
            });
    }
}

#[derive(Default, Debug)]
pub struct BookmarkView {}

impl BookmarkView {
    fn render_item(
        &mut self,
        weave: &mut TapestryWeave,
        node: &TapestryNode,
        ui: &mut Ui,
        shared: &mut WeaveUi,
        autoscroll: Option<AutoscrollData>,
        is_start: bool,
    ) {
        if !is_start {
            label_separator(ui, LABEL_SEPARATOR_OPACITY);
        }

        ui.horizontal_wrapped(|ui| {
            ui.add_space(ui.spacing().icon_spacing);
            ui.label("\u{E060}");

            shared.horizontal_node_label(
                weave,
                node,
                ui,
                &LabelOptions {
                    buttons: ButtonFlags::Bookmark.into(),
                    collapsing: false,
                    show_info: false,
                    autoscroll,
                },
                &None,
            );
        });
    }
}

impl View<EditorShared> for BookmarkView {
    fn title(&self, _shared: &EditorShared) -> WidgetText {
        WidgetText::Text("\u{E060} Bookmarks".to_string())
    }
    fn logic(&mut self, shared: &mut EditorShared, _force_close: impl FnOnce(), ctx: &Context) {}
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {
        ScrollArea::vertical()
            .auto_shrink(false)
            .animated(false)
            .show(ui, |ui| {
                listing(ui, |ui| {
                    if let Some(weave) = &mut shared.weave {
                        let autoscroll = shared.ui.calculate_autoscroll(ui);

                        let bookmarks: Vec<u64> = weave.bookmarks().iter().copied().collect();

                        for (index, bookmark) in bookmarks.into_iter().enumerate() {
                            if let Some(node) = weave.get_node(&bookmark).cloned() {
                                self.render_item(
                                    weave,
                                    &node,
                                    ui,
                                    &mut shared.ui,
                                    autoscroll,
                                    index == 0,
                                );
                            }
                        }
                    }
                })
            });
    }
}
