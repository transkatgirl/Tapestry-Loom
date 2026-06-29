use eframe::egui::Context;
use flagset::{FlagSet, flags};
use serde::{Deserialize, Serialize};

use crate::common::view::Edit;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct ShortcutSettings {}

impl ShortcutSettings {
    pub fn clear(shortcuts: &mut FlagSet<Shortcuts>) {
        *shortcuts = FlagSet::<Shortcuts>::empty();
    }
    pub fn update(&mut self, shortcuts: &mut FlagSet<Shortcuts>, ctx: &Context) {
        *shortcuts = FlagSet::<Shortcuts>::empty();
    }
}

impl Edit for ShortcutSettings {
    fn ui(&mut self, ui: &mut eframe::egui::Ui) {}
}

flags! {
    pub enum Shortcuts: u64 {}
}
