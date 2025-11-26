use eframe::egui::{Frame, Id, ScrollArea, TextEdit, TextStyle, Ui};
use egui_notify::Toasts;
use tapestry_weave::v0::TapestryWeave;

use crate::{editor::shared::SharedState, settings::Settings};

#[derive(Default, Debug)]
pub struct TextEditorView {}

impl TextEditorView {
    pub fn reset(&mut self) {}
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut TapestryWeave,
        settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
    ) {
        /*ScrollArea::vertical()
        .auto_shrink(false)
        .animated(false)
        .show(ui, |ui| {
            let textedit = TextEdit::multiline(&mut text)
                .frame(false)
                .text_color(ui.visuals().widgets.inactive.text_color())
                .min_size(ui.available_size())
                .desired_width(ui.available_size().x)
                .code_editor();

            if ui.add(textedit).changed() {};
        });*/
    }
}
