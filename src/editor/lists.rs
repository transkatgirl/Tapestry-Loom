#![allow(clippy::too_many_arguments)]

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, hash_map::Entry},
    rc::Rc,
    sync::Arc,
};

use eframe::egui::{
    Align, Button, Color32, FontFamily, Frame, Id, Layout, Pos2, Rect, RichText, ScrollArea, Sense,
    Ui, UiBuilder, Vec2, WidgetText, collapsing_header::CollapsingState,
    scroll_area::ScrollBarVisibility, vec2,
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
        NodeIndex, SharedState, change_color_opacity, get_node_color, render_node_metadata_tooltip,
        render_node_text_or_empty, render_token_tooltip, weave::WeaveWrapper,
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
                            for (index, item) in items.into_iter().enumerate() {
                                Self::render_item(weave, settings, state, ui, &item, index == 0);
                            }
                        } else {
                            self.list.ui_custom_layout(ui, items.len(), |ui, index| {
                                Self::render_item(
                                    weave,
                                    settings,
                                    state,
                                    ui,
                                    &items[index],
                                    index == 0,
                                );
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
        is_start: bool,
    ) {
        if let Some(node) = weave.get_node(item).cloned() {
            if !is_start {
                render_label_separator(ui, settings);
            }
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
                        render_node_context_menu(ui, settings, state, weave, node, false);
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
                            for (index, item) in items.into_iter().enumerate() {
                                Self::render_bookmark(
                                    weave,
                                    settings,
                                    state,
                                    ui,
                                    &item,
                                    index == 0,
                                );
                            }
                        } else {
                            self.list.ui_custom_layout(ui, items.len(), |ui, index| {
                                Self::render_bookmark(
                                    weave,
                                    settings,
                                    state,
                                    ui,
                                    &items[index],
                                    index == 0,
                                );
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
        is_start: bool,
    ) {
        if let Some(node) = weave.get_node(item).cloned() {
            if !is_start {
                render_label_separator(ui, settings);
            }
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
                        render_node_context_menu(ui, settings, state, weave, node, false);
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
    last_max_depth: usize,
}

impl Default for TreeListView {
    fn default() -> Self {
        Self {
            last_active_nodes: HashSet::with_capacity(65536),
            last_rendered_nodes: HashSet::with_capacity(65536),
            lists: HashMap::with_capacity(256),
            needs_list_refresh: false,
            last_max_depth: Settings::default().interface.max_tree_depth,
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
        _weave: &mut WeaveWrapper,
        settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
        _shortcuts: FlagSet<Shortcuts>,
    ) {
        if state.has_cursor_node_changed {
            self.last_active_nodes.clear();
            self.needs_list_refresh = true;
        } else if state.has_weave_changed
            || self.last_max_depth != settings.interface.max_tree_depth
            || state.has_opened_changed
        {
            self.needs_list_refresh = true;
        }
    }
    fn update_lists(&mut self, max_depth: usize) {
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

        self.needs_list_refresh = false;
        self.last_max_depth = max_depth;
    }
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut WeaveWrapper,
        settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
        shortcuts: FlagSet<Shortcuts>,
    ) {
        if shortcuts.contains(Shortcuts::CollapseAllVisibleInactive) {
            for item in self.last_rendered_nodes.iter().copied() {
                if !self.last_active_nodes.contains(&item) {
                    state.set_open(item, false);
                }
            }
            self.needs_list_refresh = true;
        }

        if shortcuts.contains(Shortcuts::ExpandAllVisible) {
            for item in self.last_rendered_nodes.iter().copied() {
                state.set_open(item, true);
            }
            self.needs_list_refresh = true;
        }

        if self.last_active_nodes.is_empty() {
            if let Some(cursor_node) = state.get_cursor_node().into_node() {
                let active = weave.get_thread_from_u128(&cursor_node.0).map(Ulid);

                for item in active {
                    self.last_active_nodes.insert(item);
                }
            }

            self.update_lists(settings.interface.max_tree_depth);
        }

        if (!settings.interface.optimize_tree || settings.interface.auto_scroll)
            && !self.lists.is_empty()
        {
            self.lists.clear();
        } else if (!settings.interface.auto_scroll && settings.interface.optimize_tree)
            && self.needs_list_refresh
        {
            self.update_lists(settings.interface.max_tree_depth);
        }

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
            .scroll_bar_visibility(if self.lists.is_empty() {
                ScrollBarVisibility::VisibleWhenNeeded
            } else {
                ScrollBarVisibility::AlwaysHidden // Workaround for bugs caused by nesting multiple virtual lists
            })
            .show(ui, |ui| {
                Frame::new()
                    .outer_margin(listing_margin(ui))
                    .show(ui, |ui| {
                        if weave.is_empty() {
                            ui.horizontal_wrapped(|ui| {
                                ui.add_space(ui.spacing().icon_spacing);
                                render_empty_tree_label(ui, settings, state, weave)
                            });
                        } else {
                            render_node_tree(
                                weave,
                                settings,
                                state,
                                ui,
                                state.identifier,
                                Ulid::nil(),
                                tree_roots.into_iter(),
                                settings.interface.max_tree_depth,
                                false,
                                &mut self.last_rendered_nodes,
                                &mut self.lists,
                                &mut self.needs_list_refresh,
                                true,
                                true,
                            );
                        }
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
    items: impl ExactSizeIterator<Item = Ulid>,
    max_depth: usize,
    within_virtual_list: bool,
    rendered_items: &mut HashSet<Ulid>,
    virtual_lists: &mut HashMap<Ulid, Rc<RefCell<VirtualList>>>,
    needs_list_refresh: &mut bool,
    is_display_root: bool,
    show_separators: bool,
) {
    let indent_compensation = ui.spacing().icon_width + ui.spacing().icon_spacing;

    if settings.interface.auto_scroll || !settings.interface.optimize_tree /*|| within_virtual_list*/ || items.len() < 10
    {
        for (index, item) in items.into_iter().enumerate() {
            render_node_tree_row(
                weave,
                settings,
                state,
                ui,
                editor_id,
                indent_compensation,
                item,
                max_depth,
                within_virtual_list,
                rendered_items,
                virtual_lists,
                needs_list_refresh,
                is_display_root,
                show_separators && !(is_display_root && index == 0),
                show_separators,
            );
        }
    } else if items.len() > 0 {
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
                    true,
                    rendered_items,
                    virtual_lists,
                    needs_list_refresh,
                    is_display_root,
                    show_separators && !(is_display_root && index == 0),
                    show_separators,
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
    within_virtual_list: bool,
    rendered_items: &mut HashSet<Ulid>,
    virtual_lists: &mut HashMap<Ulid, Rc<RefCell<VirtualList>>>,
    needs_list_refresh: &mut bool,
    is_display_root: bool,
    show_separator: bool,
    show_child_separators: bool,
) {
    let needs_list_refresh = RefCell::new(needs_list_refresh);

    if let Some(node) = weave.get_node(&item).cloned() {
        if show_separator {
            render_label_separator(ui, settings);
        }
        rendered_items.insert(item);

        let id = Id::new([editor_id.0, node.id, 0]);
        let mut collapsing = CollapsingState::load_with_default_open(ui.ctx(), id, true);
        collapsing.set_open(state.is_open(&Ulid(node.id)));

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
                        if is_display_root
                            && let Some(parent) = node.from
                            && ui
                                .button("\u{E042}")
                                .on_hover_text("Show parents")
                                .clicked()
                        {
                            state.set_cursor_node(NodeIndex::Node(Ulid(parent)));
                        };
                    },
                    |ui, settings, state, weave, node| {
                        render_node_context_menu(ui, settings, state, weave, node, true);
                    },
                    true,
                );
            });
        };

        if node.to.is_empty() {
            render_label(ui);
        } else {
            let collapsing_response = collapsing
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
                            within_virtual_list,
                            rendered_items,
                            virtual_lists,
                            &mut needs_list_refresh.borrow_mut(),
                            false,
                            show_child_separators,
                        );
                    } else {
                        if show_separator {
                            render_label_separator(ui, settings);
                        }
                        ui.horizontal_wrapped(|ui| {
                            let first_child = node.to.first().copied().map(Ulid).unwrap();
                            render_omitted_node_label(
                                ui,
                                state,
                                Ulid(node.id),
                                first_child,
                                "\u{E04A} Show more",
                            );
                        });
                    }
                });

            if collapsing_response.0.clicked() {
                state.set_open(
                    Ulid(node.id),
                    CollapsingState::load_with_default_open(ui.ctx(), id, true).is_open(),
                );
            }
        }
    }
}

pub fn render_horizontal_node_label_buttons_ltr(
    ui: &mut Ui,
    settings: &Settings,
    state: &mut SharedState,
    weave: &mut WeaveWrapper,
    node: &DependentNode<NodeContent>,
) {
    let is_shift_pressed = ui.input(|input| input.modifiers.shift);

    if weave.is_mergeable_with_parent(&Ulid(node.id))
        && ui
            .button("\u{E43F}")
            .on_hover_text("Merge node with parent")
            .clicked()
    {
        weave.merge_with_parent(&Ulid(node.id));
    };
    if ui
        .button("\u{E5CE}")
        .on_hover_text(if !is_shift_pressed {
            "Generate completions"
        } else {
            "Generate completions & focus node"
        })
        .clicked()
    {
        state.generate_children(weave, Some(Ulid(node.id)), settings);

        if is_shift_pressed {
            weave.set_node_active_status_u128(&node.id, true);
            state.set_cursor_node(NodeIndex::Node(Ulid(node.id)));
        }

        state.set_open(Ulid(node.id), true);
    };
    if ui
        .button("\u{E40C}")
        .on_hover_text(if !is_shift_pressed {
            "Add node"
        } else {
            "Add active node"
        })
        .clicked()
    {
        let identifier = Ulid::new().0;
        let active = if is_shift_pressed { true } else { node.active };

        if weave.add_node(DependentNode {
            id: identifier,
            from: Some(node.id),
            to: IndexSet::default(),
            active,
            bookmarked: false,
            contents: NodeContent {
                content: InnerNodeContent::Snippet(vec![]),
                metadata: IndexMap::new(),
                model: None,
            },
        }) {
            if active {
                state.set_cursor_node(NodeIndex::Node(Ulid(identifier)));
            } else {
                state.set_open(Ulid(node.id), true);
            }
        }
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
    if ui.button("\u{E28F}").on_hover_text("Delete node").clicked() {
        weave.remove_node(&Ulid(node.id));
    };
}

pub fn render_horizontal_node_label_buttons_rtl(
    ui: &mut Ui,
    settings: &Settings,
    state: &mut SharedState,
    weave: &mut WeaveWrapper,
    node: &DependentNode<NodeContent>,
) {
    let is_shift_pressed = ui.input(|input| input.modifiers.shift);

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
    if ui
        .button("\u{E40C}")
        .on_hover_text(if !is_shift_pressed {
            "Add node"
        } else {
            "Add active node"
        })
        .clicked()
    {
        let identifier = Ulid::new().0;
        let active = if is_shift_pressed { true } else { node.active };

        if weave.add_node(DependentNode {
            id: identifier,
            from: Some(node.id),
            to: IndexSet::default(),
            active,
            bookmarked: false,
            contents: NodeContent {
                content: InnerNodeContent::Snippet(vec![]),
                metadata: IndexMap::new(),
                model: None,
            },
        }) {
            if active {
                state.set_cursor_node(NodeIndex::Node(Ulid(identifier)));
            } else {
                state.set_open(Ulid(node.id), true);
            }
        }
    };
    if ui
        .button("\u{E5CE}")
        .on_hover_text(if !is_shift_pressed {
            "Generate completions"
        } else {
            "Generate completions & focus node"
        })
        .clicked()
    {
        state.generate_children(weave, Some(Ulid(node.id)), settings);

        if is_shift_pressed {
            weave.set_node_active_status_u128(&node.id, true);
            state.set_cursor_node(NodeIndex::Node(Ulid(node.id)));
        }

        state.set_open(Ulid(node.id), true);
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

fn render_omitted_node_label(
    ui: &mut Ui,
    state: &mut SharedState,
    selection_node: Ulid,
    hover_node: Ulid,
    label: impl Into<String>,
) {
    let response = ui
        .scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
            let mut frame = Frame::new();

            let is_hovered = state.get_hovered_node() == NodeIndex::Node(hover_node);

            if is_hovered {
                frame = frame.fill(ui.style().visuals.widgets.hovered.weak_bg_fill);
            }

            frame.show(ui, |ui| {
                let mut label = RichText::new(label).family(FontFamily::Proportional);

                if is_hovered {
                    label = label.color(ui.style().visuals.widgets.hovered.text_color());
                }

                let label_button_response =
                    ui.add(Button::new(label).frame(false).fill(Color32::TRANSPARENT));

                if label_button_response.contains_pointer() {
                    state.set_hovered_node(NodeIndex::Node(hover_node));
                }

                if label_button_response.clicked() {
                    state.set_cursor_node(NodeIndex::Node(selection_node));
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(0.0);
                });
            })
        })
        .response;

    if response.contains_pointer() {
        state.set_hovered_node(NodeIndex::Node(hover_node));
    }

    if response.clicked() {
        state.set_cursor_node(NodeIndex::Node(selection_node));
    }
}

fn render_empty_tree_label(
    ui: &mut Ui,
    _settings: &Settings,
    state: &mut SharedState,
    weave: &mut WeaveWrapper,
) {
    ui.scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
        let label_button_response = ui
            .add_enabled_ui(false, |ui| {
                let label = RichText::new("No nodes").family(FontFamily::Proportional);

                ui.add(Button::new(label).frame(false).fill(Color32::TRANSPARENT))
            })
            .response;

        let hover_rect = Rect {
            min: Pos2 {
                x: ui.min_rect().min.x,
                y: ui.max_rect().min.y,
            },
            max: Pos2 {
                x: ui.max_rect().max.x,
                y: ui.min_rect().max.y,
            },
        };

        let mouse_hovered =
            ui.rect_contains_pointer(hover_rect) || label_button_response.contains_pointer();

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if mouse_hovered {
                ui.add_space(ui.spacing().icon_spacing);
                if ui.button("\u{E40C}").on_hover_text("Add node").clicked() {
                    let identifier = Ulid::new().0;
                    if weave.add_node(DependentNode {
                        id: identifier,
                        from: None,
                        to: IndexSet::default(),
                        active: true,
                        bookmarked: false,
                        contents: NodeContent {
                            content: InnerNodeContent::Snippet(vec![]),
                            metadata: IndexMap::new(),
                            model: None,
                        },
                    }) {
                        state.set_cursor_node(NodeIndex::Node(Ulid(identifier)));
                    }
                };
                /*if ui
                    .button("\u{E5CE}")
                    .on_hover_text("Generate completions")
                    .clicked()
                {
                    state.generate_children(weave, None, settings);
                };*/
                ui.add_space(ui.spacing().icon_spacing);

                ui.add_space(0.0);
            } else {
                ui.add_space(0.0);
            }
        });
    });
}

