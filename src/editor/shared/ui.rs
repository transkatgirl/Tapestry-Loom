use std::collections::HashSet;

use eframe::egui::Ui;
use flagset::{FlagSet, flags};
use tapestry_weave::{
    jiff::Zoned,
    universal_weave::{dependent::DependentNode, indexmap::IndexSet},
    v1::{
        content::{Creator, InnerNodeContent, NodeContent},
        dependent::{TapestryNode, TapestryWeave},
        metadata::{AuxMetadataMap, MetadataMap},
    },
};

use crate::editor::settings::interface::InterfaceSettings;

#[derive(Default)]
pub struct WeaveUi {
    cursor: Option<u64>,
    hovered: Option<u64>,
    collapsed: HashSet<u64>,
}

impl WeaveUi {
    fn is_collapsed(&mut self, weave: &mut TapestryWeave, id: u64) -> bool {
        todo!()
    }
    fn set_collapsed(&mut self, weave: &mut TapestryWeave, id: u64, collapsed: bool) {
        // TODO
    }
    pub fn horizontal_node_label(
        &mut self,
        weave: &mut TapestryWeave,
        node: u64,
        ui: &mut Ui,
        settings: &InterfaceSettings,
        options: &LabelOptions,
    ) {
    }
    pub fn node_context_menu(
        &mut self,
        weave: &mut TapestryWeave,
        node: &TapestryNode,
        ui: &mut Ui,
        settings: &InterfaceSettings,
        flags: FlagSet<ContextMenuFlags>,
    ) {
        // TODO
    }
    pub fn node_buttons(
        &mut self,
        weave: &mut TapestryWeave,
        node: &TapestryNode,
        ui: &mut Ui,
        settings: &InterfaceSettings,
        flags: FlagSet<ButtonFlags>,
    ) {
        let is_modifier_pressed = ui.input(|input| input.modifiers.any());

        if flags.contains(ButtonFlags::Rtl) {
            // TODO
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
                    // TODO
                    //state.generate_children(weave, Some(Ulid(node.id)), settings);

                    if generate_response.clicked_with_open_in_background() {
                        weave.set_node_active_status(&node.id, true, false);
                        self.cursor = Some(node.id);
                    }

                    self.set_collapsed(weave, node.id, false);
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
                            creator: Creator::User(None), // TODO
                        },
                    }) {
                        if active {
                            self.cursor = Some(identifier);
                        } else {
                            self.set_collapsed(weave, identifier, false);
                        }
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
                let is_collapsed = self.is_collapsed(weave, node.id);

                let label = if is_collapsed { "\u{E43E}" } else { "\u{E43C}" };
                let hover_text = if is_collapsed {
                    "Expand node"
                } else {
                    "Collapse node"
                };
                if ui.button(label).on_hover_text(hover_text).clicked() {
                    self.set_collapsed(weave, node.id, !is_collapsed);
                };
            }
        }
    }
}

pub struct LabelOptions {
    buttons: FlagSet<ButtonFlags>,
    context_menu: FlagSet<ContextMenuFlags>,
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
    pub enum ContextMenuFlags: u8 {

    }
}
