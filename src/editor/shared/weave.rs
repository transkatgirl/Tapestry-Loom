use std::{
    cmp::Ordering,
    hash::BuildHasherDefault,
    time::{Duration, SystemTime},
};

use chrono::Local;
use tapestry_weave::{
    hashers::UlidHasher,
    ulid::Ulid,
    universal_weave::{
        Weave,
        indexmap::{IndexMap, IndexSet},
        rkyv::rancor,
    },
    v0::{MetadataMap, TapestryNode, TapestryWeave},
};

pub struct WeaveWrapper {
    weave: TapestryWeave,
    scratchpad: Vec<u128>,
    changed: bool,
    layout_changed: bool,
}

impl Default for WeaveWrapper {
    fn default() -> Self {
        TapestryWeave::with_capacity(
            16384,
            IndexMap::from_iter([
                ("created".to_string(), Local::now().to_rfc3339()),
                ("notes".to_string(), String::with_capacity(16384)),
            ]),
        )
        .into()
    }
}

impl From<TapestryWeave> for WeaveWrapper {
    fn from(value: TapestryWeave) -> Self {
        Self {
            scratchpad: Vec::with_capacity(value.capacity()),
            weave: value,
            changed: false, // Does not react to metadata changes
            layout_changed: false,
        }
    }
}

impl WeaveWrapper {
    pub fn to_versioned_bytes(&self) -> Result<Vec<u8>, rancor::Error> {
        self.weave.to_versioned_bytes()
    }
    pub fn metadata(&self) -> &MetadataMap {
        &self.weave.weave.metadata
    }
    pub fn metadata_mut(&mut self) -> &mut MetadataMap {
        &mut self.weave.weave.metadata
    }

    pub fn len(&self) -> usize {
        self.weave.len()
    }
    pub fn is_empty(&self) -> bool {
        self.weave.is_empty()
    }
    pub fn is_empty_including_metadata(&self) -> bool {
        self.weave.is_empty()
            && self
                .weave
                .weave
                .metadata
                .get("notes")
                .map(|n| n.is_empty())
                .unwrap_or(true)
    }
    pub fn get_bookmarks(&self) -> impl ExactSizeIterator<Item = Ulid> {
        self.weave.get_bookmarks()
    }
    pub fn contains(&self, id: &Ulid) -> bool {
        self.weave.contains(id)
    }
    pub fn get_node(&self, id: &Ulid) -> Option<&TapestryNode> {
        self.weave.get_node(id)
    }
    pub fn get_node_u128(&self, id: &u128) -> Option<&TapestryNode> {
        self.weave.weave.get_node(id)
    }

