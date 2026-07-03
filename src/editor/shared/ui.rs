use std::{
    cmp,
    collections::{HashMap, HashSet},
};

use eframe::egui::{Color32, Frame, Sense, Ui, UiBuilder, text::LayoutJob};
use egui_plot::Text;
use flagset::{FlagSet, flags};
use tapestry_weave::{
    jiff::Zoned,
    universal_weave::{dependent::DependentNode, indexmap::IndexSet},
    v1::{
        content::{Author, Creator, InnerNodeContent, InnerNodeToken, NodeContent},
        dependent::{TapestryNode, TapestryWeave},
        metadata::{AuxMetadataMap, MetadataMap},
    },
};
use ulid::Ulid;

use crate::{
    common::ui::{from_opaque_oklch, into_oklch_opaque},
    editor::settings::interface::{InterfaceSettings, NodeColors, TokenColors},
    inference::InferenceEngine,
};

#[derive(Default)]
pub struct WeaveUi {
    cursor: Option<u64>,
    opened: HashMap<u64, bool>,
    hovered: Option<u64>,
    scroll_to: Option<u64>,

    generate: Option<u64>,
    seriate: Option<u64>,
}

const DEFAULT_OPEN: bool = false;

impl WeaveUi {
    pub fn logic(
        &mut self,
        weave: &mut TapestryWeave,
        settings: &InterfaceSettings,
        inference: &mut InferenceEngine,
        id: Ulid,
    ) {
        if let Some(generate) = self.generate {
            inference.generate_children(id, weave, generate);
        }

        if let Some(seriate) = self.seriate {
            inference.seriate_siblings(id, weave, seriate);
        }

        // TODO
    }
    pub fn horizontal_node_label(
        &mut self,
        weave: &mut TapestryWeave,
        node: &TapestryNode,
        ui: &mut Ui,
        settings: &InterfaceSettings,
        options: &LabelOptions,
        user: &Option<Author>,
    ) {
        let mut mouse_hovered = false;

        let response = ui
            .scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
                let mut frame = Frame::new();

                let is_hovered = self.hovered == Some(node.id);
                let is_cursor = self.cursor == Some(node.id);
                let is_focus = self.scroll_to == Some(node.id);

                if is_hovered {
                    frame = frame.fill(ui.visuals().widgets.hovered.weak_bg_fill);
                }

                frame.show(ui, |ui| {
                    // TODO
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
            weave.set_node_active_status(
                &node.id,
                true,
                response.clicked_with_open_in_background(),
            );
            self.cursor = Some(node.id);
        }
    }
    pub fn node_text(
        &mut self,
        ui: &Ui,
        node: &TapestryNode,
        settings: &InterfaceSettings,
        flags: FlagSet<TextFlags>,
    ) -> LayoutJob {
        let color = self.node_color(ui, node, settings);

        todo!()
    }
    pub fn node_color(
        &mut self,
        ui: &Ui,
        node: &TapestryNode,
        settings: &InterfaceSettings,
    ) -> Color32 {
        match settings.node_colors {
            NodeColors::None => None,
            NodeColors::Creator => node
                .contents
                .creator
                .color()
                .and_then(|h| Color32::from_hex(h).ok()),
        }
        .unwrap_or(ui.visuals().widgets.inactive.text_color())
    }
    pub fn token_intensity(&mut self, token: &InnerNodeToken, settings: &InterfaceSettings) -> f32 {
        match settings.token_colors {
            TokenColors::None => 1.0,
            TokenColors::Logprob => {
                1.0 - (f32::ln(1.0 / token.logprob.exp().clamp(f32::EPSILON, 1.0)) / 10.0)
            }
            TokenColors::Confidence => {
                if let Some((confidence, confidence_k)) = token.calculate_confidence() {
                    f32::ln(1.0 / (-(confidence)).exp().clamp(f32::EPSILON, 1.0))
                        / (f32::ln(confidence_k as f32) + 2.0)
                } else {
                    1.0
                }
            }
            TokenColors::HybridLogprobConfidence => {
                if let Some((confidence, confidence_k)) = token.calculate_confidence() {
                    f32::ln(1.0 / (-(confidence)).exp().clamp(f32::EPSILON, 1.0))
                        / (f32::ln(confidence_k as f32) + 2.0)
                } else {
                    1.0
                }
                .min(1.0 - (f32::ln(1.0 / token.logprob.exp().clamp(f32::EPSILON, 1.0)) / 10.0))
            }
            TokenColors::Entropy => todo!(),
        }
    }
    pub fn token_color(
        &mut self,
        node_color: Color32,
        token: &InnerNodeToken,
        settings: &InterfaceSettings,
    ) -> Color32 {
        let intensity = self.token_intensity(token, settings);
        if intensity == 1.0 {
            node_color
        } else {
            from_opaque_oklch(into_oklch_opaque(node_color).map_lightness(|l| l * intensity))
        }
    }
    pub fn node_context_menu(
        &mut self,
        weave: &mut TapestryWeave,
        node: &TapestryNode,
        ui: &mut Ui,
        collapsing: bool,
        user: &Option<Author>,
    ) {
        let is_modifier_pressed = ui.input(|input| input.modifiers.any());

        let generate_response = ui.button("Generate completions");
        if generate_response.clicked() {
            self.generate = Some(node.id);

            if generate_response.clicked_with_open_in_background() {
                weave.set_node_active_status(&node.id, true, false);
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
            weave.set_node_bookmarked_status(&node.id, !node.bookmarked);
        };

        ui.separator();

        let add_child_response = ui.button(if !is_modifier_pressed {
            "Create child"
        } else {
            "Create active child"
        });
        if add_child_response.clicked() {
            let identifier = weave.generate_id();
            let active = if add_child_response.clicked_with_open_in_background() {
                true
            } else {
                node.active
            };

            if weave.add_node(DependentNode {
                id: identifier,
                from: Some(node.id),
                to: IndexSet::default(),
                active,
                bookmarked: false,
                contents: NodeContent {
                    timestamp: Zoned::now(),
                    modified: false,
                    content: InnerNodeContent::MetadataOnly,
                    metadata: MetadataMap::default(),
                    aux_metadata: AuxMetadataMap::default(),
                    creator: Creator::User(user.clone()),
                },
            }) {
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
            let identifier = weave.generate_id();
            let active = if add_sibling_response.clicked_with_open_in_background() {
                true
            } else {
                node.active
            };

            if weave.add_node(DependentNode {
                id: identifier,
                from: node.from,
                to: IndexSet::default(),
                active,
                bookmarked: false,
                contents: NodeContent {
                    timestamp: Zoned::now(),
                    modified: false,
                    content: InnerNodeContent::MetadataOnly,
                    metadata: MetadataMap::default(),
                    aux_metadata: AuxMetadataMap::default(),
                    creator: Creator::User(user.clone()),
                },
            }) && active
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

            if ui.button("Seriate children").clicked() {
                self.seriate = Some(node.id);
            }

            if ui.button("Sort children by confidence").clicked() {
                // TODO
            }

            if ui.button("Sort children by timestamp").clicked() {
                // TODO
            }

            ui.separator();

            if ui.button("Delete all children").clicked() {
                for child in &node.to {
                    weave.remove_node(child);
                }
            }
        }

        if ui.button("Delete all siblings").clicked() {
            let siblings: Vec<u64> = weave
                .get_node_siblings(&node.id)
                .map(|i| i.collect())
                .unwrap_or_default();

            for sibling in siblings {
                weave.remove_node(&sibling);
            }
        }

        if weave.is_mergeable_with_parent(&node.id) && ui.button("Merge with parent").clicked() {
            weave.merge_with_parent(&node.id);
        }

        ui.separator();

        if ui.button("Delete").clicked() {
            weave.remove_node(&node.id);
        }
    }
    pub fn node_buttons(
        &mut self,
        weave: &mut TapestryWeave,
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
                weave.remove_node(&node.id);
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
                    weave.set_node_bookmarked_status(&node.id, !node.bookmarked);
                };
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
                    let identifier = weave.generate_id();
                    let active = if add_response.clicked_with_open_in_background() {
                        true
                    } else {
                        node.active
                    };

                    if weave.add_node(DependentNode {
                        id: identifier,
                        from: Some(node.id),
                        to: IndexSet::default(),
                        active,
                        bookmarked: false,
                        contents: NodeContent {
                            timestamp: Zoned::now(),
                            modified: false,
                            content: InnerNodeContent::MetadataOnly,
                            metadata: MetadataMap::default(),
                            aux_metadata: AuxMetadataMap::default(),
                            creator: Creator::User(user.clone()),
                        },
                    }) {
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
                        weave.set_node_active_status(&node.id, true, false);
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
                && let Some(parent) = node.from
                && ui
                    .button("\u{E042}")
                    .on_hover_text("Show parents")
                    .clicked()
            {
                self.cursor = Some(parent);
            };
        } else {
            if flags.contains(ButtonFlags::Hoist)
                && let Some(parent) = node.from
                && ui
                    .button("\u{E042}")
                    .on_hover_text("Show parents")
                    .clicked()
            {
                self.cursor = Some(parent);
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
                        weave.set_node_active_status(&node.id, true, false);
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
                    let identifier = weave.generate_id();
                    let active = if add_response.clicked_with_open_in_background() {
                        true
                    } else {
                        node.active
                    };

                    if weave.add_node(DependentNode {
                        id: identifier,
                        from: Some(node.id),
                        to: IndexSet::default(),
                        active,
                        bookmarked: false,
                        contents: NodeContent {
                            timestamp: Zoned::now(),
                            modified: false,
                            content: InnerNodeContent::MetadataOnly,
                            metadata: MetadataMap::default(),
                            aux_metadata: AuxMetadataMap::default(),
                            creator: Creator::User(user.clone()),
                        },
                    }) {
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
                    weave.set_node_bookmarked_status(&node.id, !node.bookmarked);
                };
            }

            if flags.contains(ButtonFlags::Delete)
                && ui.button("\u{E28F}").on_hover_text("Delete node").clicked()
            {
                weave.remove_node(&node.id);
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
    buttons: FlagSet<ButtonFlags>,
    collapsing: bool,
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
}
