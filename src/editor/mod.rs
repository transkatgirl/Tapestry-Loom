use std::path::PathBuf;

use eframe::egui::{CentralPanel, Context, Frame, Ui, WidgetText};
use egui_tiles::{Container, Linear, LinearDir, Tabs, Tile, Tiles, Tree};
use ulid::Ulid;

use crate::{
    AppShared,
    shared::view::{View, ViewContainer},
};

pub struct Editor {
    container: ViewContainer<EditorShared, Pane>,
}

impl Editor {
    pub fn new(path: Option<PathBuf>, shared: &mut AppShared) -> Editor {
        let mut tiles = Tiles::default();

        let left_tabs = vec![
            tiles.insert_pane(Pane::Canvas),
            tiles.insert_pane(Pane::Graph),
            tiles.insert_pane(Pane::TreeList),
            tiles.insert_pane(Pane::List),
            tiles.insert_pane(Pane::BookmarkList),
        ];
        let active_left_tab = left_tabs[2];

        let left_tab_tile = tiles.insert_new(Tile::Container(Container::Tabs({
            let mut tabs = Tabs::new(left_tabs);
            tabs.set_active(active_left_tab);

            tabs
        })));

        let right = if path.is_some() {
            let right_tabs = vec![
                tiles.insert_pane(Pane::TextEdit),
                tiles.insert_pane(Pane::Menu),
                tiles.insert_pane(Pane::Info),
            ];

            tiles.insert_tab_tile(right_tabs)
        } else {
            let right_upper_tabs = vec![
                tiles.insert_pane(Pane::TextEdit),
                tiles.insert_pane(Pane::Info),
            ];

            let right_lower_tabs = vec![tiles.insert_pane(Pane::Menu)];

            let right_upper_tab_tile = tiles.insert_tab_tile(right_upper_tabs);
            let right_lower_tab_tile = tiles.insert_tab_tile(right_lower_tabs);

            tiles.insert_new(Tile::Container(Container::Linear(Linear::new_binary(
                LinearDir::Vertical,
                [right_upper_tab_tile, right_lower_tab_tile],
                0.6,
            ))))
        };

        let root = tiles.insert_horizontal_tile(vec![left_tab_tile, right]);

        let shared = EditorShared::new(path, shared);

        Editor {
            container: ViewContainer::new(
                Tree::new(format!("editor-{}-tree", shared.id), root, tiles),
                shared,
                None,
            ),
        }
    }
}

impl View<AppShared> for Editor {
    fn title(&self, shared: &AppShared) -> WidgetText {
        WidgetText::Text(self.container.behavior.shared.title(shared))
    }
    fn closable(&self, _shared: &AppShared) -> bool {
        true
    }
    fn check_close(&mut self, shared: &mut AppShared) -> bool {
        self.container.check_close() && self.container.behavior.shared.check_close(shared)
    }
    fn logic(&mut self, shared: &mut AppShared, ctx: &Context) {
        self.container.behavior.shared.logic(ctx, shared);
        self.container.logic(ctx);
    }
    fn modals(&mut self, shared: &mut AppShared, ctx: &Context) -> bool {
        let a = self.container.behavior.shared.modals(ctx, shared);
        let b = self.container.modals(ctx);

        a || b
    }
    fn ui(&mut self, shared: &mut AppShared, ui: &mut Ui) {
        self.container.behavior.shared.ui(ui, shared);

        CentralPanel::default()
            .frame(Frame::central_panel(ui.style()).inner_margin(0.0))
            .show_inside(ui, |ui| {
                self.container.ui(ui);
            });
    }
    fn save(&mut self, shared: &mut AppShared) {
        self.container.save();
        self.container.behavior.shared.save(shared);
    }
    fn close(&mut self, shared: &mut AppShared) -> bool {
        self.container.check_close()
            && self.container.behavior.shared.check_close(shared)
            && self.container.close()
            && self.container.behavior.shared.close(shared)
    }
}

struct EditorShared {
    id: Ulid,
    path: Option<PathBuf>,
}

impl EditorShared {
    fn new(path: Option<PathBuf>, shared: &mut AppShared) -> Self {
        Self {
            id: Ulid::new(),
            path,
        }
    }
    fn logic(&mut self, _ctx: &Context, shared: &mut AppShared) {}
    fn modals(&mut self, ctx: &Context, shared: &mut AppShared) -> bool {
        false
    }
    fn ui(&mut self, ui: &mut Ui, shared: &mut AppShared) {}

    fn save(&mut self, shared: &mut AppShared) {}

    fn title(&self, shared: &AppShared) -> String {
        match &self.path {
            Some(path) => {
                if let Some(filename) = path.file_stem() {
                    filename.to_string_lossy().to_string()
                } else {
                    "Untitled Weave".to_string()
                }
            }
            None => "New Weave".to_string(),
        }
    }
    fn check_close(&mut self, shared: &mut AppShared) -> bool {
        true
    }
    fn close(&mut self, shared: &mut AppShared) -> bool {
        true
    }
}

#[derive(Debug)]
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

impl View<EditorShared> for Pane {
    fn title(&self, shared: &EditorShared) -> WidgetText {
        //todo!()
        WidgetText::Text(format!("{:?}", self))
    }
    fn logic(&mut self, shared: &mut EditorShared, ctx: &Context) {}
    fn modals(&mut self, shared: &mut EditorShared, ctx: &Context) -> bool {
        false
    }
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {}
}