// Based on egui::widgets::Separator
fn render_label_separator(ui: &mut Ui, settings: &Settings) {
    if settings.interface.list_separator_opacity < f32::EPSILON {
        return;
    }

    let available_space = if ui.is_sizing_pass() {
        Vec2::ZERO
    } else {
        ui.available_size_before_wrap()
    };

    let size = vec2(available_space.x, 0.0);

    let (rect, response) = ui.allocate_at_least(size, Sense::hover());

    if ui.is_rect_visible(response.rect) {
        let mut stroke = ui.visuals().widgets.noninteractive.bg_stroke;
        stroke.color = change_color_opacity(
            stroke.color,
            settings.interface.list_separator_opacity / 100.0,
        );
        let painter = ui.painter();

        painter.hline(rect.left()..=rect.right(), rect.center().y, stroke);
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
                let mut label = WidgetText::LayoutJob(Arc::new(render_node_text_or_empty(
                    ui,
                    node,
                    settings,
                    if node.active {
                        Some(ui.visuals().widgets.active.text_color())
                    } else {
                        None
                    },
                )));
                let label_color = get_node_color(node, settings);

                let mut label_button = if node.active {
                    if let Some(label_color) = label_color {
                        Button::new(label)
                            .fill(change_color_opacity(label_color, 0.5))
                            .selected(true)
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

                let label_button_response = ui.add(label_button).on_hover_ui(|ui| {
                    if let InnerNodeContent::Tokens(tokens) = &node.contents.content
                        && tokens.len() == 1
                        && let Some(token) = tokens.first()
                    {
                        render_token_tooltip(ui, &token.0, &token.1);

                        ui.separator();
                    }

                    render_node_metadata_tooltip(ui, node)
                });

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

                let hover_rect = Rect {
                    min: Pos2 {
                        x: ui.min_rect().min.x,
                        y: ui.max_rect().min.y,
                    },
                    max: Pos2 {
                        x: ui.max_rect().max.x,
                        y: ui.min_rect().max.y,
                    },
                };

                if ui.rect_contains_pointer(hover_rect) {
                    state.set_hovered_node(NodeIndex::Node(Ulid(node.id)));
                    mouse_hovered = true;
                }

                ui.scope_builder(
                    UiBuilder::new()
                        .max_rect(hover_rect)
                        .layout(Layout::right_to_left(Align::Center)),
                    |ui| {
                        if mouse_hovered {
                            ui.scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
                                ui.add_space(ui.spacing().icon_spacing);
                                buttons(ui, settings, state, weave, node);
                                //ui.add_space(ui.spacing().icon_spacing);

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
                    },
                );
            })
        })
        .response;

    response.context_menu(|ui| {
        context_menu(ui, settings, state, weave, node);
    });

    if response.contains_pointer() {
        state.set_hovered_node(NodeIndex::Node(Ulid(node.id)));
    }

    if response.clicked() {
        weave.set_node_active_status_u128(&node.id, true);
        state.set_cursor_node(NodeIndex::Node(Ulid(node.id)));
    }
}

