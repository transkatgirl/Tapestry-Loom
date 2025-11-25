#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{cell::RefCell, collections::HashSet, path::PathBuf, rc::Rc, sync::Arc};

use eframe::{
    App, CreationContext, Frame, NativeOptions,
    egui::{
        self, CentralPanel, Context, FontData, FontDefinitions, IconData, Ui, ViewportBuilder,
        WidgetText,
    },
};
use egui_notify::Toasts;
use egui_tiles::{Behavior, SimplificationOptions, Tile, TileId, Tiles, Tree, UiResponse};
use mimalloc::MiMalloc;
use threadpool::ThreadPool;
use tokio::runtime::Runtime;

use crate::{editor::Editor, files::FileManager, settings::Settings};

mod editor;
mod files;
mod settings;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> eframe::Result {
    env_logger::init();

    #[cfg(not(debug_assertions))]
    let _ = ctrlc::set_handler(|| {}); // Hack to work around eframe's lack of signal handling

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
        let mut toasts = Toasts::new();

        let settings = if let Some(storage) = cc.storage {
            if let Some(data) = storage.get_string("settings") {
                match ron::from_str(&data) {
                    Ok(settings) => settings,
                    Err(error) => {
                        toasts.error(format!("Settings deserialization failed: {error:?}"));
                        Settings::default()
                    }
                }
            } else {
                Settings::default()
            }
        } else {
            toasts.error("Unable to open settings storage");
            Settings::default()
        };
        let settings = Rc::new(RefCell::new(settings));

        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "lucide".into(),
            Arc::new(FontData::from_static(include_bytes!("../icons/Lucide.ttf"))),
        );
        /*fonts.font_data.insert(
            "phosphor".into(),
            Arc::new(FontData::from_static(include_bytes!(
                "../icons/Phosphor.ttf"
            ))),
        );
        fonts.font_data.insert(
            "phosphor-light".into(),
            Arc::new(FontData::from_static(include_bytes!(
                "../icons/Phosphor-Light.ttf"
            ))),
        );
        fonts.font_data.insert(
            "phosphor-bold".into(),
            Arc::new(FontData::from_static(include_bytes!(
                "../icons/Phosphor-Bold.ttf"
            ))),
        );
        fonts.font_data.insert(
            "phosphor-fill".into(),
            Arc::new(FontData::from_static(include_bytes!(
                "../icons/Phosphor-Fill.ttf"
            ))),
        );*/
        if let Some(font_keys) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            font_keys.insert(1, "lucide".into());
        }
        /*fonts.families.insert(
            FontFamily::Name("lucide".into()),
            vec!["Ubuntu-Light".into(), "phosphor-bold".into()],
        );
        fonts.families.insert(
            FontFamily::Name("phosphor".into()),
            vec!["Ubuntu-Light".into(), "phosphor".into()],
        );
        fonts.families.insert(
            FontFamily::Name("phosphor-light".into()),
            vec!["Ubuntu-Light".into(), "phosphor-light".into()],
        );
        fonts.families.insert(
            FontFamily::Name("phosphor-bold".into()),
            vec!["Ubuntu-Light".into(), "phosphor-bold".into()],
        );
        fonts.families.insert(
            FontFamily::Name("phosphor-fill".into()),
            vec!["Ubuntu-Light".into(), "phosphor-fill".into()],
        );*/

        cc.egui_ctx.set_fonts(fonts);

        let toasts = Rc::new(RefCell::new(toasts));
        let threadpool = Rc::new(ThreadPool::new(16));
        let open_documents = Rc::new(RefCell::new(HashSet::with_capacity(64)));
        let behavior = TapestryLoomBehavior {
            file_manager: FileManager::new(
                settings.clone(),
                toasts.clone(),
                threadpool.clone(),
                open_documents.clone(),
            ),
            unsaved_settings_changes: false,
            new_editor_queue: Vec::with_capacity(16),
            settings,
            toasts,
            runtime: Arc::new(
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap(),
            ),
            threadpool,
            open_documents,
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
                    storage.set_string("settings", data);
                }
                Err(error) => {
                    self.behavior
                        .toasts
                        .borrow_mut()
                        .error(format!("Settings serialization failed: {error:?}"));
                }
            }
        } else {
            self.behavior
                .toasts
                .borrow_mut()
                .error("Unable to open settings storage");
        }
    }
}

