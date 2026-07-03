use eframe::egui::{Context, ScrollArea, Ui, WidgetText};

use crate::{
    common::{
        ui::{label_separator, listing},
        view::View,
    },
    editor::{
        EditorShared,
        shared::ui::{ButtonFlags, LabelOptions},
    },
};

#[derive(Default, Debug)]
pub struct TreeListView {}

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
                                if index != 0 {
                                    label_separator(ui, 0.3);
                                }

                                ui.horizontal_wrapped(|ui| {
                                    ui.add_space(ui.spacing().icon_spacing);
                                    shared.ui.horizontal_node_label(
                                        weave,
                                        &node,
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
                    }
                })
            });
    }
}

#[derive(Default, Debug)]
pub struct BookmarkView {}

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
                                if index != 0 {
                                    label_separator(ui, 0.3);
                                }

                                ui.horizontal_wrapped(|ui| {
                                    ui.add_space(ui.spacing().icon_spacing);
                                    ui.label("\u{E060}");

                                    shared.ui.horizontal_node_label(
                                        weave,
                                        &node,
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
                    }
                })
            });
    }
}
