#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{collections::HashSet, mem, path::PathBuf, sync::Arc, time::Duration};

use eframe::{
    CreationContext, NativeOptions,
    egui::{
        self, CentralPanel, Context, FontData, FontDefinitions, FontFamily, IconData, Memory,
        Rangef, Ui, ViewportBuilder, ViewportCommand, WidgetText, style::ScrollAnimation,
    },
};
use egui_notify::{Toast, Toasts};
use egui_tiles::{Tiles, Tree};
use env_logger::Env;
use log::{debug, error, trace};
use mimalloc::MiMalloc;
use parking_lot::Mutex;
use reqwest::{Client, ClientBuilder};
use tokio::runtime::{self, Runtime};

use crate::{
    common::view::{View, ViewContainer},
    editor::{Editor, preload::EditorPreloadHandle},
    files::FileManager,
    settings::{Settings, SettingsView},
};

mod common;
mod editor;
mod files;
mod settings;

const APP_NAME: &str = "Tapestry Loom (WIP REWRITE)"; // TODO

const DEFAULT_LOG_FILTER: &str = "debug,tapestry_loom=trace,tapestry_loom::settings::inference::polyparser=debug,winit=info,naga=info,wgpu_hal=info,layouting=warn,coordinate_calculation=warn,crossing_reduction=warn,ranking=warn,Cycle Removal=warn,connected_components=warn,rust_sugiyama::algorithm=warn";

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<(), anyhow::Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or(DEFAULT_LOG_FILTER)).init();

    debug!("Initalizing...");

    let runtime = Arc::new(runtime::Builder::new_multi_thread().enable_all().build()?);
    eframe::run_native(
        APP_NAME,
        NativeOptions {
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
        },
        Box::new(|cc| {
            let ctrlc_context = cc.egui_ctx.clone();
            ctrlc::set_handler(move || {
                // Hack to work around eframe's lack of signal handling
                ctrlc_context.send_viewport_cmd(ViewportCommand::Close);
            })?;

            Ok(Box::new(App::new(cc, runtime.clone())?))
        }),
    )?;

    debug!("Shutting down async runtime...");

    Arc::try_unwrap(runtime)
        .unwrap()
        .shutdown_timeout(Duration::from_secs(600));

    debug!("Async runtime terminated");

    Ok(())
}

struct App {
    container: ViewContainer<AppShared, Pane>,
    new_panes: Vec<Pane>,
}

impl App {
    fn new(cc: &CreationContext<'_>, runtime: Arc<Runtime>) -> Result<Self, anyhow::Error> {
        cc.egui_ctx.memory_mut(|memory| {
            *memory = Memory::default();
        });

        cc.egui_ctx.set_zoom_factor(1.2); // TODO: Allow customizing zoom
        cc.egui_ctx.options_mut(|options| {
            options.theme_preference = egui::ThemePreference::Dark; // TODO: Allow customizing theme preference
            options.tessellation_options.feathering = false;
        });
        cc.egui_ctx.all_styles_mut(|style| {
            style.animation_time = 0.0;
            style.scroll_animation = ScrollAnimation {
                points_per_second: f32::MAX,
                duration: Rangef {
                    min: 0.0,
                    max: f32::MAX,
                },
            };
            style.compact_menu_style = true;
        });

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
        if let Some(font_keys) = fonts.families.get_mut(&FontFamily::Monospace) {
            font_keys.push("unifontex".into());
            font_keys.insert(1, "noto-emoji".into());
        }
        if let Some(font_keys) = fonts.families.get_mut(&FontFamily::Proportional) {
            font_keys.push("unifontex".into());
            font_keys.insert(1, "noto-emoji".into());
            font_keys.insert(1, "lucide".into());
        }
        cc.egui_ctx.set_fonts(fonts);

        let mut tiles = Tiles::default();

        let tabs = vec![
            tiles.insert_pane(Pane::FileManager(FileManager::default())),
            tiles.insert_pane(Pane::Settings(SettingsView::default())),
        ];

        let root = tiles.insert_tab_tile(tabs);

        let app = Self {
            container: ViewContainer::new(
                Tree::new("global-tree", root, tiles),
                AppShared::new(runtime, Toasts::new(), cc.storage.unwrap())?,
                Some(Box::new(|shared| Pane::Editor(Editor::new(None, shared)))),
            ),
            new_panes: Vec::with_capacity(1),
        };

        debug!("Initialized application context");

        Ok(app)
    }
}

impl eframe::App for App {
    fn auto_save_interval(&self) -> Duration {
        self.container
            .behavior
            .shared
            .settings
            .documents
            .save_interval
    }
    fn logic(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.new_panes.clear();

        self.container.behavior.shared.logic(ctx, |pane| {
            self.new_panes.push(pane);
        });
        for pane in self.new_panes.drain(..) {
            self.container.add_pane(pane);
        }
        self.container.logic(ctx);

        if ctx.input(|i| i.viewport().close_requested()) {
            debug!("Closing views...");

            if !self.container.close() {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            }
        }

        if self.container.is_empty() {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        self.container.behavior.shared.ui(ui);
        if self.container.modals(ui)
            && let Some(window) = frame.winit_window()
        {
            window.focus_window();
        }

        CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).inner_margin(0.0))
            .show_inside(ui, |ui| {
                self.container.ui(ui);
            });

        self.container.behavior.shared.post_ui(ui);

        if ui.output(|output| !output.events.is_empty()) {
            if ui.current_pass_index() == 0 {
                // Discard the frame to allow same-frame feedback (lower input latency)
                ui.request_discard("UI Interaction");
            } else {
                trace!("current_pass_index > 0, falling back to request_repaint()");
                ui.request_repaint();
            }
        }
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.container.save();
        self.container.behavior.shared.save(storage);
    }
}

