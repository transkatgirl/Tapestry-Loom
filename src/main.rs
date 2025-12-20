#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{cell::RefCell, collections::HashSet, fs, path::PathBuf, rc::Rc, sync::Arc};

use eframe::{
    App, CreationContext, Frame, NativeOptions,
    egui::{
        self, CentralPanel, Context, FontData, FontDefinitions, IconData, Modal, Sides, Ui,
        ViewportBuilder, WidgetText,
    },
    epaint::MarginF32,
};
use egui_notify::Toasts;
use egui_tiles::{
    Behavior, Container, SimplificationOptions, Tile, TileId, Tiles, Tree, UiResponse,
};
use env_logger::Env;
use flagset::FlagSet;
use font_kit::{
    family_name::FamilyName, handle::Handle, properties::Properties, source::SystemSource,
};
use log::{debug, error, warn};
use mimalloc::MiMalloc;
use threadpool::ThreadPool;
use tokio::runtime::Runtime;

use crate::{
    editor::Editor,
    files::FileManager,
    settings::{
        Settings, UIFonts, UISettings,
        inference::{ClientConfig, InferenceClient},
        shortcuts::Shortcuts,
    },
};

mod editor;
mod files;
mod settings;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> eframe::Result {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug,tapestry_loom=trace,winit=info,layouting=warn,coordinate_calculation=warn,crossing_reduction=warn,ranking=warn,Cycle Removal=warn,connected_components=warn,rust_sugiyama::algorithm=warn")).init();
    let options = NativeOptions {
        #[cfg(target_os = "macos")]
        viewport: ViewportBuilder::default()
            .with_fullscreen(true)
            .with_icon(Arc::new(IconData::default())),
        #[cfg(not(target_os = "macos"))]
        viewport: ViewportBuilder::default()
            .with_maximized(true)
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
    show_confirmation: bool,
    allow_close: bool,
    last_ui_settings: UISettings,
    last_client_settings: ClientConfig,
}

impl TapestryLoomApp {
    fn new(cc: &CreationContext<'_>) -> Self {
        let ctrlc_context = cc.egui_ctx.clone();

        // Hack to work around eframe's lack of signal handling
        let _ = ctrlc::set_handler(move || {
            ctrlc_context.send_viewport_cmd(egui::ViewportCommand::Close);
        });

        let mut toasts = Toasts::new();

        let settings = if let Some(storage) = cc.storage {
            if let Some(data) = storage.get_string("settings") {
                match ron::from_str(&data) {
                    Ok(settings) => settings,
                    Err(error) => {
                        toasts.error("Settings deserialization failed");
                        error!("Settings deserialization failed: {error:#?}");
                        Settings::default()
                    }
                }
            } else {
                Settings::default()
            }
        } else {
            toasts.error("Unable to open settings storage");
            error!("Settings storage not found");
            Settings::default()
        };
        let settings = Rc::new(RefCell::new(settings));

        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "lucide".into(),
            Arc::new(FontData::from_static(include_bytes!(
                "../fonts/icons/Lucide.ttf"
            ))),
        );
        fonts.font_data.insert(
            "unifontex".into(),
            Arc::new(FontData::from_static(include_bytes!(
                "../fonts/UnifontExMono.ttf"
            ))),
        );
        fonts.font_data.insert(
            "noto-emoji".into(),
            Arc::new(FontData::from_static(include_bytes!(
                "../fonts/NotoEmoji.ttf"
            ))),
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
        if let Some(font_keys) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            font_keys.push("unifontex".into());
            font_keys.insert(1, "noto-emoji".into());
        }
        if let Some(font_keys) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            font_keys.push("unifontex".into());
            font_keys.insert(1, "noto-emoji".into());
        }
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

        if settings.borrow().interface.ui_fonts == UIFonts::System {
            debug!("Loading system monospace font");

            match SystemSource::new()
                .select_best_match(&[FamilyName::Monospace], &Properties::new())
            {
                Ok(Handle::Memory { bytes, .. }) => {
                    fonts.font_data.insert(
                        "system-monospace".into(),
                        Arc::new(FontData::from_owned(bytes.to_vec())),
                    );
                    if let Some(font_keys) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                        font_keys.insert(0, "system-monospace".into());
                    }
                }

                Ok(Handle::Path { path, .. }) => match fs::read(path) {
                    Ok(bytes) => {
                        fonts.font_data.insert(
                            "system-monospace".into(),
                            Arc::new(FontData::from_owned(bytes)),
                        );
                        if let Some(font_keys) =
                            fonts.families.get_mut(&egui::FontFamily::Monospace)
                        {
                            font_keys.insert(0, "system-monospace".into());
                        }
                    }
                    Err(error) => {
                        toasts.warning("Failed to load system monospace font");
                        warn!("Failed to load system monospace font: {error:#?}")
                    }
                },
                Err(error) => {
                    toasts.warning("Failed to load system monospace font");
                    warn!("Failed to select system monospace font: {error:#?}")
                }
            }

