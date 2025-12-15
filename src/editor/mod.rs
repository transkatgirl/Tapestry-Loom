use std::{
    cell::RefCell,
    collections::HashSet,
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    time::Instant,
};

mod canvas;
mod graph;
mod lists;
mod menus;
mod shared;
mod textedit;

// TODO: Implement node search

use eframe::egui::{
    Align, Layout, Modal, OutputCommand, Sides, Spinner, TopBottomPanel, Ui, WidgetText,
};
use egui_notify::Toasts;
use egui_tiles::{
    Behavior, Container, SimplificationOptions, Tabs, Tile, TileId, Tiles, Tree, UiResponse,
};
use flagset::FlagSet;
use log::{debug, error};
use parking_lot::Mutex;
use tapestry_weave::{
    VERSIONED_WEAVE_FILE_EXTENSION, VersionedWeave, ulid::Ulid, universal_weave::rkyv::rancor,
};
use threadpool::ThreadPool;
use tokio::runtime::Runtime;

use crate::{
    editor::{
        canvas::CanvasView,
        graph::GraphView,
        lists::{BookmarkListView, ListView, TreeListView},
        menus::{InfoView, MenuView},
        shared::{SharedState, weave::WeaveWrapper},
        textedit::TextEditorView,
    },
    settings::{Settings, inference::InferenceClient, shortcuts::Shortcuts},
};

pub struct Editor {
    settings: Rc<RefCell<Settings>>,
    toasts: Rc<RefCell<Toasts>>,
    threadpool: Rc<ThreadPool>,
    open_documents: Rc<RefCell<HashSet<PathBuf>>>,
    pub title: String,
    path: Arc<Mutex<Option<PathBuf>>>,
    old_path: Option<PathBuf>,
    weave: Arc<Mutex<Option<WeaveWrapper>>>,
    error_channel: (Arc<Sender<String>>, Receiver<String>),
    last_save: Instant,
    last_filesize: Arc<AtomicUsize>,
    panel_identifier: String,
    modal_identifier: String,
    show_modal: bool,
    save_as_input_box: String,
    tree: Tree<Pane>,
    behavior: EditorTilingBehavior,
    show_confirmation: bool,
    allow_close: bool,
    new_path_callback: Box<dyn FnMut(&PathBuf)>,
}

