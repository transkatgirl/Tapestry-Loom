#![allow(clippy::too_many_arguments)]

use std::collections::HashSet;

use eframe::egui::{
    Align, Button, Color32, FontFamily, Frame, Id, Layout, RichText, ScrollArea, Sense, Ui,
    UiBuilder, collapsing_header::CollapsingState,
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

use crate::{
    editor::shared::{
        SharedState, get_node_color, render_node_metadata_tooltip,
        should_render_node_metadata_tooltip,
    },
    listing_margin,
    settings::Settings,
};

// TODO: finish TreeListView

#[derive(Default, Debug)]
pub struct ListView {}

impl ListView {
    //pub fn reset(&mut self) {}
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut TapestryWeave,
        settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
    ) {
        let items = weave
            .get_active_thread()
            .next()
            .map(|node| node.to.iter().cloned().map(Ulid).collect::<Vec<Ulid>>())
            .unwrap_or_else(|| weave.get_roots().collect());

        let row_height = ui.spacing().interact_size.y;
        ScrollArea::vertical()
            .auto_shrink(false)
            .animated(false)
            .show_rows(ui, row_height, items.len(), |ui, range| {
                Frame::new()
                    .outer_margin(listing_margin(ui))
                    .show(ui, |ui| {
                        for item in &items[range] {
                            self.render_item(weave, settings, state, ui, item);
                        }
                    });
            });
    }
    fn render_item(
        &mut self,
        weave: &mut TapestryWeave,
        settings: &Settings,
        state: &mut SharedState,
        ui: &mut Ui,
        item: &Ulid,
    ) {
        if let Some(node) = weave.get_node(item).cloned() {
            ui.horizontal(|ui| {
                ui.add_space(ui.spacing().icon_spacing);
                render_horizontal_node_label(
                    ui,
                    settings,
                    state,
                    weave,
                    &node,
                    |ui, settings, state, weave, node| {
                        render_horizontal_node_label_buttons_rtl(ui, settings, state, weave, node);
                    },
                    |ui, settings, state, weave, node| {
                        render_node_context_menu(ui, settings, state, weave, node);
                    },
                    true,
                );
            });
        }
    }
}

#[derive(Default, Debug)]
pub struct BookmarkListView {}

impl BookmarkListView {
    //pub fn reset(&mut self) {}
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut TapestryWeave,
        settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
    ) {
        let items: Vec<Ulid> = weave.get_bookmarks().collect();
        let row_height = ui.spacing().interact_size.y;
        ScrollArea::vertical()
            .auto_shrink(false)
            .animated(false)
            .show_rows(ui, row_height, items.len(), |ui, range| {
                Frame::new()
                    .outer_margin(listing_margin(ui))
                    .show(ui, |ui| {
                        for item in &items[range] {
                            self.render_bookmark(weave, settings, state, ui, item);
                        }
                    });
            });
    }
    fn render_bookmark(
        &mut self,
        weave: &mut TapestryWeave,
        settings: &Settings,
        state: &mut SharedState,
        ui: &mut Ui,
        item: &Ulid,
    ) {
        if let Some(node) = weave.get_node(item).cloned() {
            ui.horizontal(|ui| {
                ui.add_space(ui.spacing().icon_spacing);
                ui.label("\u{E060}");

                render_horizontal_node_label(
                    ui,
                    settings,
                    state,
                    weave,
                    &node,
                    |ui, _settings, _state, weave, node| {
                        if ui
                            .button("\u{E23C}")
                            .on_hover_text("Remove bookmark")
                            .clicked()
                        {
                            weave.set_node_bookmarked_status(&Ulid(node.id), false);
                        };
                    },
                    |ui, settings, state, weave, node| {
                        render_node_context_menu(ui, settings, state, weave, node);
                    },
                    false,
                );
            });
        }
    }
}

#[derive(Default, Debug)]
pub struct TreeListView {
    last_seen_cursor_node: Option<Ulid>,
}

