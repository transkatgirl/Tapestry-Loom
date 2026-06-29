use eframe::egui::{Context, TextStyle, Ui};
use flagset::{FlagSet, flags};
use serde::{Deserialize, Serialize};

use crate::{
    common::view::Edit,
    editor::settings::shortcuts::{
        ShortcutSettings as EditorShortcutSettings, Shortcuts as EditorShortcuts,
    },
};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct ShortcutSettings {
    #[serde(flatten)]
    pub global: GlobalShortcutSettings,

    #[serde(flatten)]
    pub editor: EditorShortcutSettings,
}

impl ShortcutSettings {
    pub fn clear(shortcuts: &mut Shortcuts) {
        GlobalShortcutSettings::clear(&mut shortcuts.global);
        EditorShortcutSettings::clear(&mut shortcuts.editor);
    }
    pub fn update(&mut self, shortcuts: &mut Shortcuts, ctx: &Context) {
        self.global.update(&mut shortcuts.global, ctx);
        self.editor.update(&mut shortcuts.editor, ctx);
    }
}

impl Edit for ShortcutSettings {
    fn ui(&mut self, ui: &mut Ui) {
        self.global.ui(ui);
        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.5);
        self.editor.ui(ui);
    }
}

#[derive(Default, Debug)]
pub struct Shortcuts {
    pub global: FlagSet<GlobalShortcuts>,
    pub editor: FlagSet<EditorShortcuts>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct GlobalShortcutSettings {}

impl GlobalShortcutSettings {
    pub fn clear(shortcuts: &mut FlagSet<GlobalShortcuts>) {
        *shortcuts = FlagSet::<GlobalShortcuts>::empty();
    }
    pub fn update(&mut self, shortcuts: &mut FlagSet<GlobalShortcuts>, ctx: &Context) {
        Self::clear(shortcuts);

        // TODO
    }
}

impl Edit for GlobalShortcutSettings {
    fn ui(&mut self, ui: &mut Ui) {
        // TODO
    }
}

flags! {
    pub enum GlobalShortcuts: u32 {
        CloseFocusedTab,
        SaveAllDocuments,
    }
}