impl Editor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        settings: Rc<RefCell<Settings>>,
        toasts: Rc<RefCell<Toasts>>,
        threadpool: Rc<ThreadPool>,
        open_documents: Rc<RefCell<HashSet<PathBuf>>>,
        runtime: Arc<Runtime>,
        client: Rc<RefCell<Option<InferenceClient>>>,
        path: Option<PathBuf>,
        new_path_callback: Box<dyn FnMut(&PathBuf)>,
    ) -> Self {
        if let Some(path) = &path {
            open_documents.borrow_mut().insert(path.clone());
        }

        let (sender, receiver) = mpsc::channel();

        let identifier = Ulid::new();
        let identifier_string = identifier.to_string();

        let mut tiles = Tiles::default();

        let left_tabs = vec![
            tiles.insert_pane(Pane::Canvas),
            tiles.insert_pane(Pane::Graph),
            tiles.insert_pane(Pane::TreeList),
            tiles.insert_pane(Pane::List),
            tiles.insert_pane(Pane::BookmarkList),
        ];
        let active_left_tab = left_tabs[2];

        let right_tabs = vec![
            tiles.insert_pane(Pane::TextEdit),
            tiles.insert_pane(Pane::Menu),
            tiles.insert_pane(Pane::Info),
        ];

        let left_tab_tile = tiles.insert_new(Tile::Container(Container::Tabs({
            let mut tabs = Tabs::new(left_tabs);
            tabs.set_active(active_left_tab);

            tabs
        })));

        let right_tab_tile = tiles.insert_tab_tile(right_tabs);

        let root = tiles.insert_horizontal_tile(vec![left_tab_tile, right_tab_tile]);

        let weave = Arc::new(Mutex::new(None));

        let shared_state = SharedState::new(identifier, runtime, client, &settings.borrow());

        Self {
            settings: settings.clone(),
            toasts: toasts.clone(),
            threadpool,
            open_documents,
            title: generate_title(&path),
            path: Arc::new(Mutex::new(path.clone())),
            old_path: path,
            weave: weave.clone(),
            error_channel: (Arc::new(sender), receiver),
            last_save: Instant::now(),
            last_filesize: Arc::new(AtomicUsize::new(0)),
            panel_identifier: ["editor-", &identifier_string, "-bottom-panel"].concat(),
            modal_identifier: ["editor-", &identifier_string, "-modal"].concat(),
            show_modal: false,
            save_as_input_box: ["Untitled.", VERSIONED_WEAVE_FILE_EXTENSION].concat(),
            tree: Tree::new(
                ["editor-", &identifier_string, "-tree"].concat(),
                root,
                tiles,
            ),
            behavior: EditorTilingBehavior {
                settings,
                toasts,
                weave,
                shared_state,
                canvas_view: CanvasView::default(),
                graph_view: GraphView::default(),
                tree_list_view: TreeListView::default(),
                list_view: ListView::default(),
                bookmark_list_view: BookmarkListView::default(),
                text_edit_view: TextEditorView::default(),
                menu_view: MenuView::default(),
                info_view: InfoView::default(),
                shortcuts: FlagSet::default(),
            },
            allow_close: false,
            show_confirmation: false,
            new_path_callback,
        }
    }
    pub fn render(
        &mut self,
        ui: &mut Ui,
        mut close_callback: impl FnMut(),
        shortcuts: FlagSet<Shortcuts>,
    ) {
        self.behavior.shortcuts = shortcuts;

        if self.show_confirmation
            && Modal::new("global-close-modal".into())
                .show(ui.ctx(), |ui| {
                    ui.set_width(210.0);
                    ui.heading("Do you want to close this weave without saving?");
                    ui.label("All changes made will be lost.");
                    ui.add_space(ui.style().spacing.menu_spacing);
                    Sides::new().show(
                        ui,
                        |_ui| {},
                        |ui| {
                            if ui.button("Yes").clicked() {
                                self.allow_close = true;
                                close_callback();
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

        if let Some(mut weave) = self.weave.clone().try_lock() {
            match weave.as_mut() {
                Some(_) => {
                    drop(weave);
                    self.render_weave(ui);

                    let mut toasts = self.toasts.borrow_mut();
                    while let Ok(message) = self.error_channel.1.try_recv() {
                        toasts.error(message);
                    }

                    return;
                }
                None => {
                    drop(weave);
                    let weave = self.weave.clone();
                    let barrier = Arc::new(Barrier::new(2));
                    let thread_barrier = barrier.clone();
                    let path = self.path.clone();
                    let error_sender = self.error_channel.0.clone();
                    let file_size = self.last_filesize.clone();

                    self.threadpool.execute(move || {
                        let mut weave_dest = weave.lock();
                        let mut path = path.lock();
                        thread_barrier.wait();

                        if let Some(filepath) = path.as_deref() {
                            match read_bytes(filepath) {
                                Ok(bytes) => match VersionedWeave::from_bytes(&bytes) {
                                    Some(Ok(weave)) => {
                                        file_size.store(bytes.len(), Ordering::SeqCst);
                                        let mut weave = weave.into_latest();
                                        weave.reserve(65536_usize.saturating_sub(weave.capacity()));
                                        *weave_dest = Some(weave.into());
                                    }
                                    Some(Err(error)) => {
                                        file_size.store(0, Ordering::SeqCst);
                                        let _ = error_sender
                                            .send("Weave deserialization failed".to_string());
                                        error!("Weave deserialization failed: {:#?}", error);
                                        *path = None;
                                        *weave_dest = Some(WeaveWrapper::default());
                                    }
                                    None => {
                                        file_size.store(0, Ordering::SeqCst);
                                        let _ =
                                            error_sender.send("Invalid weave header".to_string());
                                        error!("Invalid weave header");
                                        *path = None;
                                        *weave_dest = Some(WeaveWrapper::default());
                                    }
                                },
                                Err(error) => {
                                    file_size.store(0, Ordering::SeqCst);
                                    let _ = error_sender.send(format!("Filesystem error: {error}"));
                                    error!("Filesystem error: {:#?}", error);
                                    *path = None;
                                    *weave_dest = Some(WeaveWrapper::default());
                                }
                            }
                        } else {
                            *weave_dest = Some(WeaveWrapper::default());
                        }
                    });
                    barrier.wait();
                }
            }
        }

        TopBottomPanel::bottom(self.panel_identifier.clone()).show_animated_inside(
            ui,
            true,
            |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        ui.add(Spinner::new());
                        ui.label("Loading weave...");
                    });
                });
            },
        );

        let mut toasts = self.toasts.borrow_mut();
        while let Ok(message) = self.error_channel.1.try_recv() {
            toasts.error(message);
        }
    }
    fn render_weave(&mut self, ui: &mut Ui) {
        let settings = self.settings.borrow();
        let mut path = self.path.lock();

        if self.old_path != *path {
            self.title = generate_title(&path);
            if let Some(path) = &self.old_path {
                self.open_documents.borrow_mut().remove(path);
            }
            if let Some(path) = path.as_ref() {
                self.open_documents.borrow_mut().insert(path.clone());
                (self.new_path_callback)(path);
            }
            self.old_path = path.clone();
            //self.last_filesize.store(0, Ordering::SeqCst);
            //self.behavior.reset();
        }

        TopBottomPanel::bottom(self.panel_identifier.clone()).show_animated_inside(
            ui,
            true,
            |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        if let Some(path) = path.as_ref() {
                            let label = if let Ok(short_path) =
                                path.strip_prefix(&settings.documents.location)
                            {
                                ui.label(short_path.to_string_lossy())
                            } else {
                                ui.label(path.to_string_lossy())
                            };
                            label
                                .on_hover_text(path.to_string_lossy())
                                .context_menu(|ui| {
                                    if ui.button("Copy path").clicked() {
                                        ui.output_mut(|o| {
                                            o.commands.push(OutputCommand::CopyText(
                                                path.to_string_lossy().to_string(),
                                            ))
                                        });
                                    };
                                });
                        } else if ui.button("Save As...").clicked() {
                            self.show_modal = true;
                        }
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        self.behavior
                            .panel_rtl(ui, self.last_filesize.load(Ordering::Relaxed));
                    });
                });
            },
        );

        self.tree.ui(&mut self.behavior, ui);

        if self.show_modal
            && Modal::new(self.modal_identifier.clone().into())
                .show(ui.ctx(), |ui| {
                    ui.set_width(280.0);
                    ui.heading("Save Weave");
                    let label = ui.label("Path:");
                    ui.text_edit_singleline(&mut self.save_as_input_box)
                        .labelled_by(label.id);
                    Sides::new().show(
                        ui,
                        |_ui| {},
                        |ui| {
                            if ui.button("Cancel").clicked() {
                                ui.close();
                            }
                            if ui.button("Save").clicked() && !self.save_as_input_box.is_empty() {
                                let mut new_path =
                                    settings.documents.location.join(&self.save_as_input_box);
                                if new_path.extension().is_none() {
                                    new_path.set_extension("tapestry");
                                }
                                if !self.open_documents.borrow().contains(&new_path)
                                    && !fs::exists(&new_path).unwrap_or(true)
                                {
                                    *path = Some(new_path);
                                    ui.close();
                                }
                            }
                        },
                    );
                })
                .should_close()
        {
            self.show_modal = false;
            if path.is_some() {
                drop(path);
                self.save(false);
            } else {
                drop(path);
            }
        } else {
            drop(path);
        }

        if self.last_save.elapsed() > settings.documents.save_interval {
            self.last_save = Instant::now();
            self.save(false);
            ui.ctx()
                .request_repaint_after_secs(settings.documents.save_interval.as_secs_f32() + 0.5);
        }
    }
    fn save(&self, unload: bool) {
        let weave = self.weave.clone();
        let path = self.path.clone();
        let error_sender = self.error_channel.0.clone();
        let barrier = Arc::new(Barrier::new(2));
        let thread_barrier = barrier.clone();
        let file_size = self.last_filesize.clone();

        self.threadpool.execute(move || {
            let mut weave_lock = weave.lock();
            let mut path_lock = path.lock();
            thread_barrier.wait();

            if let Some(path) = path_lock.as_ref()
                && let Some(weave) = weave_lock.as_ref()
            {
                match weave.to_versioned_bytes() {
                    Ok(bytes) => {
                        if let Err(error) = write_bytes(path, &bytes) {
                            file_size.store(0, Ordering::SeqCst);
                            let _ = error_sender.send(format!("Filesystem error: {error}"));
                            error!("Filesystem error: {:#?}", error);
                            *path_lock = None;
                        } else {
                            file_size.store(bytes.len(), Ordering::SeqCst);
                            debug!("Saved weave {} to disk", path.to_string_lossy());
                            if unload {
                                *weave_lock = None;
                                *path_lock = None;
                            }
                        }
                    }
                    Err(error) => {
                        file_size.store(0, Ordering::SeqCst);
                        let _ = error_sender.send("Weave serialization failed".to_string());
                        error!("Weave serialization failed: {:#?}", error);
                    }
                }
            }
        });
        barrier.wait();
    }
    pub fn unsaved(&self) -> bool {
        self.path.lock().is_none()
            && self
                .weave
                .try_lock()
                .and_then(|weave| {
                    weave
                        .as_ref()
                        .map(|weave| !weave.is_empty_including_metadata())
                })
                .unwrap_or(true)
    }
    pub fn close(&mut self) -> bool {
        if let Some(path) = &self.path.lock().as_ref() {
            self.open_documents.borrow_mut().remove(*path);
        } else if !self.allow_close
            && self
                .weave
                .try_lock()
                .and_then(|weave| {
                    weave
                        .as_ref()
                        .map(|weave| !weave.is_empty_including_metadata())
                })
                .unwrap_or(true)
        {
            self.show_confirmation = true;
            return false;
        }

        self.save(true);
        true
    }
}