            debug!("Loading system sans-serif font");

            match SystemSource::new()
                .select_best_match(&[FamilyName::SansSerif], &Properties::new())
            {
                Ok(Handle::Memory { bytes, .. }) => {
                    fonts.font_data.insert(
                        "system-sans-serif".into(),
                        Arc::new(FontData::from_owned(bytes.to_vec())),
                    );
                    if let Some(font_keys) = fonts.families.get_mut(&egui::FontFamily::Proportional)
                    {
                        font_keys.insert(0, "system-sans-serif".into());
                    }
                }

                Ok(Handle::Path { path, .. }) => match fs::read(path) {
                    Ok(bytes) => {
                        fonts.font_data.insert(
                            "system-sans-serif".into(),
                            Arc::new(FontData::from_owned(bytes)),
                        );
                        if let Some(font_keys) =
                            fonts.families.get_mut(&egui::FontFamily::Proportional)
                        {
                            font_keys.insert(0, "system-sans-serif".into());
                        }
                    }
                    Err(error) => {
                        toasts.warning("Failed to load system sans-serif font");
                        warn!("Failed to load system sans-serif font: {error:#?}")
                    }
                },
                Err(error) => {
                    toasts.warning("Failed to load system sans-serif font");
                    warn!("Failed to select system sans-serif font: {error:#?}")
                }
            }
        } else if settings.borrow().interface.ui_fonts == UIFonts::UnifontEX {
            if let Some(font_keys) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                font_keys.insert(0, "unifontex".into());
            }
            if let Some(font_keys) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                font_keys.insert(0, "unifontex".into());
            }
        }

        cc.egui_ctx.set_fonts(fonts);

        let client = match settings.borrow().inference.client.build() {
            Ok(client) => Some(client),
            Err(error) => {
                toasts.error("Failed to initialize HTTP client");
                error!("Reqwest client initialization failed: {error:#?}");
                None
            }
        };

        let toasts = Rc::new(RefCell::new(toasts));
        let threadpool = Rc::new(ThreadPool::new(16));
        let open_documents = Rc::new(RefCell::new(HashSet::with_capacity(64)));

        let behavior = TapestryLoomBehavior {
            file_manager: Rc::new(RefCell::new(FileManager::new(
                settings.clone(),
                toasts.clone(),
                threadpool.clone(),
                open_documents.clone(),
            ))),
            new_editor_queue: Vec::with_capacity(16),
            focus_queue: Vec::with_capacity(16),
            close_queue: Vec::with_capacity(16),
            settings,
            client: Rc::new(RefCell::new(client)),
            toasts,
            runtime: Arc::new(
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap(),
            ),
            threadpool,
            open_documents,
            pressed_shortcuts: FlagSet::empty(),
            settings_visible: false,
            settings_last_visible: false,
        };

        let mut tiles = Tiles::default();

        let tabs = vec![
            tiles.insert_pane(Pane::FileManager),
            tiles.insert_pane(Pane::Settings),
        ];

        let root = tiles.insert_tab_tile(tabs);

        behavior.settings.borrow().interface.apply(&cc.egui_ctx);

        let last_ui_settings = behavior.settings.borrow().interface;
        let last_client_settings = behavior.settings.borrow().inference.client.clone();

        Self {
            behavior,
            tree: Tree::new("global-tree", root, tiles),
            show_confirmation: false,
            allow_close: false,
            last_ui_settings,
            last_client_settings,
        }
    }
    fn allow_close(&self) -> bool {
        for tile in self.tree.tiles.tiles() {
            if let Tile::Pane(Pane::Editor(editor)) = tile
                && editor.unsaved()
            {
                return false;
            }
        }

        true
    }
}

