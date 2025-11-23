#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use eframe::{
    App, CreationContext, Frame, NativeOptions,
    egui::{self, Context, IconData, Ui, ViewportBuilder},
};
use log::{debug, warn};

use crate::settings::Settings;

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
    settings: Settings,
}

impl TapestryLoomApp {
    fn new(cc: &CreationContext<'_>) -> Self {
        Self {
            show_settings: false,
            settings: if let Some(storage) = cc.storage {
                if let Some(data) = storage.get_string("settings") {
                    match ron::from_str(&data) {
                        Ok(settings) => settings,
                        Err(error) => {
                            warn!(
                                "Unable to deserialize settings: {error:#?}\nUsing default settings"
                            );
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
            },
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
        egui::TopBottomPanel::top("my_panel").show(ctx, |ui| {});
        egui::CentralPanel::default().show(ctx, |ui| {
            // TODO
            if self.settings.render(ctx, ui) {
                self.save_settings(frame);
            }
        });
    }
}

trait AppComponent<T> {
    fn new(settings: Settings) -> Self;
    fn update_settings(&mut self, settings: Settings);
    fn render(&mut self, ctx: &Context, ui: &mut Ui) -> T;
}
