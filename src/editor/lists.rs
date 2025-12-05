#![allow(clippy::too_many_arguments)]

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, hash_map::Entry},
    rc::Rc,
    sync::Arc,
};

use eframe::egui::{
    Align, Button, Color32, FontFamily, Frame, Id, Layout, RichText, ScrollArea, Sense, Ui,
    UiBuilder, WidgetText, collapsing_header::CollapsingState,
};
use egui_notify::Toasts;
use egui_virtual_list::VirtualList;
use flagset::FlagSet;
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{
        dependent::DependentNode,
        indexmap::{IndexMap, IndexSet},
    },
    v0::{InnerNodeContent, NodeContent},
};

use crate::{
    editor::shared::{
        NodeIndex, SharedState, get_node_color, render_node_metadata_tooltip, render_node_text,
        weave::WeaveWrapper,
    },
    listing_margin,
    settings::{Settings, shortcuts::Shortcuts},
};

#[derive(Debug)]
pub struct ListView {
    list: VirtualList,
}

impl Default for ListView {
    fn default() -> Self {
        let mut list = VirtualList::new();
        list.scroll_position_sync_on_resize(false);

        Self { list }
    }
}

impl ListView {
    /*pub fn reset(&mut self) {
        self.list.reset();
    }*/
    pub fn update(
        &mut self,
        _weave: &mut WeaveWrapper,
        _settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
        _shortcuts: FlagSet<Shortcuts>,
    ) {
        if state.has_weave_changed || state.has_cursor_node_changed {
            self.list.reset();
        }
    }
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut WeaveWrapper,
        settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
        _shortcuts: FlagSet<Shortcuts>,
    ) {
        let items: Vec<Ulid> = if let Some(cursor_node) = state
            .get_cursor_node()
            .into_node()
            .and_then(|id| weave.get_node(&id))
        {
            cursor_node.to.iter().cloned().map(Ulid).collect()
        } else {
            weave.get_roots().collect()
        };

        ScrollArea::vertical()
            .auto_shrink(false)
            .animated(false)
            .show(ui, |ui| {
                Frame::new()
                    .outer_margin(listing_margin(ui))
                    .show(ui, |ui| {
                        if settings.interface.auto_scroll {
                            for item in items {
                                Self::render_item(weave, settings, state, ui, &item);
                            }
                        } else {
                            self.list.ui_custom_layout(ui, items.len(), |ui, index| {
                                Self::render_item(weave, settings, state, ui, &items[index]);
                                1
                            });
                        }
                    });
            });
    }
    fn render_item(
        weave: &mut WeaveWrapper,
        settings: &Settings,
        state: &mut SharedState,
        ui: &mut Ui,
        item: &Ulid,
    ) {
        if let Some(node) = weave.get_node(item).cloned() {
            ui.horizontal_wrapped(|ui| {
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

#[derive(Debug)]
pub struct BookmarkListView {
    list: VirtualList,
}

impl Default for BookmarkListView {
    fn default() -> Self {
        let mut list = VirtualList::new();
        list.scroll_position_sync_on_resize(false);

        Self { list }
    }
}

impl BookmarkListView {
    /*pub fn reset(&mut self) {
        self.list.reset();
    }*/
    pub fn update(
        &mut self,
        _weave: &mut WeaveWrapper,
        _settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
        _shortcuts: FlagSet<Shortcuts>,
    ) {
        if state.has_weave_changed {
            self.list.reset();
        }
    }
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut WeaveWrapper,
        settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
        _shortcuts: FlagSet<Shortcuts>,
    ) {
        let items: Vec<Ulid> = weave.get_bookmarks().collect();

        if state.has_weave_changed {
            self.list.reset();
        }

        ScrollArea::vertical()
            .auto_shrink(false)
            .animated(false)
            .show(ui, |ui| {
                Frame::new()
                    .outer_margin(listing_margin(ui))
                    .show(ui, |ui| {
                        if settings.interface.auto_scroll {
                            for item in items {
                                Self::render_bookmark(weave, settings, state, ui, &item);
                            }
                        } else {
                            self.list.ui_custom_layout(ui, items.len(), |ui, index| {
                                Self::render_bookmark(weave, settings, state, ui, &items[index]);
                                1
                            });
                        }
                    });
            });
    }
    fn render_bookmark(
        weave: &mut WeaveWrapper,
        settings: &Settings,
        state: &mut SharedState,
        ui: &mut Ui,
        item: &Ulid,
    ) {
        if let Some(node) = weave.get_node(item).cloned() {
            ui.horizontal_wrapped(|ui| {
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
                            weave.set_node_bookmarked_status_u128(&node.id, false);
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

#[derive(Debug)]
pub struct TreeListView {
    last_active_nodes: HashSet<Ulid>,
    last_rendered_nodes: HashSet<Ulid>,
    lists: HashMap<Ulid, Rc<RefCell<VirtualList>>>,
    needs_list_refresh: bool,
}

impl Default for TreeListView {
    fn default() -> Self {
        Self {
            last_active_nodes: HashSet::with_capacity(65536),
            last_rendered_nodes: HashSet::with_capacity(65536),
            lists: HashMap::with_capacity(256),
            needs_list_refresh: false,
        }
    }
}

impl TreeListView {
    /*pub fn reset(&mut self) {
        self.last_active_nodes.clear();
        self.last_rendered_nodes.clear();
        self.lists.clear();
        self.needs_list_refresh = false;
    }*/
    pub fn update(
        &mut self,
        ui: &mut Ui,
        weave: &mut WeaveWrapper,
        settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
        shortcuts: FlagSet<Shortcuts>,
    ) {
        if state.has_cursor_node_changed || self.last_active_nodes.is_empty() {
            self.last_active_nodes.clear();

            if let Some(cursor_node) = state.get_cursor_node().into_node() {
                let active = weave.get_thread_from_u128(&cursor_node.0).map(Ulid);

                for item in active {
                    self.last_active_nodes.insert(item);
                    set_node_tree_item_open_status(ui, state.identifier, item, true);
                }
            }

            self.update_lists();
            self.needs_list_refresh = false;
        } else if state.has_weave_changed {
            self.update_lists();
            self.needs_list_refresh = false;
        }

        if shortcuts.contains(Shortcuts::ToggleNodeCollapsed)
            && let Some(item) = state.get_cursor_node().into_node()
        {
            toggle_node_tree_item_open_status(ui, state.identifier, item);
            self.needs_list_refresh = true;
        }

        if shortcuts.contains(Shortcuts::CollapseAllVisibleInactive) {
            for item in self.last_rendered_nodes.iter().copied() {
                if !self.last_active_nodes.contains(&item) {
                    set_node_tree_item_open_status(ui, state.identifier, item, false);
                }
            }
            self.needs_list_refresh = true;
        }

        if shortcuts.contains(Shortcuts::CollapseChildren)
            && let Some(node) = state
                .get_cursor_node()
                .into_node()
                .and_then(|id| weave.get_node(&id))
        {
            for item in node.to.iter().cloned().map(Ulid) {
                set_node_tree_item_open_status(ui, state.identifier, item, false);
            }
            self.needs_list_refresh = true;
        }

        if shortcuts.contains(Shortcuts::ExpandAllVisible) {
            for item in self.last_rendered_nodes.iter().copied() {
                set_node_tree_item_open_status(ui, state.identifier, item, true);
            }
            self.needs_list_refresh = true;
        }

        if shortcuts.contains(Shortcuts::ExpandChildren)
            && let Some(node) = state
                .get_cursor_node()
                .into_node()
                .and_then(|id| weave.get_node(&id))
        {
            for item in node.to.iter().cloned().map(Ulid) {
                set_node_tree_item_open_status(ui, state.identifier, item, true);
            }
            self.needs_list_refresh = true;
        }

        if (settings.interface.auto_scroll && !self.lists.is_empty())
            || (!settings.interface.auto_scroll && self.needs_list_refresh)
        {
            self.update_lists();
            self.needs_list_refresh = false;
        }
    }
    fn update_lists(&mut self) {
        let mut removal_list = Vec::with_capacity(self.lists.len());

        for (id, list) in self.lists.iter() {
            if self.last_rendered_nodes.contains(id)
                || self.last_active_nodes.contains(id)
                || *id == Ulid::nil()
            {
                list.borrow_mut().reset();
            } else {
                removal_list.push(*id);
            }
        }

        for item in removal_list {
            self.lists.remove(&item);
        }
    }
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut WeaveWrapper,
        settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
        _shortcuts: FlagSet<Shortcuts>,
    ) {
        let tree_roots: Vec<Ulid> = if let Some(cursor_node) = state
            .get_cursor_node()
            .into_node()
            .and_then(|id| weave.get_node(&id))
            && let Some(cursor_node_parent) =
                cursor_node.from.and_then(|id| weave.get_node(&Ulid(id)))
            && let Some(cursor_node_parent_parent) = cursor_node_parent.from
        {
            if !cursor_node.to.is_empty() {
                vec![Ulid(cursor_node_parent.id)]
            } else {
                vec![Ulid(cursor_node_parent_parent)]
            }
        } else {
            weave.get_roots().collect()
        };

        self.last_rendered_nodes.clear();

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
                            Ulid::nil(),
                            tree_roots,
                            settings.interface.max_tree_depth,
                            &mut self.last_rendered_nodes,
                            &mut self.lists,
                            &mut self.needs_list_refresh,
                        );
                    });
            });
    }
}

fn render_node_tree(
    weave: &mut WeaveWrapper,
    settings: &Settings,
    state: &mut SharedState,
    ui: &mut Ui,
    editor_id: Ulid,
    branch_identifier: Ulid,
    items: impl IntoIterator<Item = Ulid>,
    max_depth: usize,
    rendered_items: &mut HashSet<Ulid>,
    virtual_lists: &mut HashMap<Ulid, Rc<RefCell<VirtualList>>>,
    needs_list_refresh: &mut bool,
) {
    let indent_compensation = ui.spacing().icon_width + ui.spacing().icon_spacing;

    if settings.interface.auto_scroll {
        for item in items {
            render_node_tree_row(
                weave,
                settings,
                state,
                ui,
                editor_id,
                indent_compensation,
                item,
                max_depth,
                rendered_items,
                virtual_lists,
                needs_list_refresh,
            );
        }
    } else {
        let virtual_list = match virtual_lists.entry(branch_identifier) {
            Entry::Occupied(occupied) => occupied,
            Entry::Vacant(vacant) => {
                let mut list = VirtualList::new();
                list.scroll_position_sync_on_resize(false);

                vacant.insert_entry(Rc::new(RefCell::new(list)))
            }
        }
        .get()
        .clone();

        let items: Vec<Ulid> = items.into_iter().collect();

        virtual_list
            .borrow_mut()
            .ui_custom_layout(ui, items.len(), |ui, index| {
                render_node_tree_row(
                    weave,
                    settings,
                    state,
                    ui,
                    editor_id,
                    indent_compensation,
                    items[index],
                    max_depth,
                    rendered_items,
                    virtual_lists,
                    needs_list_refresh,
                );
                1
            });
    }
}

fn render_node_tree_row(
    weave: &mut WeaveWrapper,
    settings: &Settings,
    state: &mut SharedState,
    ui: &mut Ui,
    editor_id: Ulid,
    indent_compensation: f32,
    item: Ulid,
    max_depth: usize,
    rendered_items: &mut HashSet<Ulid>,
    virtual_lists: &mut HashMap<Ulid, Rc<RefCell<VirtualList>>>,
    needs_list_refresh: &mut bool,
) {
    if let Some(node) = weave.get_node(&item).cloned() {
        rendered_items.insert(item);
        let mut render_label = |ui: &mut Ui| {
            ui.horizontal_wrapped(|ui| {
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
                        render_horizontal_node_label_buttons_rtl(ui, settings, state, weave, node);
                    },
                    |ui, settings, state, weave, node| {
                        render_node_tree_context_menu(
                            ui,
                            settings,
                            state,
                            editor_id,
                            weave,
                            node,
                            needs_list_refresh,
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
            let collapsing_response = CollapsingState::load_with_default_open(ui.ctx(), id, false)
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
                            item,
                            node.to.into_iter().map(Ulid),
                            max_depth - 1,
                            rendered_items,
                            virtual_lists,
                            needs_list_refresh,
                        );
                    } else {
                        ui.horizontal_wrapped(|ui| {
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

            if collapsing_response.0.clicked() {
                *needs_list_refresh = true;
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

fn toggle_node_tree_item_open_status(ui: &mut Ui, editor_id: Ulid, item_id: Ulid) {
    let id = Id::new([editor_id.0, item_id.0, 0]);
    let mut collapsing_state = CollapsingState::load_with_default_open(ui.ctx(), id, false);
    collapsing_state.set_open(!collapsing_state.is_open());
    collapsing_state.store(ui.ctx());
}

fn render_horizontal_node_label_buttons_rtl(
    ui: &mut Ui,
    settings: &Settings,
    state: &mut SharedState,
    weave: &mut WeaveWrapper,
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
        weave.set_node_bookmarked_status_u128(&node.id, !node.bookmarked);
    };
    if ui.button("\u{E40C}").on_hover_text("Add node").clicked() {
        let identifier = Ulid::new().0;
        if weave.add_node(DependentNode {
            id: identifier,
            from: Some(node.id),
            to: IndexSet::default(),
            active: node.active,
            bookmarked: false,
            contents: NodeContent {
                content: InnerNodeContent::Snippet(vec![]),
                metadata: IndexMap::new(),
                model: None,
            },
        }) && node.active
        {
            state.set_cursor_node(NodeIndex::Node(Ulid(identifier)));
        }
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
    weave: &mut WeaveWrapper,
    node: &DependentNode<NodeContent>,
    first_child: Ulid,
) {
    let response = ui
        .scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
            let mut frame = Frame::new();

            let is_hovered = state.get_hovered_node() == NodeIndex::Node(first_child);

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

                if label_button_response.contains_pointer() {
                    state.set_hovered_node(NodeIndex::Node(first_child));
                }

                if label_button_response.clicked() {
                    weave.set_node_active_status_u128(&node.id, true);
                    state.set_cursor_node(NodeIndex::Node(Ulid(node.id)));
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(0.0);
                });
            })
        })
        .response;

    if response.clicked() {
        weave.set_node_active_status_u128(&node.id, true);
        state.set_cursor_node(NodeIndex::Node(Ulid(node.id)));
    }

    if response.contains_pointer() {
        state.set_hovered_node(NodeIndex::Node(first_child));
    }
}

fn render_horizontal_node_label(
    ui: &mut Ui,
    settings: &Settings,
    state: &mut SharedState,
    weave: &mut WeaveWrapper,
    node: &DependentNode<NodeContent>,
    mut buttons: impl FnMut(
        &mut Ui,
        &Settings,
        &mut SharedState,
        &mut WeaveWrapper,
        &DependentNode<NodeContent>,
    ),
    mut context_menu: impl FnMut(
        &mut Ui,
        &Settings,
        &mut SharedState,
        &mut WeaveWrapper,
        &DependentNode<NodeContent>,
    ),
    show_node_info: bool,
) {
    let mut mouse_hovered = false;

    let response = ui
        .scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
            let mut frame = Frame::new();

            let is_hovered = state.get_hovered_node().into_node() == Some(Ulid(node.id));
            let is_cursor = state.get_cursor_node().into_node() == Some(Ulid(node.id));
            let is_changed = state.get_changed_node() == Some(Ulid(node.id));

            if is_hovered {
                frame = frame.fill(ui.style().visuals.widgets.hovered.weak_bg_fill);
            }

            frame.show(ui, |ui| {
                let mut label = if node.contents.content.as_bytes().is_empty() {
                    WidgetText::Text("No text".to_string())
                } else {
                    WidgetText::LayoutJob(Arc::new(render_node_text(
                        ui,
                        node,
                        settings,
                        if node.active {
                            Some(ui.visuals().widgets.active.text_color())
                        } else {
                            None
                        },
                    )))
                };
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

                if
                /*is_hovered ||*/
                is_cursor {
                    label_button =
                        label_button.stroke(ui.style().visuals.widgets.hovered.bg_stroke);
                }

                let label_button_response = ui
                    .add(label_button)
                    .on_hover_ui(|ui| render_node_metadata_tooltip(ui, node));

                if settings.interface.auto_scroll && is_changed {
                    label_button_response.scroll_to_me(None);
                }

                label_button_response.context_menu(|ui| {
                    context_menu(ui, settings, state, weave, node);
                });

                if label_button_response.contains_pointer() {
                    mouse_hovered = true;
                    state.set_hovered_node(NodeIndex::Node(Ulid(node.id)));
                }

                if label_button_response.clicked() {
                    weave.set_node_active_status_u128(&node.id, true);
                    state.set_cursor_node(NodeIndex::Node(Ulid(node.id)));
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.rect_contains_pointer(ui.max_rect()) {
                        state.set_hovered_node(NodeIndex::Node(Ulid(node.id)));
                        mouse_hovered = true;
                    }

                    if mouse_hovered {
                        ui.scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
                            ui.add_space(ui.spacing().icon_spacing);
                            buttons(ui, settings, state, weave, node);
                            ui.add_space(ui.spacing().icon_spacing);

                            ui.add_space(0.0);
                        });
                    } else if show_node_info {
                        ui.add_space(ui.spacing().icon_spacing);
                        if node.bookmarked {
                            ui.label("\u{E060}");
                        }
                        if let InnerNodeContent::Tokens(tokens) = &node.contents.content
                            && tokens.len() == 1
                            && let Some(token) = tokens.first()
                            && let Some(probability) = token.1.get("probability")
                            && let Ok(probability) = probability.parse::<f32>()
                        {
                            ui.label(format!("{:.1}%", probability * 100.0));
                        }
                        ui.add_space(ui.spacing().icon_spacing);
                    } else {
                        ui.add_space(0.0);
                    }
                });
            })
        })
        .response;

    response.context_menu(|ui| {
        context_menu(ui, settings, state, weave, node);
    });

    if response.clicked() {
        weave.set_node_active_status_u128(&node.id, true);
        state.set_cursor_node(NodeIndex::Node(Ulid(node.id)));
    }

    if response.contains_pointer() {
        state.set_hovered_node(NodeIndex::Node(Ulid(node.id)));
    }
}

