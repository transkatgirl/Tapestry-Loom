use eframe::egui::{Frame, ScrollArea, Spinner, TextEdit, Ui};
use egui_notify::Toasts;
use flagset::FlagSet;
use tapestry_weave::universal_weave::indexmap::IndexMap;

use crate::{
    editor::shared::{SharedState, weave::WeaveWrapper},
    format_file_size, format_large_number,
    settings::{Settings, inference::render_config_map, shortcuts::Shortcuts},
};

#[derive(Default, Debug)]
pub struct MenuView {
    active_node_count: usize,
}

impl MenuView {
    /*pub fn reset(&mut self) {
        self.active_node_count = 0;
    }*/
    pub fn render(
        &mut self,
        ui: &mut Ui,
        _weave: &mut WeaveWrapper,
        settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
        _shortcuts: FlagSet<Shortcuts>,
    ) {
        ScrollArea::vertical()
            .auto_shrink(false)
            .animated(false)
            .show(ui, |ui| {
                Frame::new()
                    .outer_margin(ui.style().spacing.menu_margin)
                    .show(ui, |ui| {
                        state.inference.render(&settings.inference, ui);
                    });
            });
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render_rtl_panel(
        &mut self,
        ui: &mut Ui,
        weave: &mut WeaveWrapper,
        settings: &Settings,
        _toasts: &mut Toasts,
        state: &mut SharedState,
        shortcuts: FlagSet<Shortcuts>,
        file_size: usize,
    ) {
        let request_count = state.get_request_count();

        if state.has_weave_changed || self.active_node_count == 0 {
            self.active_node_count = weave.get_active_thread_len();
        }

        if request_count > 0 {
            ui.add(Spinner::new());
            if request_count > 1 {
                ui.label(format!("{request_count} requests"))
            } else {
                ui.label("1 request")
            }
            .on_hover_ui(|ui| {
                if ui.button("Cancel requests").clicked() {
                    state.cancel_requests();
                }
            });
        } else {
            let node_count = weave.len();
            let bookmarked_node_count = weave.get_bookmarks().len();
            let label = ui.label(if bookmarked_node_count > 0 {
                format!(
                    "{}, {}, {}",
                    format_large_number(node_count, "node", "nodes"),
                    format_large_number(self.active_node_count, "active", "active"),
                    format_large_number(bookmarked_node_count, "bookmarked", "bookmarked"),
                )
            } else {
                format!(
                    "{}, {}",
                    format_large_number(node_count, "node", "nodes"),
                    format_large_number(self.active_node_count, "active", "active")
                )
            });

            if file_size > 0 {
                label.on_hover_ui(|ui| {
                    ui.label(format_file_size(file_size));
                });
            }
        }

        if shortcuts.contains(Shortcuts::ParameterPreset10) {
            state.inference.switch_preset(&settings.inference, 10);
        }

        if shortcuts.contains(Shortcuts::ParameterPreset9) {
            state.inference.switch_preset(&settings.inference, 9);
        }

        if shortcuts.contains(Shortcuts::ParameterPreset8) {
            state.inference.switch_preset(&settings.inference, 8);
        }

        if shortcuts.contains(Shortcuts::ParameterPreset7) {
            state.inference.switch_preset(&settings.inference, 7);
        }

        if shortcuts.contains(Shortcuts::ParameterPreset6) {
            state.inference.switch_preset(&settings.inference, 6);
        }

        if shortcuts.contains(Shortcuts::ParameterPreset5) {
            state.inference.switch_preset(&settings.inference, 5);
        }

        if shortcuts.contains(Shortcuts::ParameterPreset4) {
            state.inference.switch_preset(&settings.inference, 4);
        }

        if shortcuts.contains(Shortcuts::ParameterPreset3) {
            state.inference.switch_preset(&settings.inference, 3);
        }

        if shortcuts.contains(Shortcuts::ParameterPreset2) {
            state.inference.switch_preset(&settings.inference, 2);
        }

        if shortcuts.contains(Shortcuts::ParameterPreset1) {
            state.inference.switch_preset(&settings.inference, 1);
        }

        if shortcuts.contains(Shortcuts::ResetParameters) {
            state.inference.reset(&settings.inference);
        }
    }
}

#[derive(Default, Debug)]
pub struct InfoView {}

impl InfoView {
    //pub fn reset(&mut self) {}
    pub fn update(
        &mut self,
        _weave: &mut WeaveWrapper,
        _settings: &Settings,
        _toasts: &mut Toasts,
        _state: &mut SharedState,
        _shortcuts: FlagSet<Shortcuts>,
    ) {
    }
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut WeaveWrapper,
        _settings: &Settings,
        _toasts: &mut Toasts,
        _state: &mut SharedState,
        _shortcuts: FlagSet<Shortcuts>,
    ) {
        ScrollArea::vertical()
            .auto_shrink(false)
            .animated(false)
            .show(ui, |ui| {
                Frame::new()
                    .outer_margin(ui.style().spacing.menu_margin)
                    .show(ui, |ui| {
                        if let Some(notes) = weave.metadata_mut().get_mut("notes") {
                            ui.group(|ui| {
                                let label = ui.label("Notes:").id;

                                TextEdit::multiline(notes)
                                    .desired_width(ui.spacing().text_edit_width * 2.0)
                                    .lock_focus(true)
                                    .show(ui)
                                    .response
                                    .labelled_by(label);
                            });
                        }

                        ui.group(|ui| {
                            ui.label("Metadata:");

                            let mut metadata = weave
                                .metadata()
                                .iter()
                                .filter(|(k, _)| *k != "notes")
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect();

                            render_config_map(ui, &mut metadata, 0.9, 1.1);

                            metadata.push((
                                "notes".to_string(),
                                weave.metadata().get("notes").cloned().unwrap_or_default(),
                            ));

                            *weave.metadata_mut() = IndexMap::from_iter(metadata);
                        });
                    });
            });
    }
}
