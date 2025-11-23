#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use eframe::{
    App, CreationContext, Frame, NativeOptions,
    egui::{
        Button, CentralPanel, Context, FontDefinitions, IconData, SidePanel, TopBottomPanel,
        ViewportBuilder,
    },
};
use log::{debug, warn};

use crate::{editor::Editor, files::FileManager, settings::Settings};

mod editor;
mod files;
mod settings;

fn main() -> eframe::Result {
    env_logger::init();
    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_fullscreen(true)
            .with_icon(Arc::new(IconData::default())),
        persist_window: true,
        ..Default::default()
    };
    eframe::run_native(
        "Tapestry Loom",
        options,
        Box::new(|cc| Ok(Box::new(TapestryLoomApp::new(cc)))),
    )
}

struct TapestryLoomApp {
    show_settings: bool,
    show_file_manager: bool,
    file_manager: FileManager,
    editor: Editor,
    settings: Settings,
}

impl TapestryLoomApp {
    fn new(cc: &CreationContext<'_>) -> Self {
        let settings = if let Some(storage) = cc.storage {
            if let Some(data) = storage.get_string("settings") {
                match ron::from_str(&data) {
                    Ok(settings) => settings,
                    Err(error) => {
                        warn!("Unable to deserialize settings: {error:#?}\nUsing default settings");
                        Settings::default()
                    }
                }
            } else {
                debug!("Using default settings");
                Settings::default()
            }
        } else {
            warn!("Unable to connect to persistent storage; Using default settings");
            Settings::default()
        };

        let mut fonts = FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Fill);
        cc.egui_ctx.set_fonts(fonts);

        Self {
            show_settings: false,
            show_file_manager: false,
            file_manager: FileManager::new(settings.clone()),
            editor: Editor::new(settings.clone()),
            settings,
        }
    }
    fn save_settings(&self, frame: &mut Frame) {
        if let Some(storage) = frame.storage_mut() {
            match ron::to_string(&self.settings) {
                Ok(data) => {
                    debug!("Settings saved (may not yet be written to disk)");
                    storage.set_string("settings", data);
                }
                Err(error) => {
                    warn!("Unable to serialize settings: {error:#?}\n; Settings not saved");
                }
            }
        } else {
            warn!("Unable to connect to persistent storage; Settings not saved");
        }
    }
}

impl App for TapestryLoomApp {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        TopBottomPanel::top("global-top-panel").show(ctx, |ui| {
            let mut file_manager_button = Button::new("\u{E260} Files");
            let mut settings_button = Button::new("\u{E270} Settings");

            if self.show_file_manager {
                file_manager_button = file_manager_button.fill(ui.visuals().extreme_bg_color);
            }

            if self.show_settings {
                settings_button = settings_button.fill(ui.visuals().extreme_bg_color);
            }

            ui.horizontal_wrapped(|ui| {
                if ui.add(file_manager_button).clicked() {
                    self.show_file_manager = !self.show_file_manager
                };
                if ui.add(settings_button).clicked() {
                    self.show_settings = !self.show_settings
                };
                if !self.show_settings {
                    ui.label("|");
                    self.editor.render_bar(ui);
                }
            })
        });
        if self.show_file_manager {
            SidePanel::left("global-left-panel").show(ctx, |ui| {
                self.file_manager.render(ui);
            });
        }
        CentralPanel::default().show(ctx, |ui| {
            if self.show_settings {
                if self.settings.render(ui) {
                    self.file_manager.update_settings(self.settings.clone());
                    self.editor.update_settings(self.settings.clone());
                    self.save_settings(frame);
                }
            } else {
                self.editor.render_main(ui);
            }
        });
    }
}