pub fn render_node_context_menu(
    ui: &mut Ui,
    settings: &Settings,
    state: &mut SharedState,
    weave: &mut WeaveWrapper,
    node: &DependentNode<NodeContent>,
    collapsing: bool,
) {
    let is_shift_pressed = ui.input(|input| input.modifiers.shift);

    if ui.button("Generate completions").clicked() {
        state.generate_children(weave, Some(Ulid(node.id)), settings);

        if is_shift_pressed {
            weave.set_node_active_status_u128(&node.id, true);
            state.set_cursor_node(NodeIndex::Node(Ulid(node.id)));
        }

        state.set_open(Ulid(node.id), true);
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

    if ui
        .button(if !is_shift_pressed {
            "Create child"
        } else {
            "Create active child"
        })
        .clicked()
    {
        let identifier = Ulid::new().0;
        let active = if is_shift_pressed { true } else { node.active };

        if weave.add_node(DependentNode {
            id: identifier,
            from: Some(node.id),
            to: IndexSet::default(),
            active,
            bookmarked: false,
            contents: NodeContent {
                content: InnerNodeContent::Snippet(vec![]),
                metadata: IndexMap::new(),
                model: None,
            },
        }) {
            if active {
                state.set_cursor_node(NodeIndex::Node(Ulid(identifier)));
            } else {
                state.set_open(Ulid(node.id), true);
            }
        }
    }

    if ui
        .button(if !is_shift_pressed {
            "Create sibling"
        } else {
            "Create active sibling"
        })
        .clicked()
    {
        let identifier = Ulid::new().0;
        let active = if is_shift_pressed { true } else { node.active };

        if weave.add_node(DependentNode {
            id: identifier,
            from: node.from,
            to: IndexSet::default(),
            active,
            bookmarked: false,
            contents: NodeContent {
                content: InnerNodeContent::Snippet(vec![]),
                metadata: IndexMap::new(),
                model: None,
            },
        }) && active
        {
            state.set_cursor_node(NodeIndex::Node(Ulid(identifier)));
        }
    }

    ui.separator();

    if !node.to.is_empty() {
        if collapsing {
            if ui.button("Collapse all children").clicked() {
                for child in node.to.iter().copied() {
                    state.set_open(Ulid(child), false);
                }
            }

            if ui.button("Expand all children").clicked() {
                for child in node.to.iter().copied() {
                    state.set_open(Ulid(child), true);
                }
            }

            ui.separator();
        }

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