fn render_node_context_menu(
    ui: &mut Ui,
    settings: &Settings,
    state: &mut SharedState,
    weave: &mut WeaveWrapper,
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
        weave.set_node_bookmarked_status_u128(&node.id, !node.bookmarked);
    }

    ui.separator();

    if ui.button("Create child").clicked() {
        let identifier = Ulid::new().0;
        if weave.add_node(DependentNode {
            id: identifier,
            from: Some(node.id),
            to: IndexSet::default(),
            active: node.active,
            bookmarked: false,
            contents: NodeContent {
                content: InnerNodeContent::Snippet(vec![]),
                metadata: IndexMap::new(),
                model: None,
            },
        }) && node.active
        {
            state.set_cursor_node(NodeIndex::Node(Ulid(identifier)));
        }
    }

    if ui.button("Create sibling").clicked() {
        let identifier = Ulid::new().0;
        if weave.add_node(DependentNode {
            id: identifier,
            from: node.from,
            to: IndexSet::default(),
            active: node.active,
            bookmarked: false,
            contents: NodeContent {
                content: InnerNodeContent::Snippet(vec![]),
                metadata: IndexMap::new(),
                model: None,
            },
        }) && node.active
        {
            state.set_cursor_node(NodeIndex::Node(Ulid(identifier)));
        }
    }

    ui.separator();

    if !node.to.is_empty() && ui.button("Delete all children").clicked() {
        for child in node.to.iter().copied() {
            weave.remove_node(&Ulid(child));
        }
    }

    if ui.button("Delete all siblings").clicked() {
        let siblings: Vec<Ulid> =
            if let Some(parent) = node.from.and_then(|id| weave.get_node(&Ulid(id))) {
                parent
                    .to
                    .iter()
                    .copied()
                    .filter(|id| *id != node.id)
                    .map(Ulid)
                    .collect()
            } else {
                weave
                    .get_roots_u128()
                    .filter(|id| *id != node.id)
                    .map(Ulid)
                    .collect()
            };

        for sibling in siblings {
            weave.remove_node(&sibling);
        }
    }

    if node.from.is_some()
        && weave.is_mergeable_with_parent(&Ulid(node.id))
        && ui.button("Merge with parent").clicked()
    {
        ui.separator();
        weave.merge_with_parent(&Ulid(node.id));
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
    weave: &mut WeaveWrapper,
    node: &DependentNode<NodeContent>,
    needs_list_refresh: &mut bool,
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
        weave.set_node_bookmarked_status_u128(&node.id, !node.bookmarked);
    }

    ui.separator();

    if ui.button("Create child").clicked() {
        let identifier = Ulid::new().0;
        if weave.add_node(DependentNode {
            id: identifier,
            from: Some(node.id),
            to: IndexSet::default(),
            active: node.active,
            bookmarked: false,
            contents: NodeContent {
                content: InnerNodeContent::Snippet(vec![]),
                metadata: IndexMap::new(),
                model: None,
            },
        }) && node.active
        {
            state.set_cursor_node(NodeIndex::Node(Ulid(identifier)));
        }
    }

    if ui.button("Create sibling").clicked() {
        let identifier = Ulid::new().0;
        if weave.add_node(DependentNode {
            id: identifier,
            from: node.from,
            to: IndexSet::default(),
            active: node.active,
            bookmarked: false,
            contents: NodeContent {
                content: InnerNodeContent::Snippet(vec![]),
                metadata: IndexMap::new(),
                model: None,
            },
        }) && node.active
        {
            state.set_cursor_node(NodeIndex::Node(Ulid(identifier)));
        }
    }

    ui.separator();

    if !node.to.is_empty() {
        if ui.button("Collapse all children").clicked() {
            for child in node.to.iter().copied() {
                set_node_tree_item_open_status(ui, editor_id, Ulid(child), false);
            }
            *needs_list_refresh = true;
        }

        if ui.button("Expand all children").clicked() {
            for child in node.to.iter().copied() {
                set_node_tree_item_open_status(ui, editor_id, Ulid(child), true);
            }
            *needs_list_refresh = true;
        }

        ui.separator();

        if ui.button("Delete all children").clicked() {
            for child in node.to.iter().copied() {
                weave.remove_node(&Ulid(child));
            }
        }
    }

    if ui.button("Delete all siblings").clicked() {
        let siblings: Vec<Ulid> =
            if let Some(parent) = node.from.and_then(|id| weave.get_node_u128(&id)) {
                parent
                    .to
                    .iter()
                    .copied()
                    .filter(|id| *id != node.id)
                    .map(Ulid)
                    .collect()
            } else {
                weave
                    .get_roots_u128()
                    .filter(|id| *id != node.id)
                    .map(Ulid)
                    .collect()
            };

        for sibling in siblings {
            weave.remove_node(&sibling);
        }
    }

    if node.from.is_some()
        && weave.is_mergeable_with_parent(&Ulid(node.id))
        && ui.button("Merge with parent").clicked()
    {
        ui.separator();
        weave.merge_with_parent(&Ulid(node.id));
    }

    ui.separator();

    if ui.button("Delete").clicked() {
        weave.remove_node_u128(&node.id);
    }
}
