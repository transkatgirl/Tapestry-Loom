use eframe::egui::{Context, KeyboardShortcut, TextStyle, Ui};
use flagset::{FlagSet, flags};
use serde::{Deserialize, Serialize};

use crate::{
    common::{
        ui::{consume_shortcut, shortcut_ui},
        view::Edit,
    },
    editor::settings::shortcuts::{
        ShortcutSettings as EditorShortcutSettings, Shortcuts as EditorShortcuts,
    },
};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct ShortcutSettings {
    #[serde(default)]
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
        if ctx.memory(|memory| memory.top_modal_layer().is_some()) {
            ShortcutSettings::clear(shortcuts);
            return;
        }

        self.global.update(&mut shortcuts.global, ctx);
        self.editor.update(&mut shortcuts.editor, ctx);
    }
}

impl Edit for ShortcutSettings {
    fn ui(&mut self, ui: &mut Ui) {
        ui.label("Press escape to clear a keybind.");
        self.editor.ui(ui);
        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.5);
        self.global.ui(ui);
    }
}

#[derive(Default, Debug)]
pub struct Shortcuts {
    pub global: FlagSet<GlobalShortcuts>,
    pub editor: FlagSet<EditorShortcuts>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct GlobalShortcutSettings {
    #[serde(default)]
    save: Option<KeyboardShortcut>,
}

impl GlobalShortcutSettings {
    pub fn clear(shortcuts: &mut FlagSet<GlobalShortcuts>) {
        *shortcuts = FlagSet::<GlobalShortcuts>::empty();
    }
    pub fn update(&mut self, shortcuts: &mut FlagSet<GlobalShortcuts>, ctx: &Context) {
        Self::clear(shortcuts);

        ctx.input_mut(|input| {
            if let Some(shortcut) = &self.save
                && consume_shortcut(input, shortcut)
            {
                *shortcuts |= GlobalShortcuts::Save;
            }
        });
    }
}

impl Edit for GlobalShortcutSettings {
    fn ui(&mut self, ui: &mut Ui) {
        shortcut_ui(ui, &mut self.save, "keybind-save_all", "Save immediately");
    }
}

flags! {
    pub enum GlobalShortcuts: u32 {
        Save,
    }
}
