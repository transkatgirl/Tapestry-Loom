use eframe::egui::{Context, ScrollArea, Ui, WidgetText};
use tapestry_weave::v1::dependent::{TapestryNode, TapestryWeave};

use crate::{
    common::{
        ui::{label_separator, listing},
        view::View,
    },
    editor::{
        EditorShared,
        shared::ui::{AutoscrollData, ButtonFlags, LabelOptions, WeaveUi},
    },
};

#[derive(Default, Debug)]
pub struct TreeListView {}

impl TreeListView {}

impl View<EditorShared> for TreeListView {
    fn title(&self, _shared: &EditorShared) -> WidgetText {
        WidgetText::Text("\u{E408} Tree".to_string())
    }
    fn logic(&mut self, shared: &mut EditorShared, _force_close: impl FnOnce(), ctx: &Context) {}
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {
        if let Some(weave) = &mut shared.weave {
            // TODO
        }
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
            label_separator(ui, 0.3);
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
            label_separator(ui, 0.3);
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
