#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{cell::RefCell, path::PathBuf, rc::Rc, sync::Arc};

use eframe::{
    App, CreationContext, Frame, NativeOptions,
    egui::{
        CentralPanel, Context, FontDefinitions, FontFamily, IconData, RichText, Ui,
        ViewportBuilder, WidgetText,
    },
};
use egui_phosphor::{fill, regular};
use egui_tiles::{Behavior, SimplificationOptions, Tile, TileId, Tiles, Tree, UiResponse};
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
        fonts.font_data.insert(
            "phosphor".into(),
            Arc::new(egui_phosphor::Variant::Regular.font_data()),
        );
        fonts.families.insert(
            FontFamily::Name("phosphor".into()),
            vec!["Ubuntu-Light".into(), "phosphor".into()],
        );
        /*fonts.font_data.insert(
            "phosphor-bold".into(),
            Arc::new(egui_phosphor::Variant::Bold.font_data()),
        );
        fonts.families.insert(
            FontFamily::Name("phosphor-bold".into()),
            vec!["Ubuntu-Light".into(), "phosphor-bold".into()],
        );*/
        fonts.font_data.insert(
            "phosphor-fill".into(),
            Arc::new(egui_phosphor::Variant::Fill.font_data()),
        );
        fonts.families.insert(
            FontFamily::Name("phosphor-fill".into()),
            vec!["Ubuntu-Light".into(), "phosphor-fill".into()],
        );
        cc.egui_ctx.set_fonts(fonts);

        let behavior = TapestryLoomBehavior {
            file_manager: FileManager::new(settings.clone()),
            unchanged_settings_changes: false,
            new_editor_queue: Vec::with_capacity(16),
            settings,
            settings_tab_label: Arc::new(
                RichText::new([fill::GEAR, " Settings"].concat())
                    .family(FontFamily::Name("phosphor-fill".into())),
            ),
            file_manager_tab_label: Arc::new(
                RichText::new([fill::FOLDERS, " Files"].concat())
                    .family(FontFamily::Name("phosphor-fill".into())),
            ),
            new_tab_label: Arc::new(
                RichText::new(regular::PLUS).family(FontFamily::Name("phosphor".into())),
            ),
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
                self.behavior.unchanged_settings_changes = false;
            }

            if !self.behavior.new_editor_queue.is_empty() {
                let mut new_tiles = Vec::with_capacity(self.behavior.new_editor_queue.len());

                for path in &self.behavior.new_editor_queue {
                    match Editor::new(self.behavior.settings.clone(), path.as_deref()) {
                        Ok(editor) => {
                            new_tiles.push(self.tree.tiles.insert_pane(Pane::Editor(editor)));
                        }
                        Err(error) => {
                            // TODO
                        }
                    }
                }

                if let Some(Tile::Container(root)) = self
                    .tree
                    .root
                    .and_then(|root| self.tree.tiles.get_mut(root))
                {
                    for id in new_tiles {
                        root.add_child(id);
                    }
                } else {
                    // TODO
                }
                self.behavior.new_editor_queue.clear();
            }
        });
    }
}

struct TapestryLoomBehavior {
    settings_tab_label: Arc<RichText>,
    file_manager_tab_label: Arc<RichText>,
    new_tab_label: Arc<RichText>,
    settings: Rc<RefCell<Settings>>,
    unchanged_settings_changes: bool,
    new_editor_queue: Vec<Option<PathBuf>>,
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
            Pane::Settings => WidgetText::RichText(self.settings_tab_label.clone()),
            Pane::FileManager => WidgetText::RichText(self.file_manager_tab_label.clone()),
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
                    self.new_editor_queue.push(Some(path));
                }
            }
            Pane::Editor(editor) => editor.render(ui),
        }

        UiResponse::None
    }
    fn is_tab_closable(&self, tiles: &Tiles<Pane>, tile_id: TileId) -> bool {
        if let Some(tile) = tiles.get(tile_id) {
            match tile {
                Tile::Container(_) => false,
                Tile::Pane(pane) => match pane {
                    Pane::Settings => false,
                    Pane::FileManager => false,
                    Pane::Editor(_) => true,
                },
            }
        } else {
            false
        }
    }
    fn simplification_options(&self) -> SimplificationOptions {
        SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }
    fn top_bar_right_ui(
        &mut self,
        _tiles: &Tiles<Pane>,
        ui: &mut Ui,
        _tile_id: TileId,
        _tabs: &egui_tiles::Tabs,
        _scroll_offset: &mut f32,
    ) {
        if ui.button(self.new_tab_label.clone()).clicked() {
            self.new_editor_queue.push(None);
        }
    }
    fn on_tab_close(&mut self, _tiles: &mut Tiles<Pane>, _tile_id: TileId) -> bool {
        true
    }
}
