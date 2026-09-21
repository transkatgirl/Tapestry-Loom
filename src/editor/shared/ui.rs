use std::{collections::HashMap, ops::Range, sync::Arc};

use eframe::egui::{
    Align, Button, Color32, FontFamily, Frame, Layout, Pos2, Rect, RichText, ScrollArea, Sense,
    TextFormat, TextStyle, TextWrapMode, Ui, UiBuilder, WidgetText,
    containers::menu::SubMenuButton,
    text::{ByteIndex, LayoutJob, LayoutSection},
};
use flagset::{FlagSet, flags};
use tapestry_weave::{
    Author, Creator, InnerNodeContent, InnerNodeToken, MetadataMap, NodeContent, OriginalToken,
    ShortId, TapestryNode, UNKNOWN_MODEL_LABEL,
    content::sort,
    jiff::Zoned,
    nanorand::WyRand,
    universal_weave::{
        BookmarkableWeave, DiscreteWeave, SortableBookmarkableWeave, SortableWeave, Weave,
        indexmap::IndexSet,
    },
    weave::wrappers::LoggedTapestryWeave,
};
use ulid::Ulid;

use crate::{
    common::ui::{from_utf8_lossy, multiply_color_alpha},
    editor::settings::{
        interface::{InterfaceSettings, NodeColors, TokenColors},
        shortcuts::Shortcuts,
    },
    inference::InferenceEngine,
};

#[derive(Default)]
pub struct WeaveUi {
    pub cursor: Option<ShortId>,
    last_cursor: Option<ShortId>,
    pub opened: HashMap<ShortId, bool>,
    hovered: Option<ShortId>,
    last_hovered: Option<ShortId>,
    scroll_to: Option<ShortId>,

    generate: Option<ShortId>,
    seriate: Option<ShortId>,
    rendered_collapsing_labels: Vec<ShortId>,
    path_buffer: Vec<ShortId>,
    rng: WyRand,

    settings: InterfaceSettings,
}

pub const DEFAULT_OPEN: bool = false;

