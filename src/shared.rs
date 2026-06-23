use std::vec::Drain;

use eframe::egui::{self, Context, Ui, WidgetText};
use egui_tiles::{Container, SimplificationOptions, Tile, TileId, Tiles, Tree, UiResponse};

pub struct ViewContainer<T, P>
where
    P: View<T>,
{
    pub behavior: ViewContainerBehavior<T>,
    tree: Tree<P>,
    pane_list: Vec<TileId>,
}

pub struct ViewContainerBehavior<T> {
    pub shared: T,
    focus: Option<TileId>,
}

impl<T, P> egui_tiles::Behavior<P> for ViewContainerBehavior<T>
where
    P: View<T>,
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
        //todo!()
    }
    fn on_tab_close(&mut self, tiles: &mut Tiles<P>, tile_id: TileId) -> bool {
        if let Some(Tile::Pane(pane)) = tiles.get_mut(tile_id) {
            pane.close(&mut self.shared)
        } else {
            false
        }
    }
}

impl<T, P> ViewContainer<T, P>
where
    P: View<T>,
{
    pub fn new(tree: Tree<P>, shared: T) -> Self {
        let mut view_container = Self {
            behavior: ViewContainerBehavior {
                shared,
                focus: None,
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
    pub fn logic(&mut self, ctx: &Context) {
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
    pub fn close(&mut self) {
        for tile_id in self.pane_list.iter().copied() {
            if let Some(Tile::Pane(pane)) = self.tree.tiles.get_mut(tile_id) {
                pane.close(&mut self.behavior.shared);
            }
        }
    }
}

pub trait View<T> {
    fn title(&self) -> WidgetText;
    fn closable(&self) -> bool {
        false
    }

    fn logic(&mut self, shared: &mut T, ctx: &Context);
    fn modals(&mut self, shared: &mut T, ctx: &Context) -> bool;
    fn ui(&mut self, shared: &mut T, ui: &mut Ui);

    #[allow(unused_variables)]
    fn close(&mut self, shared: &mut T) -> bool {
        false
    }
}
