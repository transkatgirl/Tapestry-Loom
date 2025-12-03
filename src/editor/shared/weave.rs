use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{Weave, dependent::DependentNode},
    v0::{NodeContent, TapestryWeave},
};

pub struct WeaveWrapper {
    weave: TapestryWeave,
    changed: bool,
    layout_changed: bool,
}

impl From<TapestryWeave> for WeaveWrapper {
    fn from(value: TapestryWeave) -> Self {
        Self {
            weave: value,
            changed: false,
            layout_changed: false,
        }
    }
}

impl WeaveWrapper {
    pub fn children_or_roots_of(&mut self, id: Ulid) -> Vec<Ulid> {
        if let Some(node) = self.weave.get_node(&id) {
            node.to.iter().cloned().map(Ulid).collect()
        } else {
            self.weave
                .weave
                .get_roots()
                .iter()
                .copied()
                .map(Ulid)
                .collect()
        }
    }
    pub fn bookmarks(&self) -> impl Iterator<Item = Ulid> {
        self.weave.get_bookmarks()
    }
    pub fn get_node(&self, id: &Ulid) -> Option<&DependentNode<NodeContent>> {
        self.weave.get_node(id)
    }
    pub fn get_node_u128(&self, id: &u128) -> Option<&DependentNode<NodeContent>> {
        self.weave.weave.get_node(id)
    }
    pub fn set_node_bookmarked_status_u128(&mut self, id: &u128, value: bool) -> bool {
        self.changed = true;
        self.weave.weave.set_node_bookmarked_status(id, value)
    }
    pub fn get_thread_from(&mut self, id: &Ulid) -> impl Iterator<Item = Ulid> {
        self.weave
            .weave
            .get_thread_from(&id.0)
            .iter()
            .copied()
            .map(Ulid)
    }
    pub fn get_roots(&self) -> impl Iterator<Item = Ulid> {
        self.weave.get_roots()
    }
}