struct AppShared {
    runtime: Arc<Runtime>,
    async_toasts: Arc<Mutex<Vec<Toast>>>,

    client: Client,
    toasts: Toasts,
    settings: Settings,

    open_documents: HashSet<PathBuf>,
    open_documents_updated: bool,
    load_document_queue: Vec<PathBuf>,

    queued_preload_document: Option<PathBuf>,
    preloaded_document: Option<(PathBuf, EditorPreloadHandle)>,

    fs_needs_refresh: bool,
}

impl AppShared {
    fn new(
        runtime: Arc<Runtime>,
        mut toasts: Toasts,
        storage: &dyn eframe::Storage,
    ) -> Result<Self, anyhow::Error> {
        let settings = if let Some(data) = storage.get_string("settings") {
            match Settings::deserialize(&data) {
                Ok(settings) => settings,
                Err(error) => {
                    toasts.error("Settings deserialization failed");
                    error!("Settings deserialization failed: {error:#?}");
                    Settings::default()
                }
            }
        } else {
            Settings::default()
        };

        let client = ClientBuilder::new()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(300))
            .build()?;

        Ok(Self {
            runtime,
            async_toasts: Arc::new(Mutex::new(Vec::with_capacity(8))),
            toasts,
            client,
            settings,

            open_documents: HashSet::with_capacity(8),
            open_documents_updated: false,
            load_document_queue: Vec::with_capacity(1),

            queued_preload_document: None,
            preloaded_document: None,

            fs_needs_refresh: false,
        })
    }
    fn logic(&mut self, _ctx: &Context, mut add_pane: impl FnMut(Pane)) {
        if !self.load_document_queue.is_empty() {
            let queue = Vec::from_iter(self.load_document_queue.drain(..));

            for path in queue.into_iter() {
                if let Some((preloaded, handle)) = mem::take(&mut self.preloaded_document) {
                    if preloaded == path {
                        add_pane(Pane::Editor(handle.upgrade(self)));
                    } else {
                        self.preloaded_document = Some((preloaded, handle));
                        add_pane(Pane::Editor(Editor::new(Some(path), self)));
                    }
                } else {
                    add_pane(Pane::Editor(Editor::new(Some(path), self)));
                }
            }
        }
    }
    fn ui(&mut self, ui: &mut Ui) {
        for toast in self.async_toasts.lock().drain(..) {
            self.toasts.add(toast);
        }

        self.toasts.show(ui);
    }
    fn post_ui(&mut self, _ctx: &Context) {
        if let Some(path) = &self.queued_preload_document
            && !self.open_documents.contains(path)
        {
            if let Some((preloaded, _)) = &mut self.preloaded_document {
                if preloaded != path {
                    self.preloaded_document =
                        Some((path.clone(), EditorPreloadHandle::new(path.clone(), self)));
                }
            } else {
                self.preloaded_document =
                    Some((path.clone(), EditorPreloadHandle::new(path.clone(), self)));
            }
        }
        self.queued_preload_document = None;
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        match self.settings.serialize() {
            Ok(data) => {
                debug!("Saved settings to disk");
                storage.set_string("settings", data);
            }
            Err(error) => {
                self.toasts.error("Settings serialization failed");
                error!("Settings serialization failed: {error:#?}")
            }
        }
    }
}

#[allow(clippy::large_enum_variant)]
enum Pane {
    Settings(SettingsView),
    FileManager(FileManager),
    Editor(Editor),
}

impl View<AppShared> for Pane {
    fn title(&self, shared: &AppShared) -> WidgetText {
        match self {
            Self::Settings(view) => view.title(shared),
            Self::FileManager(view) => view.title(shared),
            Self::Editor(view) => view.title(shared),
        }
    }
    fn closable(&self, shared: &AppShared) -> bool {
        match self {
            Self::Settings(view) => view.closable(shared),
            Self::FileManager(view) => view.closable(shared),
            Self::Editor(view) => view.closable(shared),
        }
    }
    fn check_close(&mut self, shared: &mut AppShared) -> bool {
        match self {
            Self::Settings(view) => view.check_close(shared),
            Self::FileManager(view) => view.check_close(shared),
            Self::Editor(view) => view.check_close(shared),
        }
    }
    fn logic(&mut self, shared: &mut AppShared, force_close: impl FnOnce(), ctx: &Context) {
        match self {
            Self::Settings(view) => view.logic(shared, force_close, ctx),
            Self::FileManager(view) => view.logic(shared, force_close, ctx),
            Self::Editor(view) => view.logic(shared, force_close, ctx),
        }
    }
    fn modals(&mut self, shared: &mut AppShared, ctx: &Context) -> bool {
        match self {
            Self::Settings(view) => view.modals(shared, ctx),
            Self::FileManager(view) => view.modals(shared, ctx),
            Self::Editor(view) => view.modals(shared, ctx),
        }
    }
    fn ui(&mut self, shared: &mut AppShared, ui: &mut Ui) {
        match self {
            Self::Settings(view) => view.ui(shared, ui),
            Self::FileManager(view) => view.ui(shared, ui),
            Self::Editor(view) => view.ui(shared, ui),
        }
    }
    fn save(&mut self, shared: &mut AppShared) {
        match self {
            Self::Settings(view) => view.save(shared),
            Self::FileManager(view) => view.save(shared),
            Self::Editor(view) => view.save(shared),
        }
    }
    fn close(&mut self, shared: &mut AppShared) -> bool {
        match self {
            Self::Settings(view) => view.close(shared),
            Self::FileManager(view) => view.close(shared),
            Self::Editor(view) => view.close(shared),
        }
    }
}
