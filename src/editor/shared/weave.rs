use std::time::{Duration, SystemTime};

use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{
        Weave,
        dependent::DependentNode,
        indexmap::{IndexMap, IndexSet},
    },
    v0::{InnerNodeContent, NodeContent, TapestryWeave},
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
    pub fn get_bookmarks(&self) -> impl Iterator<Item = Ulid> {
        self.weave.get_bookmarks()
    }
    pub fn get_node(&self, id: &Ulid) -> Option<&DependentNode<NodeContent>> {
        self.weave.get_node(id)
    }
    pub fn get_node_u128(&self, id: &u128) -> Option<&DependentNode<NodeContent>> {
        self.weave.weave.get_node(id)
    }
    pub fn node_siblings_or_roots(&self, node: &DependentNode<NodeContent>) -> Vec<Ulid> {
        if let Some(parent) = node.from.and_then(|id| self.weave.weave.get_node(&id)) {
            parent
                .to
                .iter()
                .copied()
                .filter(|id| *id != node.id)
                .map(Ulid)
                .collect()
        } else {
            self.weave
                .weave
                .get_roots()
                .iter()
                .copied()
                .filter(|id| *id != node.id)
                .map(Ulid)
                .collect()
        }
    }
    pub fn is_mergeable_with_parent(&self, id: &Ulid) -> bool {
        self.weave.is_mergeable_with_parent(id)
    }
    pub fn get_thread_from(&mut self, id: &Ulid) -> impl Iterator<Item = Ulid> {
        self.weave
            .weave
            .get_thread_from(&id.0)
            .iter()
            .copied()
            .map(Ulid)
    }
    pub fn get_active_thread(&mut self) -> impl Iterator<Item = Ulid> {
        self.weave
            .weave
            .get_active_thread()
            .iter()
            .copied()
            .map(Ulid)
    }
    pub fn get_active_thread_nodes(&mut self) -> impl Iterator<Item = &DependentNode<NodeContent>> {
        self.weave.get_active_thread()
    }
    pub fn get_roots(&self) -> impl Iterator<Item = Ulid> {
        self.weave.get_roots()
    }
    pub fn has_changed(&mut self) -> bool {
        let value = self.changed;
        self.changed = false;
        value
    }
    pub fn has_layout_changed(&mut self) -> bool {
        let value = self.layout_changed;
        self.layout_changed = false;
        value
    }

    pub fn add_node(&mut self, node: DependentNode<NodeContent>) -> bool {
        self.changed = true;
        self.layout_changed = true;
        self.weave.add_node(node)
    }
    pub fn add_blank_node(&mut self, parent: Option<u128>, active: bool) -> Option<Ulid> {
        let identifier = Ulid::new();
        if self.add_node(DependentNode {
            id: identifier.0,
            from: parent,
            to: IndexSet::default(),
            active,
            bookmarked: false,
            contents: NodeContent {
                content: InnerNodeContent::Snippet(vec![]),
                metadata: IndexMap::new(),
                model: None,
            },
        }) {
            Some(identifier)
        } else {
            None
        }
    }
    pub fn set_node_bookmarked_status_u128(&mut self, id: &u128, value: bool) -> bool {
        self.changed = true;
        self.weave.weave.set_node_bookmarked_status(id, value)
    }
    pub fn set_node_active_status_u128(&mut self, id: &u128, value: bool) -> bool {
        self.changed = true;
        self.weave.weave.set_node_active_status(id, value)
    }
    pub fn merge_with_parent(&mut self, id: &Ulid) -> bool {
        self.changed = true;
        self.layout_changed = true;
        self.weave.merge_with_parent(id)
    }
    pub fn remove_node_u128(&mut self, id: &u128, value: bool) -> bool {
        self.changed = true;
        self.layout_changed = true;
        self.weave.weave.remove_node(id).is_some()
    }
    pub fn set_active_content<F>(
        &mut self,
        value: &[u8],
        metadata: IndexMap<String, String>,
    ) -> bool {
        self.changed = true;
        self.layout_changed = true;
        self.weave.set_active_content(value, metadata, |timestamp| {
            if let Some(timestamp) = timestamp {
                Ulid::from_datetime(SystemTime::UNIX_EPOCH + Duration::from_millis(timestamp))
            } else {
                Ulid::new()
            }
        })
    }
}