fn generate_title(path: &Option<PathBuf>) -> String {
    match path {
        Some(path) => {
            if let Some(filename) = path.file_stem() {
                filename.to_string_lossy().to_string()
            } else {
                "Editor".to_string()
            }
        }
        None => "New Weave".to_string(),
    }
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, io::Error> {
    let mut file = File::open(path)?;
    file.lock()?;

    let size = file
        .metadata()
        .map(|m| m.len() as usize)
        .unwrap_or_default();

    let mut bytes = Vec::with_capacity(size);
    file.read_to_end(&mut bytes)?;

    Ok(bytes)
}

fn write_bytes(path: &Path, contents: &[u8]) -> Result<(), io::Error> {
    let mut file = File::create(path)?;
    file.lock()?;

    file.write_all(contents)
}

pub fn blank_weave_bytes() -> Result<Vec<u8>, rancor::Error> {
    WeaveWrapper::default().to_versioned_bytes()
}

enum Pane {
    Canvas,
    Graph,
    TreeList,
    List,
    BookmarkList,
    TextEdit,
    Menu,
    Info,
}

struct EditorTilingBehavior {
    shared_state: SharedState,
    canvas_view: CanvasView,
    graph_view: GraphView,
    tree_list_view: TreeListView,
    list_view: ListView,
    bookmark_list_view: BookmarkListView,
    text_edit_view: TextEditorView,
    menu_view: MenuView,
    info_view: InfoView,
    settings: Rc<RefCell<Settings>>,
    toasts: Rc<RefCell<Toasts>>,
    weave: Arc<Mutex<Option<WeaveWrapper>>>,
    shortcuts: FlagSet<Shortcuts>,
}

// TODO: Set drag_preview_color, tab_bar_color, and tab_bg_color

impl EditorTilingBehavior {
    /*fn reset(&mut self) {
        self.shared_state.reset();
        self.canvas_view.reset();
        self.graph_view.reset();
        self.tree_list_view.reset();
        self.list_view.reset();
        self.bookmark_list_view.reset();
        self.text_edit_view.reset();
        self.menu_view.reset();
    }*/
    fn panel_rtl(&mut self, ui: &mut Ui, file_size: usize) {
        let mut weave = self.weave.lock();

        if let Some(weave) = weave.as_mut() {
            let settings = self.settings.borrow();
            let mut toasts = self.toasts.borrow_mut();
            self.shared_state
                .update(ui.ctx(), weave, &settings, &mut toasts, self.shortcuts);
            self.canvas_view.update(
                weave,
                &settings,
                &mut toasts,
                &mut self.shared_state,
                self.shortcuts,
            );
            self.graph_view.update(
                weave,
                &settings,
                &mut toasts,
                &mut self.shared_state,
                self.shortcuts,
            );
            self.tree_list_view.update(
                ui,
                weave,
                &settings,
                &mut toasts,
                &mut self.shared_state,
                self.shortcuts,
            );
            self.list_view.update(
                weave,
                &settings,
                &mut toasts,
                &mut self.shared_state,
                self.shortcuts,
            );
            self.bookmark_list_view.update(
                weave,
                &settings,
                &mut toasts,
                &mut self.shared_state,
                self.shortcuts,
            );
            self.text_edit_view.update(
                weave,
                &settings,
                &mut toasts,
                &mut self.shared_state,
                self.shortcuts,
            );
            self.info_view.update(
                weave,
                &settings,
                &mut toasts,
                &mut self.shared_state,
                self.shortcuts,
            );
            self.menu_view.render_rtl_panel(
                ui,
                weave,
                &settings,
                &mut toasts,
                &mut self.shared_state,
                self.shortcuts,
                file_size,
            );
        }
    }
}

impl Behavior<Pane> for EditorTilingBehavior {
    fn pane_ui(&mut self, ui: &mut Ui, _tile_id: TileId, pane: &mut Pane) -> UiResponse {
        let mut weave = self.weave.lock();

        if let Some(weave) = weave.as_mut() {
            let settings = self.settings.borrow();
            let mut toasts = self.toasts.borrow_mut();

            match pane {
                Pane::Canvas => self.canvas_view.render(
                    ui,
                    weave,
                    &settings,
                    &mut toasts,
                    &mut self.shared_state,
                    self.shortcuts,
                ),
                Pane::Graph => self.graph_view.render(
                    ui,
                    weave,
                    &settings,
                    &mut toasts,
                    &mut self.shared_state,
                    self.shortcuts,
                ),
                Pane::TreeList => self.tree_list_view.render(
                    ui,
                    weave,
                    &settings,
                    &mut toasts,
                    &mut self.shared_state,
                    self.shortcuts,
                ),
                Pane::List => self.list_view.render(
                    ui,
                    weave,
                    &settings,
                    &mut toasts,
                    &mut self.shared_state,
                    self.shortcuts,
                ),
                Pane::BookmarkList => self.bookmark_list_view.render(
                    ui,
                    weave,
                    &settings,
                    &mut toasts,
                    &mut self.shared_state,
                    self.shortcuts,
                ),
                Pane::TextEdit => self.text_edit_view.render(
                    ui,
                    weave,
                    &settings,
                    &mut toasts,
                    &mut self.shared_state,
                    self.shortcuts,
                ),
                Pane::Menu => self.menu_view.render(
                    ui,
                    weave,
                    &settings,
                    &mut toasts,
                    &mut self.shared_state,
                    self.shortcuts,
                ),
                Pane::Info => self.info_view.render(
                    ui,
                    weave,
                    &settings,
                    &mut toasts,
                    &mut self.shared_state,
                    self.shortcuts,
                ),
            }
        }

        UiResponse::None
    }
    fn tab_title_for_pane(&mut self, pane: &Pane) -> WidgetText {
        match pane {
            Pane::Canvas => WidgetText::Text("\u{E125} Canvas".to_string()),
            Pane::Graph => WidgetText::Text("\u{E52E} Graph".to_string()),
            Pane::TreeList => WidgetText::Text("\u{E408} Tree".to_string()),
            Pane::List => WidgetText::Text("\u{E106} List".to_string()),
            Pane::BookmarkList => WidgetText::Text("\u{E060} Bookmarks".to_string()),
            Pane::TextEdit => WidgetText::Text("\u{E265} Editor".to_string()),
            Pane::Menu => WidgetText::Text("\u{E1B1} Menu".to_string()),
            Pane::Info => WidgetText::Text("\u{E0F9} Info".to_string()),
        }
    }
    fn is_tab_closable(&self, _tiles: &Tiles<Pane>, _tile_id: TileId) -> bool {
        false
    }
    fn simplification_options(&self) -> SimplificationOptions {
        SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }
}