impl WeaveUi {
    fn generate_id(&mut self, weave: &LoggedTapestryWeave) -> ShortId {
        tapestry_weave::generate_id(&mut self.rng, weave)
    }
    fn new_user_node(
        id: ShortId,
        from: impl IntoIterator<Item = ShortId>,
        active: bool,
        user: &Option<Author>,
    ) -> TapestryNode {
        TapestryNode {
            id,
            from: IndexSet::from_iter(from),
            to: IndexSet::default(),
            active,
            bookmarked: false,
            contents: NodeContent {
                timestamp: Zoned::now(),
                modified: false,
                content: InnerNodeContent::MetadataOnly,
                metadata: MetadataMap::default(),
                creator: Creator::User(user.clone()),
            },
        }
    }
    pub fn logic(
        &mut self,
        weave: &mut LoggedTapestryWeave,
        settings: &InterfaceSettings,
        inference: &mut InferenceEngine,
        id: Ulid,
        shortcuts: FlagSet<Shortcuts>,
    ) {
        self.scroll_to = None;
        if self.last_hovered != self.hovered && self.hovered.is_some() {
            self.scroll_to = self.hovered;
        }
        self.last_hovered = self.hovered;
        self.hovered = None;

        if let Some(cursor) = self.cursor
            && !weave.contains(&cursor)
        {
            self.cursor = None;
        }

        if self.cursor.is_none() {
            weave.get_active_path(&mut self.path_buffer);

            if let Some(thread_tail) = self.path_buffer.first().copied() {
                self.cursor = Some(thread_tail);
            }
        }

        if self.last_cursor != self.cursor
            && let Some(cursor) = self.cursor
        {
            self.scroll_to = Some(cursor);

            weave.get_path_from(&cursor, &mut self.path_buffer);

            for node in self.path_buffer.iter().copied() {
                self.opened.insert(node, true);
            }
        }

        self.last_cursor = self.cursor;

        self.rendered_collapsing_labels.clear();

        if let Some(generate) = self.generate.take() {
            inference.generate_children(id, weave, generate);
        }

        // TODO: generated children should be scroll_to

        if let Some(seriate) = self.seriate.take() {
            inference.seriate_siblings(id, weave, seriate);
        }

        self.settings = *settings;

        // TODO
    }
    pub fn calculate_autoscroll(&mut self, ui: &mut Ui) -> Option<AutoscrollData> {
        let contains_pointer = ui
            .clip_rect()
            .contains(ui.ctx().pointer_hover_pos().unwrap_or_default());

        if !contains_pointer || self.cursor == self.scroll_to {
            Some(AutoscrollData {
                max_autoscroll_height: ui.available_size_before_wrap().y,
            })
        } else {
            None
        }
    }
    pub fn horizontal_node_label(
        &mut self,
        weave: &mut LoggedTapestryWeave,
        node: &TapestryNode,
        ui: &mut Ui,
        options: &LabelOptions,
        user: &Option<Author>,
        in_place: bool,
    ) {
        let mut mouse_hovered = false;

        if options.collapsing {
            self.rendered_collapsing_labels.push(node.id);
        }

        let response = ui
            .scope_builder(UiBuilder::new().sense(Sense::CLICK), |ui| {
                let mut frame = Frame::new();

                let is_hovered = self.last_hovered == Some(node.id);
                let is_cursor = self.cursor == Some(node.id);
                let is_focus = self.scroll_to == Some(node.id);

                if is_hovered {
                    frame = frame.fill(ui.visuals().widgets.hovered.weak_bg_fill);
                }

                frame.show(ui, |ui| {
                    // TODO: Cache node LayoutJobs
                    let label = WidgetText::LayoutJob(Arc::new(self.node_text(
                        ui,
                        node,
                        TextFlags::EmptyNotice.into(),
                    )));
                    let label_color = self.node_color(node);

                    let mut label_button = Button::new(label);

                    if
                    /*is_hovered ||*/
                    is_cursor {
                        label_button = label_button.stroke(ui.visuals().widgets.hovered.bg_stroke);
                    }

                    label_button = if node.active {
                        if let Some(label_color) = label_color {
                            label_button
                                .fill(multiply_color_alpha(label_color, 0.5))
                                .selected(true)
                        } else {
                            label_button.selected(true)
                        }
                    } else {
                        label_button.fill(Color32::TRANSPARENT)
                    };

                    let label_button_response = ui.add(label_button).on_hover_ui(|ui| {
                        if let InnerNodeContent::Tokens(tokens) = &node.contents.content
                            && tokens.len() == 1
                            && let Some(token) = tokens.first()
                        {
                            self.token_tooltip(token, ui, TokenTooltipFlags::WarnModified.into());
                            ui.separator();
                        }

                        self.node_tooltip(node, ui);
                    });

                    if let Some(autoscroll) = &options.autoscroll
                        && is_focus
                        && (autoscroll.max_autoscroll_height >= label_button_response.rect.height()
                            || ui.input(|i| i.modifiers.any()))
                    {
                        label_button_response.scroll_to_me(None);
                    }

                    label_button_response.context_menu(|ui| {
                        self.node_context_menu(weave, node, ui, options.collapsing, user);
                    });

                    if label_button_response.contains_pointer() {
                        mouse_hovered = true;
                        self.hovered = Some(node.id);
                    }

                    if label_button_response.clicked() {
                        if label_button_response.clicked_with_open_in_background() == in_place {
                            weave.set_active(&node.id, true);
                        } else {
                            weave.set_active_tree_semantics(&node.id, true);
                        }
                        self.cursor = Some(node.id);
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
                        mouse_hovered = true;
                        self.hovered = Some(node.id);
                    }

                    ui.scope_builder(
                        UiBuilder::new()
                            .max_rect(hover_rect)
                            .layout(Layout::right_to_left(Align::Center)),
                        |ui| {
                            if mouse_hovered {
                                ui.scope_builder(UiBuilder::new().sense(Sense::CLICK), |ui| {
                                    ui.add_space(ui.spacing().icon_spacing);
                                    self.node_buttons(
                                        weave,
                                        node,
                                        ui,
                                        options.buttons | ButtonFlags::Rtl,
                                        user,
                                    );

                                    ui.add_space(0.0);
                                });
                            } else if options.show_info {
                                ui.add_space(ui.spacing().icon_spacing);

                                if node.bookmarked {
                                    ui.label("\u{E060}");
                                }

                                if let InnerNodeContent::Tokens(tokens) = &node.contents.content
                                    && tokens.len() == 1
                                    && let Some(logprob) = tokens[0].logprob
                                    && logprob.is_finite()
                                {
                                    ui.label(format!("{:.1}%", logprob.exp() * 100.0));
                                }

                                ui.add_space(ui.spacing().icon_spacing);
                            } else {
                                ui.add_space(0.0);
                            }
                        },
                    );
                });
            })
            .response;

        response.context_menu(|ui| {
            self.node_context_menu(weave, node, ui, options.collapsing, user);
        });

        if response.contains_pointer() {
            self.hovered = Some(node.id);
        }

        if response.clicked() {
            weave.set_active_tree_semantics(&node.id, true);
            self.cursor = Some(node.id);
        }
    }
    pub fn horizontal_omitted_node_label(&mut self, node: ShortId, ui: &mut Ui) {
        let mut mouse_hovered = false;

        let response = ui
            .scope_builder(UiBuilder::new().sense(Sense::CLICK), |ui| {
                let mut frame = Frame::new();

                let is_hovered = self.last_hovered == Some(node);

                if is_hovered {
                    frame = frame.fill(ui.visuals().widgets.hovered.weak_bg_fill);
                }

                frame.show(ui, |ui| {
                    let label =
                        RichText::new("\u{E04A} Show more").family(FontFamily::Proportional);

                    let label_button_response =
                        ui.add(Button::new(label).fill(Color32::TRANSPARENT));

                    if label_button_response.contains_pointer() {
                        mouse_hovered = true;
                        self.hovered = Some(node);
                    }

                    if label_button_response.clicked() {
                        self.cursor = Some(node);
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
                        mouse_hovered = true;
                        self.hovered = Some(node);
                    }

                    ui.scope_builder(
                        UiBuilder::new()
                            .max_rect(hover_rect)
                            .layout(Layout::right_to_left(Align::Center)),
                        |ui| {
                            ui.add_space(0.0);
                        },
                    );
                });
            })
            .response;

        if response.contains_pointer() {
            self.hovered = Some(node);
        }

        if response.clicked() {
            self.cursor = Some(node);
        }
    }
    pub fn horizontal_empty_document_label(
        &mut self,
        weave: &mut LoggedTapestryWeave,
        ui: &mut Ui,
        user: &Option<Author>,
    ) {
        let response = ui
            .scope_builder(UiBuilder::new().sense(Sense::CLICK), |ui| {
                let frame = Frame::new();

                frame.show(ui, |ui| {
                    let label_button_response = ui.add_enabled(
                        false,
                        Button::new(RichText::new("No nodes").family(FontFamily::Proportional))
                            .fill(Color32::TRANSPARENT),
                    );

                    label_button_response.context_menu(|ui| {
                        self.document_context_menu(
                            weave,
                            ui,
                            DocumentContextFlags::Roots.into(),
                            user,
                        )
                    });

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

                    let mouse_hovered = label_button_response.contains_pointer()
                        || ui.rect_contains_pointer(hover_rect);

                    ui.scope_builder(
                        UiBuilder::new()
                            .max_rect(hover_rect)
                            .layout(Layout::right_to_left(Align::Center)),
                        |ui| {
                            if mouse_hovered {
                                ui.scope_builder(UiBuilder::new().sense(Sense::CLICK), |ui| {
                                    ui.add_space(ui.spacing().icon_spacing);

                                    let add_response =
                                        ui.button("\u{E40C}").on_hover_text("Add node");
                                    if add_response.clicked() {
                                        let identifier = self.generate_id(weave);

                                        if weave.insert_deduplicated(Self::new_user_node(
                                            identifier,
                                            [],
                                            true,
                                            user,
                                        )) {
                                            self.cursor = Some(identifier);
                                        }
                                    };

                                    ui.add_space(0.0);
                                });
                            } else {
                                ui.add_space(0.0);
                            }
                        },
                    );
                });
            })
            .response;

        response.context_menu(|ui| {
            self.document_context_menu(weave, ui, DocumentContextFlags::Roots.into(), user)
        });
    }
    pub fn node_text(
        &mut self,
        ui: &Ui,
        node: &TapestryNode,
        flags: FlagSet<TextFlags>,
    ) -> LayoutJob {
        let node_color = self
            .node_color(node)
            .unwrap_or(ui.visuals().widgets.inactive.text_color());
        let font_id = TextStyle::Monospace.resolve(ui.style());

        let handle_empty = || {
            if !flags.contains(TextFlags::EmptyNotice) {
                LayoutJob {
                    break_on_newline: true,
                    ..Default::default()
                }
            } else {
                let mut notice_font_id = TextStyle::Body.resolve(ui.style());
                notice_font_id.size = font_id.size;

                LayoutJob {
                    text: "No text".to_string(),
                    sections: vec![LayoutSection {
                        leading_space: 0.0,
                        byte_range: ByteIndex(0)..ByteIndex(("No text").len()),
                        format: TextFormat {
                            font_id: notice_font_id,
                            color: node_color,
                            valign: ui.text_valign(),
                            ..Default::default()
                        },
                    }],
                    break_on_newline: true,
                    ..Default::default()
                }
            }
        };

        match &node.contents.content {
            InnerNodeContent::Tokens(tokens) => {
                if tokens.iter().all(|t| t.bytes.is_empty()) {
                    handle_empty()
                } else if flags.contains(TextFlags::FirstTokenBytes)
                    && tokens.len() == 1
                    && !tokens[0].is_modified()
                    && str::from_utf8(&tokens[0].bytes).is_err()
                {
                    let token = &tokens[0];
                    let token_color = self.token_color(node_color, token);
                    let token_text = format!("{:?}", token.bytes);
                    let token_text_length = token_text.len();

                    LayoutJob {
                        text: token_text,
                        sections: vec![LayoutSection {
                            leading_space: 0.0,
                            byte_range: Range {
                                start: ByteIndex(0),
                                end: ByteIndex(token_text_length),
                            },
                            format: TextFormat {
                                font_id,
                                color: token_color,
                                valign: ui.text_valign(),
                                ..Default::default()
                            },
                        }],
                        break_on_newline: true,
                        ..Default::default()
                    }
                } else {
                    let text = from_utf8_lossy(
                        &tokens
                            .iter()
                            .flat_map(|t| t.bytes.iter().cloned())
                            .collect::<Vec<u8>>(),
                    )
                    .to_string();
                    let mut offset = 0;

                    let mut sections = Vec::with_capacity(tokens.len());

                    for token in tokens {
                        if token.bytes.is_empty() {
                            continue;
                        }

                        let token_color = self.token_color(node_color, token);
                        let token_length = token.bytes.len();

                        sections.push(LayoutSection {
                            leading_space: 0.0,
                            byte_range: Range {
                                start: ByteIndex(text.floor_char_boundary(offset)),
                                end: ByteIndex(text.floor_char_boundary(offset + token_length)),
                            },
                            format: TextFormat {
                                font_id: font_id.clone(),
                                color: token_color,
                                valign: ui.text_valign(),
                                ..Default::default()
                            },
                        });
                        offset += token_length;
                    }

                    LayoutJob {
                        text,
                        sections,
                        break_on_newline: true,
                        ..Default::default()
                    }
                }
            }
            InnerNodeContent::Snippet(snippet) => {
                if snippet.is_empty() {
                    handle_empty()
                } else {
                    let text = from_utf8_lossy(snippet).to_string();
                    let text_length = text.len();

                    LayoutJob {
                        text,
                        sections: vec![LayoutSection {
                            leading_space: 0.0,
                            byte_range: ByteIndex(0)..ByteIndex(text_length),
                            format: TextFormat {
                                font_id,
                                color: node_color,
                                valign: ui.text_valign(),
                                ..Default::default()
                            },
                        }],
                        break_on_newline: true,
                        ..Default::default()
                    }
                }
            }
            InnerNodeContent::MetadataOnly => handle_empty(),
        }
    }
    pub fn node_color(&mut self, node: &TapestryNode) -> Option<Color32> {
        match self.settings.node_colors {
            NodeColors::None => None,
            NodeColors::Creator => node
                .contents
                .creator
                .color()
                .and_then(|h| Color32::from_hex(h).ok()),
        }
    }
    pub fn token_intensity(&mut self, token: &InnerNodeToken) -> f32 {
        let logprob_intensity = match token.logprob {
            Some(logprob) if logprob.is_finite() => {
                Some(1.0 - (f32::ln(1.0 / logprob.exp().clamp(f32::EPSILON, 1.0)) / 10.0))
            }
            _ => None,
        };
        let confidence_intensity =
            token
                .calculate_confidence()
                .map(|(confidence, confidence_k)| {
                    f32::ln(1.0 / (-(confidence)).exp().clamp(f32::EPSILON, 1.0))
                        / (f32::ln(confidence_k as f32) + 2.0)
                });

        match self.settings.token_colors {
            TokenColors::None => 1.0,
            TokenColors::Logprob => logprob_intensity.unwrap_or(1.0),
            TokenColors::Confidence => confidence_intensity.unwrap_or(1.0),
            TokenColors::HybridLogprobConfidence => {
                let confidence_intensity = confidence_intensity.unwrap_or(1.0);

                match logprob_intensity {
                    Some(logprob_intensity) => confidence_intensity.min(logprob_intensity),
                    None => confidence_intensity,
                }
            }
            TokenColors::Entropy => todo!(),
        }
    }
    pub fn token_color(&mut self, node_color: Color32, token: &InnerNodeToken) -> Color32 {
        let intensity = self.token_intensity(token);
        if intensity == 1.0 {
            node_color
        } else {
            multiply_color_alpha(
                node_color,
                intensity.clamp(self.settings.min_token_opacity, 1.0),
            )
        }
    }
    pub fn node_context_menu(
        &mut self,
        weave: &mut LoggedTapestryWeave,
        node: &TapestryNode,
        ui: &mut Ui,
        collapsing: bool,
        user: &Option<Author>,
    ) {
        let style = ui.style_mut();
        style.wrap_mode = Some(TextWrapMode::Extend);

        let is_modifier_pressed = ui.input(|input| input.modifiers.any());

        let generate_response = ui.button("Generate completions");
        if generate_response.clicked() {
            self.generate = Some(node.id);

            if generate_response.clicked_with_open_in_background() {
                weave.set_active_tree_semantics(&node.id, true);
                self.cursor = Some(node.id);
            }

            self.opened.insert(node.id, true);
        }

        if ui
            .button(if node.bookmarked {
                "Remove bookmark"
            } else {
                "Bookmark"
            })
            .clicked()
        {
            weave.set_bookmarked(&node.id, !node.bookmarked);
        };

        ui.separator();

        let add_child_response = ui.button(if !is_modifier_pressed || node.active {
            "Create child"
        } else {
            "Create active child"
        });
        if add_child_response.clicked() {
            let identifier = self.generate_id(weave);
            let active = add_child_response.clicked_with_open_in_background() || node.active;

            if weave.insert_deduplicated(Self::new_user_node(identifier, [node.id], active, user)) {
                if active {
                    self.cursor = Some(identifier);
                }

                self.opened.insert(node.id, true);
            }
        };

        let add_sibling_response = ui.button(if !is_modifier_pressed {
            "Create sibling"
        } else {
            "Create active sibling"
        });
        if add_sibling_response.clicked() {
            let identifier = self.generate_id(weave);
            let active = add_sibling_response.clicked_with_open_in_background();

            if weave.insert_deduplicated(Self::new_user_node(
                identifier,
                node.from.iter().copied(),
                active,
                user,
            )) && active
            {
                self.cursor = Some(identifier);
            }
        }

        ui.separator();

        if !node.to.is_empty() {
            if collapsing {
                if ui.button("Collapse all children").clicked() {
                    for child in node.to.iter().copied() {
                        self.opened.insert(child, false);
                    }
                }

                if ui.button("Expand all children").clicked() {
                    for child in node.to.iter().copied() {
                        self.opened.insert(child, true);
                    }
                }

                ui.separator();
            }

            SubMenuButton::new("Sort children by...").ui(ui, |ui| {
                if ui.button("Seriation").clicked() {
                    self.seriate = Some(node.id);
                }

                if ui.button("Confidence").clicked() {
                    weave.sort_children_by(&node.id, sort::by_confidence);
                }

                if ui.button("Metadata").clicked() {
                    weave.sort_children_by(&node.id, sort::grouped);
                }
            });

            ui.separator();

            if ui.button("Delete all children").clicked() {
                for child in &node.to {
                    weave.remove(child);
                }
            }
        }

        if ui.button("Delete all siblings").clicked() {
            let siblings: Vec<ShortId> = weave
                .get_siblings(&node.id, false)
                .map(Iterator::collect)
                .unwrap_or_default();

            for sibling in siblings {
                weave.remove(&sibling);
            }
        }

        if weave.is_mergeable_with_parent(&node.id) && ui.button("Merge with parent").clicked() {
            weave.merge_with_parent(&node.id);
        }

        ui.separator();

        if ui.button("Delete").clicked() {
            weave.remove(&node.id);
        }
    }
    pub fn document_context_menu(
        &mut self,
        weave: &mut LoggedTapestryWeave,
        ui: &mut Ui,
        flags: FlagSet<DocumentContextFlags>,
        user: &Option<Author>,
    ) {
        let style = ui.style_mut();
        style.wrap_mode = Some(TextWrapMode::Extend);

        let is_modifier_pressed = ui.input(|input| input.modifiers.any());

        if flags.contains(DocumentContextFlags::Roots) {
            let add_child_response =
                ui.button(if !is_modifier_pressed || weave.roots().is_empty() {
                    "Create root"
                } else {
                    "Create active root"
                });
            if add_child_response.clicked() {
                let identifier = self.generate_id(weave);
                let active = add_child_response.clicked_with_open_in_background()
                    || weave.roots().is_empty();

                if weave.insert_deduplicated(Self::new_user_node(identifier, [], active, user))
                    && active
                {
                    self.cursor = Some(identifier);
                }
            };

            if !weave.roots().is_empty() {
                ui.separator();

                SubMenuButton::new("Sort roots by...").ui(ui, |ui| {
                    if ui.button("Seriation").clicked() {
                        // TODO
                    }

                    if ui.button("Confidence").clicked() {
                        weave.sort_roots_by(sort::by_confidence);
                    }

                    if ui.button("Metadata").clicked() {
                        weave.sort_roots_by(sort::grouped);
                    }
                });
            }
        }

        if flags.contains(DocumentContextFlags::Bookmarks) {
            if flags.contains(DocumentContextFlags::Roots) {
                ui.separator();
            }

            SubMenuButton::new("Sort bookmarks by...").ui(ui, |ui| {
                if ui.button("Seriation").clicked() {
                    // TODO
                }

                if ui.button("Confidence").clicked() {
                    weave.sort_bookmarks_by(sort::by_confidence);
                }

                if ui.button("Metadata").clicked() {
                    weave.sort_bookmarks_by(sort::grouped);
                }
            });
        }

        // TODO
    }
    pub fn node_tooltip(&mut self, node: &TapestryNode, ui: &mut Ui) {
        ui.set_max_width(ui.spacing().tooltip_width);

        match &node.contents.creator {
            Creator::Model(Some(model)) => {
                let color = model.color.as_ref().and_then(|h| Color32::from_hex(h).ok());

                if let Some(color) = color {
                    ui.colored_label(color, &model.label);
                } else {
                    ui.label(&model.label);
                }

                #[cfg(debug_assertions)]
                if let Some(identifier) = model.identifier {
                    ui.weak(identifier.to_string());
                }
            }
            Creator::Model(None) => {
                ui.label(UNKNOWN_MODEL_LABEL);
            }
            Creator::User(Some(user)) => {
                let color = user.color.as_ref().and_then(|h| Color32::from_hex(h).ok());
                let text = RichText::new(format!("[USER] {}", user.label)).small();

                if let Some(color) = color {
                    ui.label(text.color(color));
                } else {
                    ui.label(text);
                }

                #[cfg(debug_assertions)]
                if let Some(identifier) = user.identifier {
                    ui.weak(identifier.to_string());
                }
            }
            Creator::User(None) => {
                ui.label(RichText::new("USER").small());
            }
            Creator::Unknown => {}
        }

        if node.contents.creator.is_model() && node.contents.modified {
            ui.colored_label(ui.visuals().warn_fg_color, "modified: true");
        }

        for (key, value) in &node.contents.metadata {
            ui.label(format!("{key}: {value}"));
        }

        if let Creator::Model(Some(model)) = &node.contents.creator {
            if let Some(seed) = model.seed {
                ui.label(format!("seed: {}", seed));
            }
            if let Some(finish_reason) = &model.finish_reason {
                ui.label(format!("finish_reason: {}", finish_reason));
            }
            if let Some(system_fingerprint) = &model.system_fingerprint {
                ui.label(format!("system_fingerprint: {}", system_fingerprint));
            }
        }

        if let Some(mean_logprob) = node.contents.content.calculate_average_logprob()
            && let Some(cum_logprob) = node.contents.content.calculate_cumulative_logprob()
            && let Some(tokens) = node.contents.content.token_count()
            && mean_logprob.is_finite()
            && cum_logprob.is_finite()
        {
            if ui.input(|i| i.modifiers.any()) {
                ui.label(format!(
                    "logprobs: (μ = {:.4} ({:.2}%), sum = {:.4}, n = {})",
                    mean_logprob,
                    mean_logprob.exp() * 100.0,
                    cum_logprob,
                    tokens
                ));
            }

            if let Some(mean_entropy) = node.contents.content.calculate_average_entropy()
                && mean_entropy.is_finite()
            {
                ui.label(format!("entropy: (μ = {:.4})", mean_entropy));
            }

            if let Some((confidence, confidence_k, confidence_n)) =
                node.contents.content.calculate_confidence()
                && confidence.is_finite()
            {
                ui.label(format!(
                    "confidence: {:.2} (k = {}, n = {})",
                    confidence, confidence_k, confidence_n
                ));
            }
        }

        ui.label(
            node.contents
                .timestamp
                .strftime("%m/%d/%Y %I:%M:%S %p")
                .to_string(),
        );

        #[cfg(debug_assertions)]
        ui.weak(node.id.to_string());
    }
    pub fn token_tooltip(
        &mut self,
        token: &InnerNodeToken,
        ui: &mut Ui,
        flags: FlagSet<TokenTooltipFlags>,
    ) -> Option<usize> {
        if flags.contains(TokenTooltipFlags::Counterfactual)
            && token.counterfactual.len() > 1
            && !token.is_modified()
        {
            let mut choice = None;

            ScrollArea::horizontal().animated(false).show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (index, counterfactual) in token.counterfactual.iter().enumerate() {
                        let Some(logprob) = counterfactual.logprob.filter(|l| l.is_finite()) else {
                            continue;
                        };

                        if ui
                            .button(
                                if let Ok(string) = str::from_utf8(&counterfactual.bytes) {
                                    RichText::new(format!(
                                        "{string:#?}\n({:.2}%)",
                                        logprob.exp() * 100.0
                                    ))
                                } else {
                                    RichText::new(format!(
                                        "{:?}\n({:.2}%)",
                                        counterfactual.bytes,
                                        logprob.exp() * 100.0
                                    ))
                                }
                                .monospace(),
                            )
                            .on_hover_ui(|ui| {
                                ui.label(format!(
                                    "probability: {:.2}% [{:.4}]",
                                    logprob.exp() * 100.0,
                                    logprob
                                ));

                                if let Some(id) = counterfactual.id {
                                    ui.label(format!("id: {}", id));
                                }
                            })
                            .clicked()
                        {
                            choice = Some(index);
                        }
                    }
                })
            });

            if let Some(choice) = choice {
                ui.request_discard("Token selected");
                return Some(choice);
            }

            ui.separator();
        }

        if !token.is_modified() {
            if flags.contains(TokenTooltipFlags::Contents) {
                ui.label(
                    if let Ok(string) = str::from_utf8(&token.bytes) {
                        RichText::new(format!("{string:#?}"))
                    } else {
                        RichText::new(format!("{:?}", token.bytes))
                    }
                    .monospace(),
                );
            }
        } else {
            if flags.contains(TokenTooltipFlags::WarnModified) {
                ui.colored_label(ui.visuals().warn_fg_color, "modified: true");
            }
            if flags.contains(TokenTooltipFlags::Counterfactual)
                && let OriginalToken::Known {
                    bytes: original, ..
                } = &token.original
                && original != &token.bytes
            {
                ui.label(
                    if let Ok(string) = str::from_utf8(original) {
                        RichText::new(format!("original: {string:#?}"))
                    } else {
                        RichText::new(format!("original: {:?}", original))
                    }
                    .monospace(),
                ); // TODO: click on original token to restore it
            }
        }

        if let Some(logprob) = token.logprob
            && logprob.is_finite()
        {
            ui.label(format!(
                "probability: {:.2}% [{:.4}]",
                logprob.exp() * 100.0,
                logprob
            ));
        }

        if let Some(entropy) = token.entropy
            && entropy.is_finite()
        {
            ui.label(format!("entropy: {:.4}", entropy));
        }

        if let Some((confidence, confidence_k)) = token.calculate_confidence()
            && confidence.is_finite()
        {
            ui.label(format!(
                "confidence: {:.2} (k = {})",
                confidence, confidence_k
            ));
        }

        if let Some(id) = token.id {
            ui.label(format!("id: {}", id));
        }

        None
    }
    pub fn node_buttons(
        &mut self,
        weave: &mut LoggedTapestryWeave,
        node: &TapestryNode,
        ui: &mut Ui,
        flags: FlagSet<ButtonFlags>,
        user: &Option<Author>,
    ) {
        let is_modifier_pressed = ui.input(|input| input.modifiers.any());

        if flags.contains(ButtonFlags::Rtl) {
            if flags.contains(ButtonFlags::Collapse) {
                let is_open = self.opened.get(&node.id).copied().unwrap_or(DEFAULT_OPEN);

                let label = if is_open { "\u{E43C}" } else { "\u{E43E}" };
                let hover_text = if is_open {
                    "Collapse node"
                } else {
                    "Expand node"
                };
                if ui.button(label).on_hover_text(hover_text).clicked() {
                    self.opened.insert(node.id, !is_open);
                };
            }

            if flags.contains(ButtonFlags::Delete)
                && ui.button("\u{E28F}").on_hover_text("Delete node").clicked()
            {
                weave.remove(&node.id);
            };

            if flags.contains(ButtonFlags::Bookmark) {
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
                    weave.set_bookmarked(&node.id, !node.bookmarked);
                };
            }

            if flags.contains(ButtonFlags::Add) {
                let add_response =
                    ui.button("\u{E40C}")
                        .on_hover_text(if !is_modifier_pressed || node.active {
                            "Add node"
                        } else {
                            "Add active node"
                        });
                if add_response.clicked() {
                    let identifier = self.generate_id(weave);
                    let active = add_response.clicked_with_open_in_background() || node.active;

                    if weave.insert_deduplicated(Self::new_user_node(
                        identifier,
                        [node.id],
                        active,
                        user,
                    )) {
                        if active {
                            self.cursor = Some(identifier);
                        }

                        self.opened.insert(node.id, true);
                    }
                };
            }

            if flags.contains(ButtonFlags::Generate) {
                let generate_response =
                    ui.button("\u{E5CE}")
                        .on_hover_text(if !is_modifier_pressed {
                            "Generate completions"
                        } else {
                            "Generate completions & focus node"
                        });
                if generate_response.clicked() {
                    self.generate = Some(node.id);

                    if generate_response.clicked_with_open_in_background() {
                        weave.set_active_tree_semantics(&node.id, true);
                        self.cursor = Some(node.id);
                    }

                    self.opened.insert(node.id, true);
                }
            }

            if flags.contains(ButtonFlags::Merge)
                && weave.is_mergeable_with_parent(&node.id)
                && ui
                    .button("\u{E43F}")
                    .on_hover_text("Merge node with parent")
                    .clicked()
            {
                weave.merge_with_parent(&node.id);
            };

            if flags.contains(ButtonFlags::Hoist)
                && node.from.len() == 1
                && ui
                    .button("\u{E042}")
                    .on_hover_text("Show parents")
                    .clicked()
            {
                self.cursor = Some(node.from.first().copied().unwrap());
            };
        } else {
            if flags.contains(ButtonFlags::Hoist)
                && node.from.len() == 1
                && ui
                    .button("\u{E042}")
                    .on_hover_text("Show parents")
                    .clicked()
            {
                self.cursor = Some(node.from.first().copied().unwrap());
            };

            if flags.contains(ButtonFlags::Merge)
                && weave.is_mergeable_with_parent(&node.id)
                && ui
                    .button("\u{E43F}")
                    .on_hover_text("Merge node with parent")
                    .clicked()
            {
                weave.merge_with_parent(&node.id);
            };

            if flags.contains(ButtonFlags::Generate) {
                let generate_response =
                    ui.button("\u{E5CE}")
                        .on_hover_text(if !is_modifier_pressed {
                            "Generate completions"
                        } else {
                            "Generate completions & focus node"
                        });
                if generate_response.clicked() {
                    self.generate = Some(node.id);

                    if generate_response.clicked_with_open_in_background() {
                        weave.set_active_tree_semantics(&node.id, true);
                        self.cursor = Some(node.id);
                    }

                    self.opened.insert(node.id, true);
                }
            }

            if flags.contains(ButtonFlags::Add) {
                let add_response = ui
                    .button("\u{E40C}")
                    .on_hover_text(if !is_modifier_pressed {
                        "Add node"
                    } else {
                        "Add active node"
                    });
                if add_response.clicked() {
                    let identifier = self.generate_id(weave);
                    let active = if add_response.clicked_with_open_in_background() {
                        true
                    } else {
                        node.active
                    };

                    if weave.insert_deduplicated(Self::new_user_node(
                        identifier,
                        [node.id],
                        active,
                        user,
                    )) {
                        if active {
                            self.cursor = Some(identifier);
                        }

                        self.opened.insert(node.id, true);
                    }
                };
            }

            if flags.contains(ButtonFlags::Bookmark) {
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
                    weave.set_bookmarked(&node.id, !node.bookmarked);
                };
            }

            if flags.contains(ButtonFlags::Delete)
                && ui.button("\u{E28F}").on_hover_text("Delete node").clicked()
            {
                weave.remove(&node.id);
            };

            if flags.contains(ButtonFlags::Collapse) {
                let is_open = self.opened.get(&node.id).copied().unwrap_or(DEFAULT_OPEN);

                let label = if is_open { "\u{E43C}" } else { "\u{E43E}" };
                let hover_text = if is_open {
                    "Collapse node"
                } else {
                    "Expand node"
                };
                if ui.button(label).on_hover_text(hover_text).clicked() {
                    self.opened.insert(node.id, !is_open);
                };
            }
        }
    }
}

pub struct LabelOptions {
    pub buttons: FlagSet<ButtonFlags>,
    pub collapsing: bool,
    pub show_info: bool,
    pub autoscroll: Option<AutoscrollData>,
}

#[derive(Debug, Clone, Copy)]
pub struct AutoscrollData {
    max_autoscroll_height: f32,
}

flags! {
    pub enum ButtonFlags: u8 {
        Rtl,
        Hoist,
        Merge,
        Generate,
        Add,
        Bookmark,
        Delete,
        Collapse,
    }
    pub enum TextFlags: u8 {
        EmptyNotice,
        FirstTokenBytes,
    }
    pub enum TokenTooltipFlags: u8 {
        WarnModified,
        Counterfactual,
        Contents,
    }
    pub enum DocumentContextFlags: u8 {
        Roots,
        Bookmarks,
    }
}
