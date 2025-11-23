#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use eframe::{
    App, CreationContext, Frame, NativeOptions,
    egui::{
        Button, CentralPanel, Context, FontDefinitions, IconData, SidePanel, TopBottomPanel, Ui,
        ViewportBuilder, WidgetText,
    },
};
use egui_tiles::{Behavior, SimplificationOptions, Tile, TileId, Tiles, Tree, UiResponse};
use log::{debug, warn};
use parking_lot::Mutex;

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
    behavior: TapestryLoomBehavior,
    tree: Tree<Pane>,
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
        let settings = Rc::new(RefCell::new(settings));

        let mut fonts = FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Fill);
        cc.egui_ctx.set_fonts(fonts);

        let behavior = TapestryLoomBehavior {
            file_manager: FileManager::new(settings.clone()),
            unchanged_settings_changes: false,
            queued_files: Vec::with_capacity(16),
            settings,
        };

        let mut tiles = Tiles::default();

        let tabs = vec![
            tiles.insert_pane(Pane::FileManager),
            tiles.insert_pane(Pane::Settings),
        ];

        let root = tiles.insert_tab_tile(tabs);

        Self {
            behavior,
            tree: Tree::new("global-tree", root, tiles),
        }
    }
    fn save_settings(&self, frame: &mut Frame) {
        if let Some(storage) = frame.storage_mut() {
            match ron::to_string(&self.behavior.settings) {
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
        CentralPanel::default().show(ctx, |ui| {
            self.tree.ui(&mut self.behavior, ui);

            if self.behavior.unchanged_settings_changes {
                self.save_settings(frame);
            }

            if !self.behavior.queued_files.is_empty() {}
        });
    }
}

struct TapestryLoomBehavior {
    settings: Rc<RefCell<Settings>>,
    unchanged_settings_changes: bool,
    queued_files: Vec<PathBuf>,
    file_manager: FileManager,
}

enum Pane {
    Settings,
    FileManager,
    Editor(Editor),
}

impl Behavior<Pane> for TapestryLoomBehavior {
    fn tab_title_for_pane(&mut self, pane: &Pane) -> WidgetText {
        match pane {
            Pane::Settings => WidgetText::Text("Settings".to_string()),
            Pane::FileManager => WidgetText::Text("Files".to_string()),
            Pane::Editor(editor) => WidgetText::Text(editor.title.clone()),
        }
    }
    fn pane_ui(&mut self, ui: &mut Ui, _tile_id: TileId, pane: &mut Pane) -> UiResponse {
        match pane {
            Pane::Settings => {
                if self.settings.borrow_mut().render(ui) {
                    self.unchanged_settings_changes = true;
                }
            }
            Pane::FileManager => {
                if let Some(path) = self.file_manager.render(ui) {
                    self.queued_files.push(path);
                }
            }
            Pane::Editor(editor) => editor.render(ui),
        }

        UiResponse::None
    }
    fn is_tab_closable(&self, tiles: &Tiles<Pane>, tile_id: TileId) -> bool {
        if let Some(tile) = tiles.get(tile_id) {
            match tile {
                Tile::Container(_) => true,
                Tile::Pane(pane) => match pane {
                    Pane::Settings => false,
                    Pane::FileManager => false,
                    Pane::Editor(_) => true,
                },
            }
        } else {
            true
        }
    }
    fn simplification_options(&self) -> SimplificationOptions {
        SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }
    fn on_tab_close(&mut self, _tiles: &mut Tiles<Pane>, _tile_id: TileId) -> bool {
        true
    }
}
