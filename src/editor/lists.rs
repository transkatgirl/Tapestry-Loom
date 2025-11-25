use std::collections::HashSet;

use eframe::egui::{
    Align, Button, Color32, FontFamily, Layout, RichText, ScrollArea, Sense, Ui, UiBuilder,
    collapsing_header::CollapsingState,
};
use egui_notify::Toasts;
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
        _settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
    ) {
        let items = weave
            .get_active_thread()
            .next()
            .map(|node| node.to.iter().cloned().map(Ulid).collect::<Vec<Ulid>>());
        if let Some(items) = items {
            let row_height = ui.spacing().interact_size.y;
            ScrollArea::vertical()
                .auto_shrink(false)
                .animated(false)
                .show_rows(ui, row_height, items.len(), |ui, range| {
                    for item in &items[range] {
                        self.render_item(weave, ui, item);
                    }
                });
        }
    }
    fn render_item(&mut self, weave: &mut TapestryWeave, ui: &mut Ui, item: &Ulid) {
        // TODO: Add inference

        if let Some(node) = weave.get_node(item).cloned() {
            ui.horizontal(|ui| {
                ui.add_space(ui.spacing().icon_spacing);

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
                                ui.scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
                                    ui.add_space(ui.spacing().icon_spacing);
                                    if ui.button("\u{E28F}").clicked() {
                                        weave.remove_node(&Ulid(node.id));
                                    };
                                    let bookmark_label = if node.bookmarked {
                                        "\u{E23C}"
                                    } else {
                                        "\u{E23d}"
                                    };
                                    if ui.button(bookmark_label).clicked() {
                                        weave.set_node_bookmarked_status(
                                            &Ulid(node.id),
                                            !node.bookmarked,
                                        );
                                    };
                                    if ui.button("\u{E40C}").clicked() {
                                        weave.add_node(DependentNode {
                                            id: Ulid::new().0,
                                            from: Some(node.id),
                                            to: IndexSet::default(),
                                            active: false,
                                            bookmarked: false,
                                            contents: NodeContent {
                                                content: InnerNodeContent::Snippet(vec![]),
                                                metadata: IndexMap::new(),
                                                model: None,
                                            },
                                        });
                                    };
                                    if ui.button("\u{E5CE}").clicked() {
                                        todo!()
                                    };
                                    if weave.is_mergeable_with_parent(&Ulid(node.id))
                                        && ui.button("\u{E43F}").clicked()
                                    {
                                        weave.merge_with_parent(&Ulid(node.id));
                                    };
                                    ui.add_space(ui.spacing().icon_spacing);
                                });
                                ui.add_space(0.0);
                            });
                        } else if node.bookmarked {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.add_space(ui.spacing().icon_spacing);
                                ui.label("\u{E060}");
                                ui.add_space(ui.spacing().icon_spacing);
                            });
                        }
                    })
                    .response;

                if response.clicked() {
                    weave.set_node_active_status(&Ulid(node.id), !node.active);
                }
            });
        }
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
        _settings: &Settings,
        _toasts: &mut Toasts,
        _state: &mut SharedState,
    ) {
        let items: Vec<Ulid> = weave.get_bookmarks().collect();
        let row_height = ui.spacing().interact_size.y;
        ScrollArea::vertical()
            .auto_shrink(false)
            .animated(false)
            .show_rows(ui, row_height, items.len(), |ui, range| {
                for item in &items[range] {
                    self.render_bookmark(weave, ui, item);
                }
            });
    }
    fn render_bookmark(&mut self, weave: &mut TapestryWeave, ui: &mut Ui, item: &Ulid) {
        if let Some(node) = weave.get_node(item).cloned() {
            ui.horizontal(|ui| {
                ui.add_space(ui.spacing().icon_spacing);
                ui.label("\u{E060}");

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
                                ui.scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
                                    ui.add_space(ui.spacing().icon_spacing);
                                    if ui.button("\u{E23C}").clicked() {
                                        weave.set_node_bookmarked_status(&Ulid(node.id), false);
                                    };
                                    ui.add_space(ui.spacing().icon_spacing);
                                });
                                ui.add_space(0.0);
                            });
                        }
                    })
                    .response;

                if response.clicked() {
                    weave.set_node_active_status(&Ulid(node.id), !node.active);
                }
            });
        }
    }
}

#[derive(Default, Debug)]
pub struct TreeListView {
    hoist: Option<Ulid>,
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
        ScrollArea::vertical()
            .auto_shrink(false)
            .animated(false)
            .show(ui, |ui| {
                self.render_node_tree(
                    weave,
                    ui,
                    state.identifier,
                    roots,
                    &active_set,
                    settings.interface.max_tree_depth,
                );

                /*if ui.button("test").clicked() {
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
                }*/
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
        // TODO: Test node activation handling, add inference

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
                                                if ui.button("\u{E28F}").clicked() {
                                                    weave.remove_node(&Ulid(node.id));
                                                };
                                                let bookmark_label = if node.bookmarked {
                                                    "\u{E23C}"
                                                } else {
                                                    "\u{E23d}"
                                                };
                                                if ui.button(bookmark_label).clicked() {
                                                    weave.set_node_bookmarked_status(
                                                        &Ulid(node.id),
                                                        !node.bookmarked,
                                                    );
                                                };
                                                if ui.button("\u{E40C}").clicked() {
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
                                                if ui.button("\u{E5CE}").clicked() {
                                                    todo!()
                                                };
                                                if weave.is_mergeable_with_parent(&Ulid(node.id))
                                                    && ui.button("\u{E43F}").clicked()
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
                                        ui.label("\u{E060}");
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
