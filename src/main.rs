#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{sync::Arc, time::Duration};

use eframe::{
    CreationContext, NativeOptions,
    egui::{
        self, CentralPanel, Context, FontData, FontDefinitions, FontFamily, IconData, Memory, Ui,
        ViewportBuilder, ViewportCommand, WidgetText,
    },
};
use egui_notify::Toasts;
use egui_tiles::{Tiles, Tree};
use env_logger::Env;
use log::{debug, error};
use mimalloc::MiMalloc;
use reqwest::{Client, ClientBuilder};
use tokio::runtime::{self, Runtime};

use crate::{
    editor::Editor,
    files::FileManager,
    settings::{Settings, SettingsView},
    shared::view::{View, ViewContainer},
};

mod editor;
mod files;
mod settings;
mod shared;

const DEFAULT_LOG_FILTER: &str = "debug,tapestry_loom=trace,tapestry_loom::settings::inference::polyparser=debug,winit=info,naga=info,wgpu_hal=info,layouting=warn,coordinate_calculation=warn,crossing_reduction=warn,ranking=warn,Cycle Removal=warn,connected_components=warn,rust_sugiyama::algorithm=warn";

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<(), anyhow::Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or(DEFAULT_LOG_FILTER)).init();

    debug!("Initalizing...");

    let runtime = Arc::new(runtime::Builder::new_multi_thread().enable_all().build()?);
    eframe::run_native(
        "Tapestry Loom (WIP REWRITE)",
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
}

impl App {
    fn new(cc: &CreationContext<'_>, runtime: Arc<Runtime>) -> Result<Self, anyhow::Error> {
        cc.egui_ctx.memory_mut(|memory| {
            *memory = Memory::default();
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
        };

        debug!("Initialized application context");

        Ok(app)
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.container.behavior.shared.logic(ctx);
        self.container.logic(ctx);

        if ctx.input(|i| i.viewport().close_requested()) && !self.container.close() {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
        }
    }
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.container.behavior.shared.ui(ui);
        self.container.modals(ui);

        CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).inner_margin(0.0))
            .show_inside(ui, |ui| {
                self.container.ui(ui);
            });
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.container.save();
        self.container.behavior.shared.save(storage);
    }
}

enum Pane {
    Settings(SettingsView),
    FileManager(FileManager),
    Editor(Editor),
}

struct AppShared {
    runtime: Arc<Runtime>,
    client: Client,
    toasts: Toasts,
    settings: Settings,
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
            toasts,
            client,
            settings,
        })
    }
    fn logic(&mut self, _ctx: &Context) {}
    fn ui(&mut self, ui: &mut Ui) {
        self.toasts.show(ui);
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

impl View<AppShared> for Pane {
    fn title(&self, shared: &AppShared) -> WidgetText {
        match self {
            Self::Settings(settings) => settings.title(shared),
            Self::FileManager(file_manager) => file_manager.title(shared),
            Self::Editor(editor) => editor.title(shared),
        }
    }
    fn closable(&self, shared: &AppShared) -> bool {
        match self {
            Self::Settings(settings) => settings.closable(shared),
            Self::FileManager(file_manager) => file_manager.closable(shared),
            Self::Editor(editor) => editor.closable(shared),
        }
    }
    fn check_close(&mut self, shared: &mut AppShared) -> bool {
        match self {
            Self::Settings(settings) => settings.check_close(shared),
            Self::FileManager(file_manager) => file_manager.check_close(shared),
            Self::Editor(editor) => editor.check_close(shared),
        }
    }
    fn logic(&mut self, shared: &mut AppShared, ctx: &Context) {
        match self {
            Self::Settings(settings) => settings.logic(shared, ctx),
            Self::FileManager(file_manager) => file_manager.logic(shared, ctx),
            Self::Editor(editor) => editor.logic(shared, ctx),
        }
    }
    fn modals(&mut self, shared: &mut AppShared, ctx: &Context) -> bool {
        match self {
            Self::Settings(settings) => settings.modals(shared, ctx),
            Self::FileManager(file_manager) => file_manager.modals(shared, ctx),
            Self::Editor(editor) => editor.modals(shared, ctx),
        }
    }
    fn ui(&mut self, shared: &mut AppShared, ui: &mut Ui) {
        match self {
            Self::Settings(settings) => settings.ui(shared, ui),
            Self::FileManager(file_manager) => file_manager.ui(shared, ui),
            Self::Editor(editor) => editor.ui(shared, ui),
        }
    }
    fn close(&mut self, shared: &mut AppShared) -> bool {
        match self {
            Self::Settings(settings) => settings.close(shared),
            Self::FileManager(file_manager) => file_manager.close(shared),
            Self::Editor(editor) => editor.close(shared),
        }
    }
}
