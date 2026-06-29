use std::collections::HashSet;

use eframe::egui::Ui;
use flagset::{FlagSet, flags};
use tapestry_weave::v1::dependent::{TapestryNode, TapestryWeave};

use crate::editor::settings::{interface::InterfaceSettings, shortcuts::Shortcuts};

#[derive(Default)]
pub struct WeaveUi {
    cursor: Option<u64>,
    hovered: Option<u64>,
    collapsed: HashSet<u64>,
}

impl WeaveUi {
    fn horizontal_node_label_ui(
        &mut self,
        weave: &mut TapestryWeave,
        node: u64,
        ui: &mut Ui,
        settings: &InterfaceSettings,
        options: &LabelOptions,
    ) {
    }
    fn horizontal_node_buttons_ui(
        &mut self,
        weave: &mut TapestryWeave,
        node: &TapestryNode,
        ui: &mut Ui,
        settings: &InterfaceSettings,
        flags: FlagSet<ButtonFlags>,
    ) {
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

            // TODO
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
