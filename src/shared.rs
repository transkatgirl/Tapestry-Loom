use eframe::egui::{Context, Ui, WidgetText};
use egui_tiles::{Container, SimplificationOptions, Tile, TileId, Tiles, Tree, UiResponse};

pub struct ViewContainer<T, P, F>
where
    P: View<T>,
    F: Fn(&mut T) -> Option<P>,
{
    pub behavior: ViewContainerBehavior<T, P, F>,
    tree: Tree<P>,
    pane_list: Vec<TileId>,
}

pub struct ViewContainerBehavior<T, P, F>
where
    P: View<T>,
    F: Fn(&mut T) -> Option<P>,
{
    pub shared: T,
    pub creation_callback: Option<F>,
    focus: Option<TileId>,
    create: Option<Option<TileId>>,
    add: Vec<TileId>,
}

impl<T, P, F> egui_tiles::Behavior<P> for ViewContainerBehavior<T, P, F>
where
    P: View<T>,
    F: Fn(&mut T) -> Option<P>,
{
    fn tab_title_for_pane(&mut self, pane: &P) -> WidgetText {
        pane.title()
    }
    fn pane_ui(&mut self, ui: &mut Ui, tile_id: TileId, pane: &mut P) -> UiResponse {
        if pane.modals(&mut self.shared, ui) {
            self.focus = Some(tile_id);
        }

        pane.ui(&mut self.shared, ui);

        UiResponse::None
    }
    fn is_tab_closable(&self, tiles: &Tiles<P>, tile_id: TileId) -> bool {
        if let Some(Tile::Pane(pane)) = tiles.get(tile_id) {
            pane.closable()
        } else {
            false
        }
    }
    fn simplification_options(&self) -> SimplificationOptions {
        SimplificationOptions {
            all_panes_must_have_tabs: true,
            prune_single_child_containers: true,
            ..Default::default()
        }
    }
    fn top_bar_right_ui(
        &mut self,
        _tiles: &Tiles<P>,
        ui: &mut Ui,
        tile_id: TileId,
        _tabs: &egui_tiles::Tabs,
        _scroll_offset: &mut f32,
    ) {
        if self.creation_callback.is_some() && ui.button("\u{E13D}").clicked() {
            self.create = Some(Some(tile_id));
        }
    }
    fn on_tab_close(&mut self, tiles: &mut Tiles<P>, tile_id: TileId) -> bool {
        if let Some(Tile::Pane(pane)) = tiles.get_mut(tile_id) {
            pane.close(&mut self.shared)
        } else {
            false
        }
    }
}

impl<T, P, F> ViewContainer<T, P, F>
where
    P: View<T>,
    F: Fn(&mut T) -> Option<P>,
{
    pub fn new(tree: Tree<P>, shared: T) -> Self {
        let mut view_container = Self {
            behavior: ViewContainerBehavior {
                shared,
                creation_callback: None,
                focus: None,
                create: None,
                add: Vec::with_capacity(1),
            },
            tree,
            pane_list: Vec::new(),
        };
        view_container.refresh_pane_list();

        view_container
    }
    fn refresh_pane_list(&mut self) {
        self.pane_list.clear();
        self.pane_list
            .extend(self.tree.tiles.iter().filter_map(|(tile_id, tile)| {
                if tile.is_pane() { Some(tile_id) } else { None }
            }));
        self.pane_list.sort_unstable_by_key(|a| a.0);
    }
    pub fn add_pane(&mut self, pane: P) {
        self.behavior
            .add
            .push(self.tree.tiles.insert_new(Tile::Pane(pane)));
    }
    pub fn logic(&mut self, ctx: &Context) {
        if let Some(create) = self.behavior.create {
            if let Some(callback) = &self.behavior.creation_callback
                && let Some(pane) = callback(&mut self.behavior.shared)
            {
                let tile_id = self.tree.tiles.insert_new(Tile::Pane(pane));

                if let Some(create) = create
                    && let Some(Tile::Container(parent)) = self.tree.tiles.get_mut(create)
                {
                    parent.add_child(tile_id);
                    if let egui_tiles::Container::Tabs(tabs) = parent {
                        tabs.set_active(tile_id);
                    }
                } else if let Some(root) = self.tree.root
                    && let Some(Tile::Container(root)) = self.tree.tiles.get_mut(root)
                {
                    root.add_child(tile_id);
                    if let egui_tiles::Container::Tabs(tabs) = root {
                        tabs.set_active(tile_id);
                    }
                }
            }
            self.behavior.create = None;
        }

        if !self.behavior.add.is_empty() {
            if let Some(root) = self.tree.root
                && let Some(Tile::Container(root)) = self.tree.tiles.get_mut(root)
            {
                for tile_id in self.behavior.add.drain(..) {
                    root.add_child(tile_id);
                    if let egui_tiles::Container::Tabs(tabs) = root {
                        tabs.set_active(tile_id);
                    }
                }
            } else {
                self.behavior.add.clear();
            }
        }

        if let Some(tile_id) = self.behavior.focus {
            if let Some(parent_id) = self.tree.tiles.parent_of(tile_id)
                && let Some(Tile::Container(Container::Tabs(tabs))) =
                    self.tree.tiles.get_mut(parent_id)
            {
                tabs.set_active(tile_id);
            }

            self.behavior.focus = None;
        }

        for tile_id in self.pane_list.iter().copied() {
            if let Some(Tile::Pane(pane)) = self.tree.tiles.get_mut(tile_id) {
                pane.logic(&mut self.behavior.shared, ctx);
            }
        }
    }
    pub fn modals(&mut self, ctx: &Context) {
        for tile_id in self.pane_list.iter().copied() {
            if let Some(Tile::Pane(pane)) = self.tree.tiles.get_mut(tile_id) {
                pane.modals(&mut self.behavior.shared, ctx);
            }
        }
    }
    pub fn ui(&mut self, ui: &mut Ui) {
        self.tree.ui(&mut self.behavior, ui);
        self.refresh_pane_list();
    }
    pub fn close(&mut self) -> bool {
        let mut would_close = true;

        for tile_id in self.pane_list.iter().copied() {
            if let Some(Tile::Pane(pane)) = self.tree.tiles.get_mut(tile_id)
                && !pane.check_close()
            {
                would_close = false;
            }
        }

        if would_close {
            for tile_id in self.pane_list.iter().copied() {
                if let Some(Tile::Pane(pane)) = self.tree.tiles.get_mut(tile_id) {
                    pane.close(&mut self.behavior.shared);
                }
            }
        }

        would_close
    }
}

pub trait View<T> {
    fn title(&self) -> WidgetText;
    fn closable(&self) -> bool {
        false
    }
    fn check_close(&mut self) -> bool {
        true
    }

    fn logic(&mut self, shared: &mut T, ctx: &Context);
    fn modals(&mut self, shared: &mut T, ctx: &Context) -> bool;
    fn ui(&mut self, shared: &mut T, ui: &mut Ui);

    #[allow(unused_variables)]
    fn close(&mut self, shared: &mut T) -> bool {
        false
    }
}