impl App for TapestryLoomApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        if !self.behavior.settings_last_visible {
            self.behavior.pressed_shortcuts =
                self.behavior.settings.borrow().shortcuts.get_pressed(ctx);
        }

        CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(0.0))
            .show(ctx, |ui| {
                self.tree.ui(&mut self.behavior, ui);

                for tile in self.behavior.close_queue.drain(..) {
                    self.tree.remove_recursively(tile);
                }

                for tile in self.behavior.focus_queue.drain(..) {
                    focus_tile(&mut self.tree.tiles, tile);
                }

                if !self.behavior.new_editor_queue.is_empty() {
                    let mut new_tiles = Vec::with_capacity(self.behavior.new_editor_queue.len());

                    for (path, parent) in self.behavior.new_editor_queue.drain(..) {
                        if let Some(path) = &path
                            && self.behavior.open_documents.borrow().contains(path)
                        {
                            continue;
                        }

                        let file_manager = self.behavior.file_manager.clone();

                        let identifier =
                            self.tree
                                .tiles
                                .insert_pane(Pane::Editor(Box::new(Editor::new(
                                    self.behavior.settings.clone(),
                                    self.behavior.toasts.clone(),
                                    self.behavior.threadpool.clone(),
                                    self.behavior.open_documents.clone(),
                                    self.behavior.runtime.clone(),
                                    self.behavior.client.clone(),
                                    path,
                                    Box::new(move |_| {
                                        file_manager.borrow_mut().update();
                                    }),
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
                        error!("Unable to find window root");
                    }
                }
            });
        self.behavior.toasts.borrow_mut().show(ctx);

        if ctx.input(|i| i.viewport().close_requested())
            && !(self.allow_close || self.allow_close())
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.show_confirmation = true;
        }

        if self.show_confirmation
            && Modal::new("global-close-modal".into())
                .show(ctx, |ui| {
                    ui.set_width(230.0);
                    ui.heading("Do you want to quit without saving?");
                    ui.label("One or more open weaves are unsaved.");
                    ui.add_space(ui.style().spacing.menu_spacing);
                    Sides::new().show(
                        ui,
                        |_ui| {},
                        |ui| {
                            if ui.button("Yes").clicked() {
                                self.allow_close = true;
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                                ui.close();
                            }
                            if ui.button("No").clicked() {
                                self.allow_close = false;
                                ui.close();
                            }
                        },
                    );
                })
                .should_close()
        {
            self.show_confirmation = false;
        }

        let mut settings = self.behavior.settings.borrow_mut();

        if settings.interface != self.last_ui_settings {
            settings.interface.apply(ctx);
            self.last_ui_settings = settings.interface;
        }
        if settings.inference.client != self.last_client_settings {
            *self.behavior.client.borrow_mut() = match settings.inference.client.build() {
                Ok(client) => Some(client),
                Err(error) => {
                    self.behavior
                        .toasts
                        .borrow_mut()
                        .error("Failed to initialize HTTP client");
                    error!("Reqwest client initialization failed: {error:#?}");
                    None
                }
            };
            self.last_client_settings = settings.inference.client.clone();
        }
        if self.behavior.settings_last_visible {
            self.behavior.pressed_shortcuts = settings.shortcuts.get_pressed(ctx);
        }
        settings.handle_shortcuts(&ctx.style(), self.behavior.pressed_shortcuts);

        self.behavior.settings_last_visible = self.behavior.settings_visible;
        self.behavior.settings_visible = false;
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        match ron::to_string(&self.behavior.settings) {
            Ok(data) => {
                debug!("Saved settings to disk");
                storage.set_string("settings", data);
            }
            Err(error) => {
                self.behavior
                    .toasts
                    .borrow_mut()
                    .error("Settings serialization failed");
                error!("Settings serialization failed: {error:#?}")
            }
        }
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        debug!("Closing editors...");
        for tile in self.tree.tiles.tiles_mut() {
            if let Tile::Pane(Pane::Editor(editor)) = tile {
                editor.close();
            }
        }

        debug!("Waiting for IO threads to terminate...");
        self.behavior.threadpool.join();
    }
}

struct TapestryLoomBehavior {
    settings: Rc<RefCell<Settings>>,
    client: Rc<RefCell<Option<InferenceClient>>>,
    new_editor_queue: Vec<(Option<PathBuf>, Option<TileId>)>,
    focus_queue: Vec<TileId>,
    close_queue: Vec<TileId>,
    file_manager: Rc<RefCell<FileManager>>,
    toasts: Rc<RefCell<Toasts>>,
    threadpool: Rc<ThreadPool>,
    runtime: Arc<Runtime>,
    open_documents: Rc<RefCell<HashSet<PathBuf>>>,
    pressed_shortcuts: FlagSet<Shortcuts>,
    settings_visible: bool,
    settings_last_visible: bool,
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
    fn pane_ui(&mut self, ui: &mut Ui, tile_id: TileId, pane: &mut Pane) -> UiResponse {
        match pane {
            Pane::Settings => {
                self.settings_visible = true;
                self.settings.borrow_mut().render(ui);
            }
            Pane::FileManager => {
                for path in self
                    .file_manager
                    .borrow_mut()
                    .render(ui, self.pressed_shortcuts)
                {
                    self.new_editor_queue.push((Some(path), None));
                }
            }
            Pane::Editor(editor) => editor.render(
                ui,
                || {
                    self.close_queue.push(tile_id);
                },
                self.pressed_shortcuts,
            ),
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
            return if editor.close() {
                true
            } else {
                self.focus_queue.push(tile_id);
                false
            };
        }

        true
    }
}

pub fn listing_margin(ui: &mut Ui) -> MarginF32 {
    MarginF32::same(ui.style().spacing.menu_spacing)
}

fn focus_tile(tiles: &mut Tiles<Pane>, tile_id: TileId) {
    if let Some(parent_id) = tiles.parent_of(tile_id)
        && let Some(Tile::Container(Container::Tabs(tabs))) = tiles.get_mut(parent_id)
    {
        tabs.set_active(tile_id);
    }
}