impl TreeListView {
    /*pub fn reset(&mut self) {
        self.last_seen_cursor_node = None;
    }*/
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut TapestryWeave,
        settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
    ) {
        // TODO: hoisting using cursor node
        let roots: Vec<Ulid> = weave.get_roots().collect();

        if self.last_seen_cursor_node != state.cursor_node {
            if let Some(cursor_node) = state.cursor_node {
                // FIXME
                // This is a hack which currently works because the cursor node is currently always equal to the active note, but will likely break in the future.
                let active: Vec<Ulid> = weave
                    .get_active_thread()
                    .map(|node| Ulid(node.id))
                    .collect();
                assert_eq!(cursor_node, active.first().cloned().unwrap());

                for item in active {
                    set_node_tree_item_open_status(ui, state.identifier, item, true);
                }
            }
            self.last_seen_cursor_node = state.cursor_node;
        }

        ScrollArea::vertical()
            .auto_shrink(false)
            .animated(false)
            .show(ui, |ui| {
                Frame::new()
                    .outer_margin(listing_margin(ui))
                    .show(ui, |ui| {
                        render_node_tree(
                            weave,
                            settings,
                            state,
                            ui,
                            state.identifier,
                            roots,
                            settings.interface.max_tree_depth,
                        );
                    });
            });
    }
}

fn render_node_tree(
    weave: &mut TapestryWeave,
    settings: &Settings,
    state: &mut SharedState,
    ui: &mut Ui,
    editor_id: Ulid,
    items: impl IntoIterator<Item = Ulid>,
    max_depth: usize,
) {
    let indent_compensation = ui.spacing().icon_width + ui.spacing().icon_spacing;

    for item in items {
        if let Some(node) = weave.get_node(&item).cloned() {
            let mut render_label = |ui: &mut Ui| {
                ui.horizontal(|ui| {
                    if node.to.is_empty() {
                        ui.add_space(indent_compensation);
                    }
                    render_horizontal_node_label(
                        ui,
                        settings,
                        state,
                        weave,
                        &node,
                        |ui, settings, state, weave, node| {
                            render_horizontal_node_label_buttons_rtl(
                                ui, settings, state, weave, node,
                            );
                        },
                        |ui, settings, state, weave, node| {
                            render_node_tree_context_menu(
                                ui, settings, state, editor_id, weave, node,
                            );
                        },
                        true,
                    );
                });
            };

            if node.to.is_empty() {
                render_label(ui);
            } else {
                let id = Id::new([editor_id.0, node.id, 0]);
                CollapsingState::load_with_default_open(ui.ctx(), id, false)
                    .show_header(ui, |ui| {
                        render_label(ui);
                    })
                    .body(|ui| {
                        if max_depth > 0 {
                            render_node_tree(
                                weave,
                                settings,
                                state,
                                ui,
                                editor_id,
                                node.to.into_iter().map(Ulid),
                                max_depth - 1,
                            );
                        } else {
                            ui.horizontal(|ui| {
                                let first_child = node.to.first().copied().map(Ulid).unwrap();
                                render_omitted_chidren_tree_node_label(
                                    ui,
                                    state,
                                    weave,
                                    &node,
                                    first_child,
                                );
                            });
                        }
                    });
            }
        }
    }
}

fn set_node_tree_item_open_status(ui: &mut Ui, editor_id: Ulid, item_id: Ulid, status: bool) {
    let id = Id::new([editor_id.0, item_id.0, 0]);
    let mut collapsing_state = CollapsingState::load_with_default_open(ui.ctx(), id, false);
    collapsing_state.set_open(status);
    collapsing_state.store(ui.ctx());
}

fn render_horizontal_node_label_buttons_rtl(
    ui: &mut Ui,
    settings: &Settings,
    state: &mut SharedState,
    weave: &mut TapestryWeave,
    node: &DependentNode<NodeContent>,
) {
    if ui.button("\u{E28F}").on_hover_text("Delete node").clicked() {
        weave.remove_node(&Ulid(node.id));
    };
    let bookmark_label = if node.bookmarked {
        "\u{E23C}"
    } else {
        "\u{E23d}"
    };
    let bookmark_hover_text = if node.bookmarked {
        "Remove bookmark"
    } else {
        "Bookmark node"
    };
    if ui
        .button(bookmark_label)
        .on_hover_text(bookmark_hover_text)
        .clicked()
    {
        weave.set_node_bookmarked_status(&Ulid(node.id), !node.bookmarked);
    };
    if ui.button("\u{E40C}").on_hover_text("Add node").clicked() {
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
    if ui
        .button("\u{E5CE}")
        .on_hover_text("Generate completions")
        .clicked()
    {
        state.generate_children(weave, Some(Ulid(node.id)), settings);
    };
    if weave.is_mergeable_with_parent(&Ulid(node.id))
        && ui
            .button("\u{E43F}")
            .on_hover_text("Merge node with parent")
            .clicked()
    {
        weave.merge_with_parent(&Ulid(node.id));
    };
}

fn render_omitted_chidren_tree_node_label(
    ui: &mut Ui,
    state: &mut SharedState,
    weave: &mut TapestryWeave,
    node: &DependentNode<NodeContent>,
    first_child: Ulid,
) {
    let response = ui
        .scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
            let mut frame = Frame::new();

            let is_hovered = state.last_hovered_node == Some(first_child);

            if is_hovered {
                frame = frame.fill(ui.style().visuals.widgets.hovered.weak_bg_fill);
            }

            frame.show(ui, |ui| {
                let mut label =
                    RichText::new("\u{E04A} Show more").family(FontFamily::Proportional);

                if is_hovered {
                    label = label.color(ui.style().visuals.widgets.hovered.text_color());
                }

                let label_button_response =
                    ui.add(Button::new(label).frame(false).fill(Color32::TRANSPARENT));

                if label_button_response.hovered() {
                    state.hovered_node = Some(first_child);
                }

                if label_button_response.clicked() && !node.active {
                    weave.set_node_active_status(&Ulid(node.id), true);
                    state.cursor_node = Some(Ulid(node.id));
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(0.0);
                });
            })
        })
        .response;

    if response.clicked() && !node.active {
        weave.set_node_active_status(&Ulid(node.id), true);
        state.cursor_node = Some(Ulid(node.id));
    }

    if response.hovered() {
        state.hovered_node = Some(first_child);
    }
}

