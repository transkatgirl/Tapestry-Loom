use std::{
    hash::BuildHasherDefault,
    time::{Duration, SystemTime},
};

use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{
        DiscreteWeave, Weave,
        dependent::DependentNode,
        indexmap::{IndexMap, IndexSet},
        rkyv::{hash::FxHasher64, rancor},
    },
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
    pub fn to_versioned_bytes(&self) -> Result<Vec<u8>, rancor::Error> {
        self.weave.to_versioned_bytes()
    }

    pub fn len(&self) -> usize {
        self.weave.len()
    }
    pub fn get_bookmarks(&self) -> impl Iterator<Item = Ulid> {
        self.weave.get_bookmarks()
    }
    pub fn contains(&self, id: &Ulid) -> bool {
        self.weave.contains(id)
    }
    pub fn get_node(&self, id: &Ulid) -> Option<&DependentNode<NodeContent>> {
        self.weave.get_node(id)
    }
    pub fn get_node_u128(&self, id: &u128) -> Option<&DependentNode<NodeContent>> {
        self.weave.weave.get_node(id)
    }

    pub fn is_mergeable_with_parent(&self, id: &Ulid) -> bool {
        self.weave.is_mergeable_with_parent(id)
    }
    pub fn get_thread_from(&mut self, id: &Ulid) -> impl DoubleEndedIterator<Item = Ulid> {
        self.weave
            .weave
            .get_thread_from(&id.0)
            .iter()
            .copied()
            .map(Ulid)
    }
    pub fn get_thread_from_u128(&mut self, id: &u128) -> impl DoubleEndedIterator<Item = u128> {
        self.weave.weave.get_thread_from(id).iter().copied()
    }
    pub fn get_active_thread(&mut self) -> impl DoubleEndedIterator<Item = Ulid> {
        self.weave
            .weave
            .get_active_thread()
            .iter()
            .copied()
            .map(Ulid)
    }
    pub fn get_active_thread_u128(&mut self) -> impl DoubleEndedIterator<Item = u128> {
        self.weave.weave.get_active_thread().iter().copied()
    }
    pub fn get_active_thread_nodes(&mut self) -> impl Iterator<Item = &DependentNode<NodeContent>> {
        self.weave.get_active_thread()
    }
    pub fn get_active_thread_first(&mut self) -> Option<Ulid> {
        self.weave
            .weave
            .get_active_thread()
            .front()
            .copied()
            .map(Ulid)
    }
    pub fn get_active_thread_len(&mut self) -> usize {
        self.weave.weave.get_active_thread().len()
    }
    pub fn get_roots(&self) -> impl Iterator<Item = Ulid> {
        self.weave.get_roots()
    }
    pub fn get_roots_u128(&self) -> impl Iterator<Item = u128> {
        self.weave.weave.get_roots().iter().copied()
    }
    pub fn get_roots_u128_direct(&self) -> &IndexSet<u128, BuildHasherDefault<FxHasher64>> {
        self.weave.weave.get_roots()
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
    pub fn set_node_bookmarked_status_u128(&mut self, id: &u128, value: bool) -> bool {
        self.changed = true;
        self.weave.weave.set_node_bookmarked_status(id, value)
    }
    pub fn set_node_active_status(&mut self, id: &Ulid, value: bool) -> bool {
        self.changed = true;
        self.weave.set_node_active_status(id, value)
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
    pub fn merge_with_parent_u128(&mut self, id: &u128) -> bool {
        self.changed = true;
        self.layout_changed = true;
        self.weave.weave.merge_with_parent(id)
    }
    pub fn split_node(&mut self, id: &Ulid, at: usize) -> bool {
        self.changed = true;
        self.layout_changed = true;
        self.weave
            .split_node(id, at, |timestamp| {
                Ulid::from_datetime(SystemTime::UNIX_EPOCH + Duration::from_millis(timestamp))
            })
            .is_some()
    }
    pub fn remove_node(&mut self, id: &Ulid) -> bool {
        self.changed = true;
        self.layout_changed = true;
        self.weave.remove_node(id).is_some()
    }
    pub fn remove_node_u128(&mut self, id: &u128) -> bool {
        self.changed = true;
        self.layout_changed = true;
        self.weave.weave.remove_node(id).is_some()
    }
    pub fn set_active_content(&mut self, value: &[u8], metadata: IndexMap<String, String>) -> bool {
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
