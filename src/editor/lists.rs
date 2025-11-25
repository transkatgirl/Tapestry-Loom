use std::{collections::HashSet, sync::Arc};

use eframe::egui::{
    self, Align, Button, Color32, FontFamily, Layout, Margin, RichText, ScrollArea, Sense, Ui,
    UiBuilder, Vec2, WidgetText, collapsing_header::CollapsingState,
};
use egui_notify::Toasts;
use egui_phosphor::{fill, regular};
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{
        dependent::DependentNode,
        indexmap::{IndexMap, IndexSet},
    },
    v0::{InnerNodeContent, NodeContent, TapestryWeave},
};

use crate::{editor::shared::SharedState, settings::Settings};

#[derive(Default, Debug)]
pub struct ListView {}

impl ListView {
    pub fn reset(&mut self) {}
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut TapestryWeave,
        settings: &Settings,
        toasts: &mut Toasts,
        state: &mut SharedState,
    ) {
    }
}

#[derive(Default, Debug)]
pub struct BookmarkListView {}

impl BookmarkListView {
    pub fn reset(&mut self) {}
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut TapestryWeave,
        settings: &Settings,
        toasts: &mut Toasts,
        state: &mut SharedState,
    ) {
        let bookmarks: Vec<Ulid> = weave.get_bookmarks().collect();
    }
}

#[derive(Debug)]
pub struct TreeListView {
    hoist: Option<Ulid>,
    bookmark_icon: Arc<RichText>,
    unbookmark_icon: Arc<RichText>,
}

impl Default for TreeListView {
    fn default() -> Self {
        Self {
            hoist: None,
            bookmark_icon: Arc::new(
                RichText::new(regular::BOOKMARK_SIMPLE.to_string())
                    .family(FontFamily::Name("phosphor".into())),
            ),
            unbookmark_icon: Arc::new(
                RichText::new(fill::BOOKMARK_SIMPLE.to_string())
                    .family(FontFamily::Name("phosphor-fill".into())),
            ),
        }
    }
}

impl TreeListView {
    pub fn reset(&mut self) {
        self.hoist = None;
    }
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut TapestryWeave,
        settings: &Settings,
        toasts: &mut Toasts,
        state: &mut SharedState,
    ) {
        // TODO: hoisting, hover tooltips, right click menu
        let roots: Vec<Ulid> = weave.get_roots().collect();
        let active: Vec<Ulid> = weave
            .get_active_thread()
            .map(|node| Ulid(node.id))
            .collect();
        let active_set = HashSet::from_iter(active.clone());
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            self.render_node_tree(
                weave,
                ui,
                state.identifier,
                roots,
                &active_set,
                settings.interface.max_tree_depth,
            );

            if ui.button("test").clicked() {
                weave.add_node(DependentNode {
                    id: Ulid::new().0,
                    from: None,
                    to: IndexSet::default(),
                    active: false,
                    bookmarked: false,
                    contents: NodeContent {
                        content: InnerNodeContent::Snippet(
                            Ulid::new().to_string().as_bytes().to_vec(),
                        ),
                        metadata: IndexMap::new(),
                        model: None,
                    },
                });
            }
        });
    }
    fn render_node_tree(
        &self,
        weave: &mut TapestryWeave,
        ui: &mut Ui,
        editor_id: Ulid,
        items: impl IntoIterator<Item = Ulid>,
        active_items: &HashSet<Ulid>,
        max_depth: usize,
    ) {
        if max_depth == 0 {
            return;
        }

        let indent_compensation = ui.spacing().icon_width + ui.spacing().icon_spacing;

        for item in items {
            if let Some(node) = weave.get_node(&item).cloned() {
                let mut render_label = |ui: &mut Ui| {
                    ui.horizontal(|ui| {
                        if node.to.is_empty() {
                            ui.add_space(indent_compensation);
                        }

                        let response = ui
                            .scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
                                let label = RichText::new(String::from_utf8_lossy(
                                    &node.contents.content.as_bytes().to_vec(),
                                ))
                                .family(FontFamily::Monospace);

                                let label_button = if node.active {
                                    Button::new(label).selected(true)
                                } else {
                                    Button::new(label).fill(Color32::TRANSPARENT)
                                };

                                if ui.add(label_button).clicked() {
                                    weave.set_node_active_status(&Ulid(node.id), !node.active);
                                }

                                if ui.rect_contains_pointer(ui.max_rect()) {
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.scope_builder(
                                            UiBuilder::new().sense(Sense::click()),
                                            |ui| {
                                                ui.add_space(ui.spacing().icon_spacing);
                                                if ui.button(regular::ERASER).clicked() {
                                                    weave.remove_node(&Ulid(node.id));
                                                };
                                                let bookmark_label =
                                                    WidgetText::RichText(if node.bookmarked {
                                                        self.unbookmark_icon.clone()
                                                    } else {
                                                        self.bookmark_icon.clone()
                                                    });
                                                if ui.button(bookmark_label).clicked() {
                                                    weave.set_node_bookmarked_status(
                                                        &Ulid(node.id),
                                                        !node.bookmarked,
                                                    );
                                                };
                                                if ui.button(regular::CHAT_TEXT).clicked() {
                                                    weave.add_node(DependentNode {
                                                        id: Ulid::new().0,
                                                        from: Some(node.id),
                                                        to: IndexSet::default(),
                                                        active: false,
                                                        bookmarked: false,
                                                        contents: NodeContent {
                                                            content: InnerNodeContent::Snippet(
                                                                vec![],
                                                            ),
                                                            metadata: IndexMap::new(),
                                                            model: None,
                                                        },
                                                    });
                                                };
                                                if ui.button(regular::SPARKLE).clicked() {
                                                    todo!()
                                                };
                                                if weave.is_mergeable_with_parent(&Ulid(node.id))
                                                    && ui.button(regular::GIT_MERGE).clicked()
                                                {
                                                    weave.merge_with_parent(&Ulid(node.id));
                                                };
                                                ui.add_space(ui.spacing().icon_spacing);
                                            },
                                        );
                                        ui.add_space(0.0);
                                    });
                                } else if node.bookmarked {
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.add_space(ui.spacing().icon_spacing);
                                        ui.label(regular::BOOKMARK_SIMPLE);
                                        ui.add_space(ui.spacing().icon_spacing);
                                    });
                                }
                            })
                            .response;

                        if response.clicked() {
                            weave.set_node_active_status(&Ulid(node.id), !node.active);
                        }
                    });
                };

                if node.to.is_empty() {
                    render_label(ui);
                } else {
                    let id = ui.make_persistent_id([editor_id.0, node.id, 0]);
                    CollapsingState::load_with_default_open(
                        ui.ctx(),
                        id,
                        active_items.contains(&Ulid(node.id)),
                    )
                    .show_header(ui, |ui| {
                        render_label(ui);
                    })
                    .body(|ui| {
                        self.render_node_tree(
                            weave,
                            ui,
                            editor_id,
                            node.to.into_iter().map(Ulid),
                            active_items,
                            max_depth - 1,
                        );
                    });
                }
            }
        }
    }
}
