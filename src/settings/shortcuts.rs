use eframe::egui::Context;
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
    pub editor: EditorShortcutSettings,
}

impl ShortcutSettings {
    pub fn clear(shortcuts: &mut Shortcuts) {
        shortcuts.global = FlagSet::<GlobalShortcuts>::empty();
        EditorShortcutSettings::clear(&mut shortcuts.editor);
    }
    pub fn update(&mut self, shortcuts: &mut Shortcuts, ctx: &Context) {
        shortcuts.global = FlagSet::<GlobalShortcuts>::empty();

        // TODO

        self.editor.update(&mut shortcuts.editor, ctx);
    }
}

impl Edit for ShortcutSettings {
    fn ui(&mut self, ui: &mut eframe::egui::Ui) {
        // TODO
        self.editor.ui(ui);
    }
}

#[derive(Default, Debug)]
pub struct Shortcuts {
    pub global: FlagSet<GlobalShortcuts>,
    pub editor: FlagSet<EditorShortcuts>,
}

flags! {
    pub enum GlobalShortcuts: u64 {}
}