fn render_horizontal_node_label(
    ui: &mut Ui,
    settings: &Settings,
    state: &mut SharedState,
    weave: &mut TapestryWeave,
    node: &DependentNode<NodeContent>,
    buttons: impl Fn(
        &mut Ui,
        &Settings,
        &mut SharedState,
        &mut TapestryWeave,
        &DependentNode<NodeContent>,
    ),
    context_menu: impl Fn(
        &mut Ui,
        &Settings,
        &mut SharedState,
        &mut TapestryWeave,
        &DependentNode<NodeContent>,
    ),
    show_bookmarks_icon: bool,
) {
    let response = ui
        .scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
            let mut frame = Frame::new();

            let is_hovered = state.last_hovered_node == Some(Ulid(node.id));

            if is_hovered {
                frame = frame.fill(ui.style().visuals.widgets.hovered.weak_bg_fill);
            }

            frame.show(ui, |ui| {
                let mut label = RichText::new(String::from_utf8_lossy(
                    &node.contents.content.as_bytes().to_vec(),
                ))
                .family(FontFamily::Monospace);
                let label_color = get_node_color(node, settings);

                let mut label_button = if node.active {
                    if let Some(label_color) = label_color {
                        Button::new(label).fill(label_color).selected(true)
                    } else {
                        Button::new(label).selected(true)
                    }
                } else {
                    if let Some(label_color) = label_color {
                        label = label.color(label_color);
                    } else if is_hovered {
                        label = label.color(ui.style().visuals.widgets.hovered.text_color());
                    }
                    Button::new(label).fill(Color32::TRANSPARENT)
                };

                if is_hovered {
                    label_button =
                        label_button.stroke(ui.style().visuals.widgets.hovered.bg_stroke);
                }

                let mut label_button_response = ui.add(label_button);

                label_button_response.context_menu(|ui| {
                    context_menu(ui, settings, state, weave, node);
                });

                if should_render_node_metadata_tooltip(node) {
                    label_button_response = label_button_response
                        .on_hover_ui(|ui| render_node_metadata_tooltip(ui, node));
                }

                if label_button_response.hovered() {
                    state.hovered_node = Some(Ulid(node.id));
                }

                if label_button_response.clicked() && !node.active {
                    weave.set_node_active_status(&Ulid(node.id), true);
                    state.cursor_node = Some(Ulid(node.id));
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.rect_contains_pointer(ui.max_rect()) {
                        ui.scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
                            ui.add_space(ui.spacing().icon_spacing);
                            buttons(ui, settings, state, weave, node);
                            ui.add_space(ui.spacing().icon_spacing);

                            ui.add_space(0.0);
                        });
                        state.hovered_node = Some(Ulid(node.id));
                    } else if node.bookmarked && show_bookmarks_icon {
                        ui.add_space(ui.spacing().icon_spacing);
                        ui.label("\u{E060}");
                        ui.add_space(ui.spacing().icon_spacing);
                    } else {
                        ui.add_space(0.0);
                    }
                });
            })
        })
        .response;

    if response.clicked() && !node.active {
        weave.set_node_active_status(&Ulid(node.id), true);
        state.cursor_node = Some(Ulid(node.id));
    }

    if response.hovered() {
        state.hovered_node = Some(Ulid(node.id));
    }
}

