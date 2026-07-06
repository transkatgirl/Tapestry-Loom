use eframe::egui::{Context, Ui, WidgetText};
use egui_tiles::{Behavior, SimplificationOptions, Tabs, Tile, TileId, Tiles, Tree, UiResponse};
use log::warn;

pub struct ViewContainer<T, P>
where
    P: View<T>,
{
    pub behavior: ViewContainerBehavior<T, P>,
    tree: Tree<P>,
    pane_list: Vec<TileId>,
}

#[allow(clippy::type_complexity)]
pub struct ViewContainerBehavior<T, P>
where
    P: View<T>,
{
    pub shared: T,
    pub creation_callback: Option<Box<dyn Fn(&mut T) -> P>>,
    focus: Option<TileId>,
    create: Option<Option<TileId>>,
    add: Vec<TileId>,
    remove: Vec<TileId>,
}

impl<T, P> Behavior<P> for ViewContainerBehavior<T, P>
where
    P: View<T>,
{
    fn tab_title_for_pane(&mut self, pane: &P) -> WidgetText {
        pane.title(&self.shared)
    }
    fn pane_ui(&mut self, ui: &mut Ui, _tile_id: TileId, pane: &mut P) -> UiResponse {
        if ui.is_visible() {
            pane.ui(&mut self.shared, ui);
        }

        UiResponse::None
    }
    fn is_tab_closable(&self, tiles: &Tiles<P>, tile_id: TileId) -> bool {
        if let Some(Tile::Pane(pane)) = tiles.get(tile_id) {
            pane.closable(&self.shared)
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
        _tabs: &Tabs,
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

impl<T, P> ViewContainer<T, P>
where
    P: View<T>,
{
    #[allow(clippy::type_complexity)]
    pub fn new(
        tree: Tree<P>,
        shared: T,
        creation_callback: Option<Box<dyn Fn(&mut T) -> P>>,
    ) -> Self {
        Self {
            behavior: ViewContainerBehavior {
                shared,
                creation_callback,
                focus: None,
                create: None,
                add: Vec::with_capacity(1),
                remove: Vec::with_capacity(1),
            },
            tree,
            pane_list: Vec::new(),
        }
    }
    pub fn is_empty(&self) -> bool {
        !self.tree.tiles.tiles().any(|t| t.is_pane())
            && (self.behavior.create.is_none() || self.behavior.creation_callback.is_none())
            && self.behavior.add.is_empty()
    }
    fn update_pane_list_display_order(&mut self) {
        self.pane_list.clear();
        if let Some(root) = self.tree.root {
            build_tree_pane_list(&self.tree, &mut self.pane_list, root);
        } else if !self.tree.tiles.is_empty() {
            warn!("Tree {:?} contains no root tile", self.tree.id());
        }
    }
    fn update_pane_list_id_order(&mut self) {
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
        let original_focus = self.behavior.focus;

        if let Some(create) = self.behavior.create {
            if let Some(callback) = &self.behavior.creation_callback {
                let tile_id = self
                    .tree
                    .tiles
                    .insert_new(Tile::Pane(callback(&mut self.behavior.shared)));

                if let Some(create) = create
                    && let Some(Tile::Container(parent)) = self.tree.tiles.get_mut(create)
                {
                    parent.add_child(tile_id);
                    self.behavior.focus = Some(tile_id);
                } else if let Some(root) = self.tree.root
                    && let Some(Tile::Container(root)) = self.tree.tiles.get_mut(root)
                {
                    root.add_child(tile_id);
                    self.behavior.focus = Some(tile_id);
                } else {
                    warn!("Created orphaned view {:?}", tile_id);
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
                    self.behavior.focus = Some(tile_id);
                }
            } else {
                warn!("Tree {:?} contains no valid root tile", self.tree.id());
                self.behavior.add.clear();
            }
        }

        if original_focus.is_some() {
            self.behavior.focus = original_focus;
        }

        if let Some(tile_id) = self.behavior.focus {
            self.tree
                .make_active(|other_tile_id, _| tile_id == other_tile_id);
            self.behavior.focus = None;
        }

        self.update_pane_list_id_order();

        for tile_id in self.pane_list.drain(..) {
            if let Some(Tile::Pane(pane)) = self.tree.tiles.get_mut(tile_id) {
                pane.logic(
                    &mut self.behavior.shared,
                    || {
                        self.behavior.remove.push(tile_id);
                    },
                    ctx,
                );
            }
        }

        for tile_id in self.behavior.remove.drain(..) {
            self.tree.remove_recursively(tile_id);
        }
    }
    pub fn modals(&mut self, ctx: &Context) -> bool {
        self.update_pane_list_display_order();

        for tile_id in self.pane_list.drain(..) {
            if ctx.will_discard() {
                break;
            }

            if let Some(Tile::Pane(pane)) = self.tree.tiles.get_mut(tile_id)
                && pane.modals(&mut self.behavior.shared, ctx)
            {
                self.behavior.focus = Some(tile_id);
            };
        }

        self.behavior.focus.is_some()
    }
    pub fn ui(&mut self, ui: &mut Ui) {
        self.tree.ui(&mut self.behavior, ui);
    }
    pub fn save(&mut self) {
        self.update_pane_list_id_order();

        for tile_id in self.pane_list.drain(..) {
            if let Some(Tile::Pane(pane)) = self.tree.tiles.get_mut(tile_id) {
                pane.save(&mut self.behavior.shared);
            }
        }
    }
    pub fn check_close(&mut self) -> bool {
        let mut would_close = true;

        self.update_pane_list_display_order();

        for tile_id in self.pane_list.iter().copied() {
            if let Some(Tile::Pane(pane)) = self.tree.tiles.get_mut(tile_id)
                && !pane.check_close(&mut self.behavior.shared)
            {
                would_close = false;
                break;
            }
        }

        would_close
    }
    pub fn close(&mut self) -> bool {
        let mut would_close = true;

        self.update_pane_list_display_order();

        for tile_id in self.pane_list.iter().copied() {
            if let Some(Tile::Pane(pane)) = self.tree.tiles.get_mut(tile_id)
                && !pane.check_close(&mut self.behavior.shared)
            {
                would_close = false;
                break;
            }
        }

        if would_close {
            for tile_id in self.pane_list.drain(..) {
                if let Some(Tile::Pane(pane)) = self.tree.tiles.get_mut(tile_id) {
                    if pane.close(&mut self.behavior.shared) {
                        self.tree.remove_recursively(tile_id);
                    } else {
                        warn!("View {:?} allowed check_close() but not close()", tile_id);
                        would_close = false;
                    }
                }
            }
        }

        would_close
    }
}

fn build_tree_pane_list<P>(tree: &Tree<P>, panes: &mut Vec<TileId>, current: TileId) {
    match tree.tiles.get(current) {
        Some(Tile::Container(container)) => {
            for child in container.children() {
                build_tree_pane_list(tree, panes, *child);
            }
        }
        Some(Tile::Pane(_)) => {
            panes.push(current);
        }
        None => {}
    }
}

#[allow(unused_variables)]
pub trait View<T> {
    fn title(&self, shared: &T) -> WidgetText;
    fn closable(&self, shared: &T) -> bool {
        false
    }
    fn check_close(&mut self, shared: &mut T) -> bool {
        true
    }

    fn logic(&mut self, shared: &mut T, force_close: impl FnOnce(), ctx: &Context);
    fn modals(&mut self, shared: &mut T, ctx: &Context) -> bool {
        false
    }
    fn ui(&mut self, shared: &mut T, ui: &mut Ui);

    fn save(&mut self, shared: &mut T) {}
    fn close(&mut self, shared: &mut T) -> bool {
        true
    }
}

pub trait Edit {
    fn ui(&mut self, ui: &mut Ui);
}
