use eframe::egui::{Frame, ScrollArea, Spinner, TextEdit, Ui};
use egui_notify::Toasts;
use flagset::FlagSet;
use tapestry_weave::universal_weave::indexmap::IndexMap;

use crate::{
    editor::shared::{SharedState, weave::WeaveWrapper},
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
                ui.label(format!("{request_count} requests"));
            } else {
                ui.label("1 request");
            }
        } else {
            let node_count = weave.len();
            let bookmarked_node_count = weave.get_bookmarks().len();
            let node_count_label = if node_count >= 100_000_000 {
                format!("{:.0}M nodes", node_count as f32 / 1_000_000.0)
            } else if node_count >= 1_000_000 {
                format!("{:.1}M nodes", node_count as f32 / 1_000_000.0)
            } else if node_count >= 100_000 {
                format!("{:.0}k nodes", node_count as f32 / 1_000.0)
            } else if node_count >= 1_000 {
                format!("{:.1}k nodes", node_count as f32 / 1_000.0)
            } else if node_count == 1 {
                "1 node".to_string()
            } else {
                format!("{} nodes", node_count)
            };
            let active_node_count_label = if self.active_node_count >= 100_000_000 {
                format!("{:.0}M active", self.active_node_count as f32 / 1_000_000.0)
            } else if self.active_node_count >= 1_000_000 {
                format!("{:.1}M active", self.active_node_count as f32 / 1_000_000.0)
            } else if self.active_node_count >= 100_000 {
                format!("{:.0}k active", self.active_node_count as f32 / 1_000.0)
            } else if self.active_node_count >= 1_000 {
                format!("{:.1}k active", self.active_node_count as f32 / 1_000.0)
            } else if self.active_node_count == 1 {
                "1 active".to_string()
            } else {
                format!("{} active", self.active_node_count)
            };
            let bookmarked_node_count_label = if bookmarked_node_count >= 100_000_000 {
                format!(
                    "{:.0}M bookmarked",
                    bookmarked_node_count as f32 / 1_000_000.0
                )
            } else if bookmarked_node_count >= 1_000_000 {
                format!(
                    "{:.1}M bookmarked",
                    bookmarked_node_count as f32 / 1_000_000.0
                )
            } else if bookmarked_node_count >= 100_000 {
                format!("{:.0}k bookmarked", bookmarked_node_count as f32 / 1_000.0)
            } else if bookmarked_node_count >= 1_000 {
                format!("{:.1}k bookmarked", bookmarked_node_count as f32 / 1_000.0)
            } else if bookmarked_node_count == 1 {
                "1 bookmarked".to_string()
            } else {
                format!("{} bookmarked", bookmarked_node_count)
            };
            let label = ui.label(if bookmarked_node_count > 0 {
                format!(
                    "{node_count_label}, {active_node_count_label}, {bookmarked_node_count_label}"
                )
            } else {
                format!("{node_count_label}, {active_node_count_label}")
            });

            if file_size > 0 {
                label.on_hover_ui(|ui| {
                    let file_size_label = if file_size >= 100_000_000_000 {
                        format!("{:.0} GB", file_size as f32 / 1_000_000_000.0)
                    } else if file_size >= 1_000_000_000 {
                        format!("{:.1} GB", file_size as f32 / 1_000_000_000.0)
                    } else if file_size >= 100_000_000 {
                        format!("{:.0} MB", file_size as f32 / 1_000_000.0)
                    } else if file_size >= 1_000_000 {
                        format!("{:.1} MB", file_size as f32 / 1_000_000.0)
                    } else if file_size >= 100_000 {
                        format!("{:.0} kB", file_size as f32 / 1_000.0)
                    } else if file_size >= 1_000 {
                        format!("{:.1} kB", file_size as f32 / 1_000.0)
                    } else {
                        format!("{} bytes", file_size)
                    };

                    ui.label(file_size_label);
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