fn render_node_context_menu(
    ui: &mut Ui,
    settings: &Settings,
    state: &mut SharedState,
    weave: &mut TapestryWeave,
    node: &DependentNode<NodeContent>,
) {
    if ui.button("Generate completions").clicked() {
        state.generate_children(weave, Some(Ulid(node.id)), settings);
    }

    let bookmark_label = if node.bookmarked {
        "Remove bookmark"
    } else {
        "Bookmark"
    };
    if ui.button(bookmark_label).clicked() {
        weave.set_node_bookmarked_status(&Ulid(node.id), !node.bookmarked);
    }

    ui.separator();

    if ui.button("Create child").clicked() {
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
    }

    if ui.button("Create sibling").clicked() {
        weave.add_node(DependentNode {
            id: Ulid::new().0,
            from: node.from,
            to: IndexSet::default(),
            active: false,
            bookmarked: false,
            contents: NodeContent {
                content: InnerNodeContent::Snippet(vec![]),
                metadata: IndexMap::new(),
                model: None,
            },
        });
    }

    if !node.to.is_empty() || node.from.is_some() {
        ui.separator();
    }

    if !node.to.is_empty() && ui.button("Delete all children").clicked() {
        for child in node.to.iter().copied() {
            weave.remove_node(&Ulid(child));
        }
    }

    if node.from.is_some() {
        if ui.button("Delete all siblings").clicked() {
            let parent = weave.get_node(&Ulid(node.from.unwrap()));
            let siblings: Vec<Ulid> = parent
                .iter()
                .flat_map(|parent| parent.to.iter().copied().filter(|id| *id != node.id))
                .map(Ulid)
                .collect();

            for child in siblings {
                weave.remove_node(&child);
            }
        }

        if weave.is_mergeable_with_parent(&Ulid(node.id))
            && ui.button("Merge with parent").clicked()
        {
            ui.separator();
            weave.merge_with_parent(&Ulid(node.id));
        }
    }

    ui.separator();

    if ui.button("Delete").clicked() {
        weave.remove_node(&Ulid(node.id));
    }
}

fn render_node_tree_context_menu(
    ui: &mut Ui,
    settings: &Settings,
    state: &mut SharedState,
    editor_id: Ulid,
    weave: &mut TapestryWeave,
    node: &DependentNode<NodeContent>,
) {
    if ui.button("Generate completions").clicked() {
        state.generate_children(weave, Some(Ulid(node.id)), settings);
    }

    let bookmark_label = if node.bookmarked {
        "Remove bookmark"
    } else {
        "Bookmark"
    };
    if ui.button(bookmark_label).clicked() {
        weave.set_node_bookmarked_status(&Ulid(node.id), !node.bookmarked);
    }

    ui.separator();

    if ui.button("Create child").clicked() {
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
    }

    if ui.button("Create sibling").clicked() {
        weave.add_node(DependentNode {
            id: Ulid::new().0,
            from: node.from,
            to: IndexSet::default(),
            active: false,
            bookmarked: false,
            contents: NodeContent {
                content: InnerNodeContent::Snippet(vec![]),
                metadata: IndexMap::new(),
                model: None,
            },
        });
    }

    if !node.to.is_empty() || node.from.is_some() {
        ui.separator();
    }

    if !node.to.is_empty() {
        if ui.button("Collapse all children").clicked() {
            for child in node.to.iter().copied() {
                set_node_tree_item_open_status(ui, editor_id, Ulid(child), false);
            }
        }

        if ui.button("Expand all children").clicked() {
            for child in node.to.iter().copied() {
                set_node_tree_item_open_status(ui, editor_id, Ulid(child), true);
            }
        }

        ui.separator();

        if ui.button("Delete all children").clicked() {
            for child in node.to.iter().copied() {
                weave.remove_node(&Ulid(child));
            }
        }
    }

    if node.from.is_some() {
        if ui.button("Delete all siblings").clicked() {
            let parent = weave.get_node(&Ulid(node.from.unwrap()));
            let siblings: Vec<Ulid> = parent
                .iter()
                .flat_map(|parent| parent.to.iter().copied().filter(|id| *id != node.id))
                .map(Ulid)
                .collect();

            for child in siblings {
                weave.remove_node(&child);
            }
        }

        if weave.is_mergeable_with_parent(&Ulid(node.id))
            && ui.button("Merge with parent").clicked()
        {
            ui.separator();
            weave.merge_with_parent(&Ulid(node.id));
        }
    }

    ui.separator();

    if ui.button("Delete").clicked() {
        weave.remove_node(&Ulid(node.id));
    }
}