    pub fn is_mergeable_with_parent(&self, id: &Ulid) -> bool {
        self.weave.is_mergeable_with_parent(id)
    }
    /*pub fn get_thread_from(&mut self, id: &Ulid) -> impl DoubleEndedIterator<Item = Ulid> {
        self.weave
            .weave
            .get_thread_from(&id.0)
            .iter()
            .copied()
            .map(Ulid)
    }*/
    pub fn get_thread_from_u128(&mut self, id: &u128) -> impl DoubleEndedIterator<Item = u128> {
        self.weave.weave.get_thread_from(id, &mut self.scratchpad);

        self.scratchpad.drain(..)
    }
    /*pub fn get_active_thread(&mut self) -> impl DoubleEndedIterator<Item = Ulid> {
        self.weave
            .weave
            .get_active_thread()
            .iter()
            .copied()
            .map(Ulid)
    }*/
    /*pub fn dump_identifiers_u128(&self) -> impl ExactSizeIterator<Item = u128> {
        self.weave.weave.get_all_nodes_unordered()
    }*/
    pub fn dump_identifiers_ordered_u128(&self) -> Vec<u128> {
        let mut identifiers = Vec::with_capacity(self.weave.len());

        for root in self.weave.weave.roots() {
            self.add_node_identifiers(root, &mut identifiers);
        }

        identifiers
    }
    pub fn dump_identifiers_ordered_u128_rev(&self) -> Vec<u128> {
        let mut identifiers = Vec::with_capacity(self.weave.len());

        for root in self.weave.weave.roots() {
            self.add_node_identifiers_rev(root, &mut identifiers);
        }

        identifiers
    }
    fn add_node_identifiers(&self, id: &u128, identifiers: &mut Vec<u128>) {
        if let Some(node) = self.weave.weave.get_node(id) {
            identifiers.push(node.id);
            for child in &node.to {
                self.add_node_identifiers(child, identifiers);
            }
        }
    }
    fn add_node_identifiers_rev(&self, id: &u128, identifiers: &mut Vec<u128>) {
        if let Some(node) = self.weave.weave.get_node(id) {
            identifiers.push(node.id);
            for child in node.to.iter().rev() {
                self.add_node_identifiers_rev(child, identifiers);
            }
        }
    }
    pub fn get_active_thread_u128(&mut self) -> impl DoubleEndedIterator<Item = u128> {
        self.weave.weave.get_active_thread(&mut self.scratchpad);

        self.scratchpad.drain(..)
    }
    pub fn get_active_thread(&mut self) -> impl DoubleEndedIterator<Item = Ulid> {
        self.weave.weave.get_active_thread(&mut self.scratchpad);

        self.scratchpad.drain(..).map(Ulid)
    }
    /*pub fn get_active_thread_nodes(&mut self) -> impl Iterator<Item = &DependentNode<NodeContent>> {
        self.weave.get_active_thread()
    }*/
    pub fn get_active_thread_first(&mut self) -> Option<Ulid> {
        self.weave.weave.get_active_thread(&mut self.scratchpad);

        self.scratchpad.drain(..).next().map(Ulid)
    }
    pub fn get_active_thread_len(&mut self) -> usize {
        self.weave.weave.get_active_thread(&mut self.scratchpad);

        self.scratchpad.len()
    }
    pub fn get_roots(&self) -> impl Iterator<Item = Ulid> {
        self.weave.get_roots()
    }
    pub fn get_roots_u128(&self) -> impl Iterator<Item = u128> {
        self.weave.weave.roots().iter().copied()
    }
    pub fn get_roots_u128_direct(&self) -> &IndexSet<u128, BuildHasherDefault<UlidHasher>> {
        self.weave.weave.roots()
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

    pub fn add_node(&mut self, node: TapestryNode) -> bool {
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
        self.weave.weave.set_node_active_status(id, value, false)
    }
    pub fn merge_with_parent(&mut self, id: &Ulid) -> bool {
        self.changed = true;
        self.layout_changed = true;
        self.weave.merge_with_parent(id)
    }
    /*pub fn merge_with_parent_u128(&mut self, id: &u128) -> bool {
        self.changed = true;
        self.layout_changed = true;
        self.weave.weave.merge_with_parent(id)
    }*/
    pub fn split_node(&mut self, id: &Ulid, at: usize) -> bool {
        self.changed = true;
        self.layout_changed = true;
        self.weave
            .split_node(id, at, |timestamp| {
                Ulid::from_datetime(SystemTime::UNIX_EPOCH + Duration::from_millis(timestamp))
            })
            .is_some()
    }
    /*pub fn split_out_token(
        &mut self,
        id: &Ulid,
        index: usize,
    ) -> Option<(Ulid, Ulid, Option<Ulid>)> {
        if let Some(node) = self.weave.get_node(id) {
            if let InnerNodeContent::Tokens(tokens) = &node.contents.content
                && tokens.len() > index
            {
                let split_index: usize = tokens.iter().take(index).map(|token| token.0.len()).sum();

                let second_split_index = if tokens.len() > index + 1 {
                    Some(
                        tokens
                            .iter()
                            .take(index)
                            .map(|token| token.0.len())
                            .sum::<usize>()
                            - split_index,
                    )
                } else {
                    None
                };

                self.changed = true;
                self.layout_changed = true;

                let middle_id = self
                    .weave
                    .split_node(id, split_index, |timestamp| {
                        Ulid::from_datetime(
                            SystemTime::UNIX_EPOCH + Duration::from_millis(timestamp),
                        )
                    })
                    .unwrap();

                if let Some(second_split_index) = second_split_index {
                    let tail_id = self
                        .weave
                        .split_node(&middle_id, second_split_index, |timestamp| {
                            Ulid::from_datetime(
                                SystemTime::UNIX_EPOCH + Duration::from_millis(timestamp),
                            )
                        })
                        .unwrap();

                    Some((*id, middle_id, Some(tail_id)))
                } else {
                    Some((*id, middle_id, None))
                }
            } else {
                None
            }
        } else {
            None
        }
    }*/
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
    pub fn set_active_content(&mut self, value: &[u8], metadata: MetadataMap) -> bool {
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
    pub fn sort_node_children_u128_by(
        &mut self,
        id: &u128,
        compare: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering,
    ) -> bool {
        self.changed = true;
        self.layout_changed = true;
        self.weave.weave.sort_node_children_by(id, compare)
    }
    pub fn sort_roots_by(&mut self, compare: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering) {
        self.changed = true;
        self.layout_changed = true;
        self.weave.weave.sort_roots_by(compare)
    }
}
