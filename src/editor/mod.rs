use std::path::PathBuf;

use eframe::egui::{CentralPanel, Context, Frame, Ui, WidgetText};
use egui_tiles::{Container, Linear, LinearDir, Tabs, Tile, Tiles, Tree};

mod shared;
mod subviews;

pub use shared::preload;

use crate::{
    AppShared,
    common::view::{View, ViewContainer},
    editor::{
        shared::EditorShared,
        subviews::{
            canvas::CanvasView,
            graph::GraphView,
            lists::{BookmarkView, ListView, TreeListView},
            menus::{InfoView, MenuView},
            textedit::TextEditView,
        },
    },
};

pub struct Editor {
    container: ViewContainer<EditorShared, Pane>,
}

impl From<EditorShared> for Editor {
    fn from(value: EditorShared) -> Self {
        let mut tiles = Tiles::default();

        let left_tabs = vec![
            tiles.insert_pane(Pane::Canvas(CanvasView::default())),
            tiles.insert_pane(Pane::Graph(GraphView::default())),
            tiles.insert_pane(Pane::TreeList(TreeListView::default())),
            tiles.insert_pane(Pane::List(ListView::default())),
            tiles.insert_pane(Pane::BookmarkList(BookmarkView::default())),
        ];
        let active_left_tab = left_tabs[2];

        let left_tab_tile = tiles.insert_new(Tile::Container(Container::Tabs({
            let mut tabs = Tabs::new(left_tabs);
            tabs.set_active(active_left_tab);

            tabs
        })));

        let right = if value.path().is_some() {
            let right_tabs = vec![
                tiles.insert_pane(Pane::TextEdit(TextEditView::default())),
                tiles.insert_pane(Pane::Menu(MenuView::default())),
                tiles.insert_pane(Pane::Info(InfoView::default())),
            ];

            tiles.insert_tab_tile(right_tabs)
        } else {
            let right_upper_tabs = vec![
                tiles.insert_pane(Pane::TextEdit(TextEditView::default())),
                tiles.insert_pane(Pane::Info(InfoView::default())),
            ];

            let right_lower_tabs = vec![tiles.insert_pane(Pane::Menu(MenuView::default()))];

            let right_upper_tab_tile = tiles.insert_tab_tile(right_upper_tabs);
            let right_lower_tab_tile = tiles.insert_tab_tile(right_lower_tabs);

            tiles.insert_new(Tile::Container(Container::Linear(Linear::new_binary(
                LinearDir::Vertical,
                [right_upper_tab_tile, right_lower_tab_tile],
                0.6,
            ))))
        };

        let root = tiles.insert_horizontal_tile(vec![left_tab_tile, right]);

        Self {
            container: ViewContainer::new(
                Tree::new(
                    ["editor-", &value.id.to_string(), "-tree"].concat(),
                    root,
                    tiles,
                ),
                value,
                None,
            ),
        }
    }
}

impl Editor {
    pub fn new(path: Option<PathBuf>, shared: &mut AppShared) -> Self {
        let shared = EditorShared::new(path, shared);
        Self::from(shared)
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

#[derive(Debug)]
enum Pane {
    Canvas(CanvasView),
    Graph(GraphView),
    TreeList(TreeListView),
    List(ListView),
    BookmarkList(BookmarkView),
    TextEdit(TextEditView),
    Menu(MenuView),
    Info(InfoView),
}

impl View<EditorShared> for Pane {
    fn title(&self, shared: &EditorShared) -> WidgetText {
        match self {
            Self::Canvas(view) => view.title(shared),
            Self::Graph(view) => view.title(shared),
            Self::TreeList(view) => view.title(shared),
            Self::List(view) => view.title(shared),
            Self::BookmarkList(view) => view.title(shared),
            Self::TextEdit(view) => view.title(shared),
            Self::Menu(view) => view.title(shared),
            Self::Info(view) => view.title(shared),
        }
    }
    fn closable(&self, shared: &EditorShared) -> bool {
        match self {
            Self::Canvas(view) => view.closable(shared),
            Self::Graph(view) => view.closable(shared),
            Self::TreeList(view) => view.closable(shared),
            Self::List(view) => view.closable(shared),
            Self::BookmarkList(view) => view.closable(shared),
            Self::TextEdit(view) => view.closable(shared),
            Self::Menu(view) => view.closable(shared),
            Self::Info(view) => view.closable(shared),
        }
    }
    fn check_close(&mut self, shared: &mut EditorShared) -> bool {
        match self {
            Self::Canvas(view) => view.check_close(shared),
            Self::Graph(view) => view.check_close(shared),
            Self::TreeList(view) => view.check_close(shared),
            Self::List(view) => view.check_close(shared),
            Self::BookmarkList(view) => view.check_close(shared),
            Self::TextEdit(view) => view.check_close(shared),
            Self::Menu(view) => view.check_close(shared),
            Self::Info(view) => view.check_close(shared),
        }
    }
    fn logic(&mut self, shared: &mut EditorShared, ctx: &Context) {
        match self {
            Self::Canvas(view) => view.logic(shared, ctx),
            Self::Graph(view) => view.logic(shared, ctx),
            Self::TreeList(view) => view.logic(shared, ctx),
            Self::List(view) => view.logic(shared, ctx),
            Self::BookmarkList(view) => view.logic(shared, ctx),
            Self::TextEdit(view) => view.logic(shared, ctx),
            Self::Menu(view) => view.logic(shared, ctx),
            Self::Info(view) => view.logic(shared, ctx),
        }
    }
    fn modals(&mut self, shared: &mut EditorShared, ctx: &Context) -> bool {
        match self {
            Self::Canvas(view) => view.modals(shared, ctx),
            Self::Graph(view) => view.modals(shared, ctx),
            Self::TreeList(view) => view.modals(shared, ctx),
            Self::List(view) => view.modals(shared, ctx),
            Self::BookmarkList(view) => view.modals(shared, ctx),
            Self::TextEdit(view) => view.modals(shared, ctx),
            Self::Menu(view) => view.modals(shared, ctx),
            Self::Info(view) => view.modals(shared, ctx),
        }
    }
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {
        match self {
            Self::Canvas(view) => view.ui(shared, ui),
            Self::Graph(view) => view.ui(shared, ui),
            Self::TreeList(view) => view.ui(shared, ui),
            Self::List(view) => view.ui(shared, ui),
            Self::BookmarkList(view) => view.ui(shared, ui),
            Self::TextEdit(view) => view.ui(shared, ui),
            Self::Menu(view) => view.ui(shared, ui),
            Self::Info(view) => view.ui(shared, ui),
        }
    }
    fn save(&mut self, shared: &mut EditorShared) {
        match self {
            Self::Canvas(view) => view.save(shared),
            Self::Graph(view) => view.save(shared),
            Self::TreeList(view) => view.save(shared),
            Self::List(view) => view.save(shared),
            Self::BookmarkList(view) => view.save(shared),
            Self::TextEdit(view) => view.save(shared),
            Self::Menu(view) => view.save(shared),
            Self::Info(view) => view.save(shared),
        }
    }
    fn close(&mut self, shared: &mut EditorShared) -> bool {
        match self {
            Self::Canvas(view) => view.close(shared),
            Self::Graph(view) => view.close(shared),
            Self::TreeList(view) => view.close(shared),
            Self::List(view) => view.close(shared),
            Self::BookmarkList(view) => view.close(shared),
            Self::TextEdit(view) => view.close(shared),
            Self::Menu(view) => view.close(shared),
            Self::Info(view) => view.close(shared),
        }
    }
}