impl App for TapestryLoomApp {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(0.0))
            .show(ctx, |ui| {
                ctx.style_mut(|style| style.animation_time = 0.0);

                self.tree.ui(&mut self.behavior, ui);

                if self.behavior.unsaved_settings_changes {
                    self.save_settings(frame);
                    self.behavior.unsaved_settings_changes = false;
                }

                if !self.behavior.new_editor_queue.is_empty() {
                    let mut new_tiles = Vec::with_capacity(self.behavior.new_editor_queue.len());

                    for (path, parent) in self.behavior.new_editor_queue.drain(..) {
                        if let Some(path) = &path
                            && self.behavior.open_documents.borrow().contains(path)
                        {
                            continue;
                        }

                        let identifier =
                            self.tree
                                .tiles
                                .insert_pane(Pane::Editor(Box::new(Editor::new(
                                    self.behavior.settings.clone(),
                                    self.behavior.toasts.clone(),
                                    self.behavior.threadpool.clone(),
                                    self.behavior.open_documents.clone(),
                                    self.behavior.runtime.clone(),
                                    path,
                                ))));

                        if let Some(Tile::Container(parent)) =
                            parent.and_then(|root| self.tree.tiles.get_mut(root))
                        {
                            parent.add_child(identifier);
                            if let egui_tiles::Container::Tabs(tabs) = parent {
                                tabs.set_active(identifier);
                            }
                        } else {
                            new_tiles.push(identifier);
                        }
                    }

                    if let Some(Tile::Container(root)) = self
                        .tree
                        .root
                        .and_then(|root| self.tree.tiles.get_mut(root))
                    {
                        for id in new_tiles {
                            root.add_child(id);
                            if let egui_tiles::Container::Tabs(tabs) = root {
                                tabs.set_active(id);
                            }
                        }
                    } else {
                        self.behavior
                            .toasts
                            .borrow_mut()
                            .error("Unable to find window root");
                    }
                }
            });
        self.behavior.toasts.borrow_mut().show(ctx);
    }
}

struct TapestryLoomBehavior {
    settings: Rc<RefCell<Settings>>,
    unsaved_settings_changes: bool,
    new_editor_queue: Vec<(Option<PathBuf>, Option<TileId>)>,
    file_manager: FileManager,
    toasts: Rc<RefCell<Toasts>>,
    threadpool: Rc<ThreadPool>,
    runtime: Arc<Runtime>,
    open_documents: Rc<RefCell<HashSet<PathBuf>>>,
}

enum Pane {
    Settings,
    FileManager,
    Editor(Box<Editor>),
}

impl Behavior<Pane> for TapestryLoomBehavior {
    fn tab_title_for_pane(&mut self, pane: &Pane) -> WidgetText {
        match pane {
            Pane::Settings => WidgetText::Text("\u{E154} Settings".to_string()),
            Pane::FileManager => WidgetText::Text("\u{E33C} Files".to_string()),
            Pane::Editor(editor) => WidgetText::Text(editor.title.clone()),
        }
    }
    fn pane_ui(&mut self, ui: &mut Ui, _tile_id: TileId, pane: &mut Pane) -> UiResponse {
        match pane {
            Pane::Settings => {
                if self.settings.borrow_mut().render(ui) {
                    self.unsaved_settings_changes = true;
                }
            }
            Pane::FileManager => {
                for path in self.file_manager.render(ui) {
                    self.new_editor_queue.push((Some(path), None));
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
        tile_id: TileId,
        _tabs: &egui_tiles::Tabs,
        _scroll_offset: &mut f32,
    ) {
        if ui.button("\u{E13D}").clicked() {
            self.new_editor_queue.push((None, Some(tile_id)));
        }
    }
    fn on_tab_close(&mut self, tiles: &mut Tiles<Pane>, tile_id: TileId) -> bool {
        if let Some(Tile::Pane(Pane::Editor(editor))) = tiles.get_mut(tile_id) {
            return editor.close();
        }

        true
    }
}

impl Drop for TapestryLoomBehavior {
    fn drop(&mut self) {
        self.threadpool.join();
    }
}
