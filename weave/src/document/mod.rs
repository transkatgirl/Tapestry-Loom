//! Interactive representations of Weave documents.

use std::collections::{BTreeSet, HashMap, HashSet, hash_map::Entry};

use serde::Serialize;
use ulid::Ulid;

pub mod content;
mod update;

#[cfg(test)]
mod tests;

use content::{Model, Node, NodeContent, NodeContents, WeaveTimeline};

/// A trait for interactive Weave representations.
pub trait WeaveView {
    /// Retrieve an [`Node`] by its [`Ulid`].
    fn get_node(&self, identifier: &Ulid) -> (Option<&Node>, Option<&Model>);
    /// Retrieve all [`Node`] objects which do not have any parents.
    fn get_root_nodes(&self) -> impl Iterator<Item = (&Node, Option<&Model>)>;
    /// Retrieve all active nodes as [`WeaveTimeline`] objects.
    fn get_active_timelines(&self) -> Vec<WeaveTimeline>;
}

/// An owned Weave document.
///
/// A Weave is a collection of [`Node`] objects, along with their corresponding [`Model`] objects. This interactive representation of a Weave ensures that connections between objects are valid and prevents objects from becoming orphaned.
///
/// In addition to keeping the Weave internally consistent, this implementation also allows for fast retrieval of objects and useful groups of objects (such has active nodes and root nodes) from the Weave.
///
/// Note: This document is built on top of [`std::collections`] types, and as a result, does not automatically shrink its capacity. If you would like to manage the Weave's capacity manually, see the [`Weave::reserve`], [`Weave::shrink_to_fit`], and [`Weave::add_model`] functions.
#[derive(Default, Debug, PartialEq)]
pub struct Weave {
    nodes: HashMap<Ulid, Node>,
    models: HashMap<Ulid, Model>,

    /// Metadata associated with the document.
    pub metadata: HashMap<String, String>,

    root_nodes: BTreeSet<Ulid>,
    model_nodes: HashMap<Ulid, HashSet<Ulid>>,
    multiparent_nodes: HashSet<Ulid>,
    nonconcatable_nodes: HashSet<Ulid>,
}

impl Weave {
    /// Add a [`Node`] (along with it's corresponding [`Model`]).
    ///
    /// If a model corresponding to the node's model identifier is already present in the Weave, the model will be updated.
    ///
    /// Performs content deduplication if `deduplicate` is true.
    ///
    /// Care must be taken to prevent loops between nodes, as loop checking is not performed by this function. If a loop between nodes is added to the [`Weave`], it may cause unintended behavior (such as functions panicking or getting stuck in infinite loops).
    ///
    /// Returns the [`Ulid`] of the input node if the node was successfully added. If the node was deduplicated, the returned Ulid will correspond to a node which was already in the document (the node's active & bookmarked statuses will be updated to match the input). Returns [`None`] if the node could not be added.
    ///
    /// Nodes which have the same identifier as a node already in the [`Weave`] cannot be added. If the Weave contains any nodes with multiple parents, non-concatable nodes cannot be added. If the Weave contains any non-concatable nodes, nodes with multiple parents cannot be added.
    pub fn add_node(
        &mut self,
        mut node: Node,
        model: Option<Model>,
        deduplicate: bool,
    ) -> Option<Ulid> {
        if self.nodes.contains_key(&node.id) {
            return None;
        }
        if (!self.nonconcatable_nodes.is_empty() || !node.content.is_concatable())
            && (!self.multiparent_nodes.is_empty() || node.from.len() > 1 || !node.to.is_empty())
        {
            return None;
        }
        if deduplicate {
            let siblings = node
                .from
                .iter()
                .filter_map(|id| self.nodes.get(id))
                .flat_map(|parent| &parent.to)
                .filter_map(|id| self.nodes.get(id));

            for sibling in siblings {
                if sibling.content == node.content {
                    let identifier = sibling.id;
                    let sibling_active = sibling.active;
                    let sibling_bookmarked = sibling.bookmarked;
                    if sibling_active != node.active {
                        self.update_node_activity(&identifier, node.active, true);
                    }
                    if sibling_bookmarked != node.bookmarked {
                        self.update_node_bookmarked_status(&identifier, node.bookmarked);
                    }
                    return Some(identifier);
                }
            }
        }
        for child in node.to.clone() {
            if let Some(child) = self.nodes.get_mut(&child) {
                child.from.insert(node.id);
            } else {
                node.to.remove(&child);
            }
        }
        for parent in node.from.clone() {
            if node.active {
                self.update_node_activity(&parent, true, true);
            }
            if let Some(parent) = self.nodes.get_mut(&parent) {
                parent.to.insert(node.id);
            } else {
                node.from.remove(&parent);
            }
        }
        if node.from.is_empty() {
            self.root_nodes.insert(node.id);
        }
        if node.from.len() > 1 || !node.to.is_empty() {
            self.multiparent_nodes.insert(node.id);
        }
        if !node.content.is_concatable() {
            self.nonconcatable_nodes.insert(node.id);
        }
        if let Some(node_model) = node.content.model() {
            if let Some(mut model) = model {
                model.id = node_model.id;
                self.models.insert(model.id, model);
            }
            match self.model_nodes.entry(node_model.id) {
                Entry::Occupied(mut entry) => {
                    entry.get_mut().insert(node.id);
                }
                Entry::Vacant(entry) => {
                    entry.insert(HashSet::from([node.id]));
                }
            }
        }
        let identifier = node.id;
        self.nodes.insert(identifier, node);

        Some(identifier)
    }
    /// Add a [`Model`] without a corresponding [`Node`]. If it is already present, the model's contents will be updated.
    ///
    /// In addition to the model, this function also takes a capacity hint. If this hint is present, capacity will be reserved for at least n nodes associated with the model.
    pub fn add_model(&mut self, model: Model, model_nodes: Option<usize>) {
        let identifier = model.id;

        self.models.insert(model.id, model);

        if let Some(capacity) = model_nodes {
            match self.model_nodes.entry(identifier) {
                Entry::Occupied(mut entry) => {
                    let len = entry.get().len();
                    if capacity > len {
                        entry.get_mut().reserve(capacity - len);
                    }
                }
                Entry::Vacant(entry) => {
                    entry.insert(HashSet::with_capacity(capacity));
                }
            }
        } else {
            match self.model_nodes.entry(identifier) {
                Entry::Occupied(_) => {}
                Entry::Vacant(entry) => {
                    entry.insert(HashSet::new());
                }
            }
        }
    }
    /// Recursively update the active status of a [`Node`] by it's [`Ulid`].
    ///
    /// If the node already has the desired active status, no update propagation is performed.
    ///
    /// This function can propagate the active status in two different ways:
    /// ```ignore
    /// if in_place {
    ///     if active {
    ///         /* Recursively activates the node and its first parent node (sorted by Ulid)
    ///            if it does not any have active parent nodes.
    ///            If the parent node is active and has other active siblings besides the
    ///            selected node, they are deactivated using in-place deactivation. */
    ///     } else {
    ///         /* Deactivates the node, and recursively updates the active status of the child
    ///            nodes based on if they have other active parents. */
    ///     }
    /// } else {
    ///     if active {
    ///         /* Recursively activates the node and its first parent node (sorted by Ulid)
    ///            if it does not any have active parent nodes.
    ///            If the node has at least one active parent node, no further propagation is
    ///            performed (parent nodes can have multiple active siblings). */
    ///     } else {
    ///         /* Recursively deactivates the node along with all of its parent nodes.
    ///            Child nodes are recursively updated based on if they have other active parents. */
    ///     }
    /// }
    /// ```
    pub fn update_node_activity(&mut self, identifier: &Ulid, active: bool, in_place: bool) {
        if let Some(node) = self.nodes.get(identifier) {
            if node.active == active {
                return;
            }

            let mut is_parent_active = false;

            for parent in node.from.iter().filter_map(|id| self.nodes.get(id)) {
                if parent.active {
                    is_parent_active = true;
                    break;
                }
            }

            if is_parent_active != active {
                if active {
                    let mut parents: Vec<Ulid> = node.from.iter().copied().collect();
                    parents.sort();
                    if let Some(parent) = parents.first() {
                        self.update_node_activity(parent, true, in_place);
                    }
                } else if !in_place {
                    for parent in node.from.clone() {
                        self.update_node_activity(&parent, false, false);
                    }
                }
            } else if in_place && active {
                let siblings: Vec<Ulid> = node
                    .from
                    .iter()
                    .filter_map(|id| self.nodes.get(id))
                    .flat_map(|parent| parent.to.clone())
                    .collect();

                for sibling in siblings {
                    self.update_node_activity(&sibling, false, true);
                }
            }
        }
        if let Some(node) = self.nodes.get_mut(identifier) {
            node.active = active;
            if !active {
                for child in node.to.clone() {
                    self.update_removed_child_activity(&child);
                }
            }
        }
    }
    /// Update the bookmarked status of a [`Node`] by it's [`Ulid`].
    pub fn update_node_bookmarked_status(&mut self, identifier: &Ulid, bookmarked: bool) {
        if let Some(node) = self.nodes.get_mut(identifier) {
            node.bookmarked = bookmarked;
        }
    }
    fn update_removed_child_activity(&mut self, identifier: &Ulid) {
        if let Some(node) = self.nodes.get(identifier) {
            if !node.active {
                return;
            }

            for parent in node.from.iter().filter_map(|id| self.nodes.get(id)) {
                if parent.active {
                    return;
                }
            }
        }
        if let Some(node) = self.nodes.get_mut(identifier) {
            node.active = false;
            for child in node.to.clone() {
                self.update_removed_child_activity(&child);
            }
        }
    }
    /// Moves a [`Node`] to a new position on the tree (without performing deduplication).
    ///
    /// Care must be taken to prevent loops between nodes, as loop checking is not performed by this function. If a loop between nodes is added to the [`Weave`], it may cause unintended behavior (such as functions panicking or getting stuck in infinite loops).
    ///
    /// The modified node retains all of it's other attributes, including its identifier. Returns if the node was moved successfully.
    pub fn move_node(&mut self, identifier: &Ulid, mut parents: HashSet<Ulid>) -> bool {
        if !self.nonconcatable_nodes.is_empty() && parents.len() > 1 {
            return false;
        }

        let mut old_parents = HashSet::new();
        #[allow(unused_assignments)]
        let mut active_status = false;

        if let Some(node) = self.nodes.get(identifier) {
            old_parents.clone_from(&node.from);
            active_status = node.active;

            if node.id != *identifier {
                return false;
            }
            for child in &node.to {
                if parents.contains(child) {
                    return false;
                }
            }
        } else {
            return false;
        }

        for parent in parents.clone() {
            if active_status {
                self.update_node_activity(&parent, true, true);
            }
            if let Some(parent) = self.nodes.get_mut(&parent) {
                parent.to.insert(*identifier);
            } else {
                parents.remove(&parent);
            }
        }
        if parents.is_empty() {
            self.root_nodes.insert(*identifier);
        } else {
            self.root_nodes.remove(identifier);
        }
        if parents.len() > 1 {
            self.multiparent_nodes.insert(*identifier);
        } else {
            self.multiparent_nodes.remove(identifier);
        }

        if let Some(node) = self.nodes.get_mut(identifier) {
            node.from.clone_from(&parents);
        }

        for old_parent in old_parents {
            if !parents.contains(&old_parent) {
                if let Some(parent) = self.nodes.get_mut(&old_parent) {
                    parent.to.remove(identifier);
                }
            }
        }

        true
    }
    /// Split one [`Node`] into two nodes (without performing deduplication).
    ///
    /// This uses [`NodeContent::split`] to split the [`NodeContent`] object, then updates the Weave as necessary to split the existing node into two nodes. The identifiers of the split node (from left to right) are returned, with no guarantees regarding if they are new identifiers or reused ones (along with no guarantee that the input identifier is still valid). Returns [`None`] if the node could not be split.
    ///
    /// If the node being split is bookmarked, only the left side of the split will be bookmarked. Otherwise, the split nodes retains all other properties of the original node.
    pub fn split_node(&mut self, identifier: &Ulid, index: usize) -> Option<(Ulid, Ulid)> {
        let original = self.nodes.get(identifier)?;
        let (left_content, right_content) = original.content.clone().split(index)?;

        let node = Node {
            id: Ulid::from_datetime(identifier.datetime()),
            from: original.from.clone(),
            to: HashSet::from([*identifier]),
            active: original.active,
            bookmarked: original.bookmarked,
            content: left_content,
        };

        let left_identifier = self.add_node(node, None, false)?;

        let right = self.nodes.get_mut(identifier)?;
        right.content = right_content;
        right.bookmarked = false;
        right.from = HashSet::from([left_identifier]);

        Some((left_identifier, right.id))
    }
    /// Merge two [`Node`]s into one (without performing deduplication).
    ///
    /// The `right` node must be a child of the `left` node for merging to succeed. Children of the `left` node will be removed from the Weave if they have no other parents.
    ///
    /// This uses [`NodeContent::merge`] to merge both [`NodeContent`] objects, then updates the Weave as necessary to merge the two nodes into one node. The identifier of the merged node is returned, with no guarantee regarding if the identifier is new or reused (along with no guarantees that the input identifiers are still valid). Returns [`None`] if the nodes could not be merged.
    ///
    /// The merged node inherits the active status of the `right` node. Otherwise, the merged node retains all other properties of the original nodes.
    pub fn merge_nodes(&mut self, left: &Ulid, right: &Ulid) -> Option<Ulid> {
        let left = self.nodes.get(left)?;
        let right = self.nodes.get(right)?;
        if !(left.to.contains(&right.id) && right.from.contains(&left.id)) {
            return None;
        }

        let content = NodeContent::merge(left.content.clone(), right.content.clone())?;

        let left_identifier = left.id;
        let right_identifier = right.id;

        let from = left.from.clone();
        let bookmarked = left.bookmarked;

        let node = self.nodes.get_mut(&right_identifier)?;
        node.content = content;
        if !node.bookmarked {
            node.bookmarked = bookmarked;
        }
        node.from.clone_from(&from);
        for parent in from {
            if let Some(parent) = self.nodes.get_mut(&parent) {
                parent.to.insert(right_identifier);
            }
        }

        self.remove_node(&left_identifier);

        Some(left_identifier)
    }
    /// Remove a [`Node`] by it's [`Ulid`], returning it's value if it was present.
    ///
    /// All child nodes orphaned by removing the node will be removed. Non-orphaned child nodes will have their active status updated based on if they have other active parents.
    ///
    /// All [`Model`] objects orphaned by removing the node will be removed.
    pub fn remove_node(&mut self, identifier: &Ulid) -> Option<Node> {
        if let Some(node) = self.nodes.remove(identifier) {
            self.root_nodes.remove(&node.id);
            self.multiparent_nodes.remove(&node.id);
            self.nonconcatable_nodes.remove(&node.id);
            for parent in &node.from {
                if let Some(parent) = self.nodes.get_mut(parent) {
                    parent.to.remove(&node.id);
                }
            }
            for child in &node.to {
                if let Some(child) = self.nodes.get_mut(child) {
                    child.from.remove(&node.id);
                    let identifier = child.id;
                    if child.from.is_empty() {
                        self.remove_node(&identifier);
                    } else if node.active {
                        self.update_removed_child_activity(&identifier);
                    }
                }
            }
            if let Some(node_model) = node.content.model() {
                if let Some(model_nodes) = self.model_nodes.get_mut(&node_model.id) {
                    model_nodes.remove(&node.id);
                    if model_nodes.is_empty() {
                        self.models.remove(&node_model.id);
                        self.model_nodes.remove(&node_model.id);
                    }
                }
            }
            Some(node)
        } else {
            None
        }
    }
    /// Reserves capacity in the Weave for (at least) the given number of additional elements.
    ///
    /// This will usually reserve more capacity than strictly necessary, as some data structures within the Weave only contain information regarding certain subsets of objects.
    ///
    /// This will only reserve capacity in private fields of the Weave. Public fields must have their capacity adjusted manually.
    pub fn reserve(&mut self, nodes: usize, models: usize) {
        self.nodes.reserve(nodes);
        self.models.reserve(models);
        self.model_nodes.reserve(models);
        for model_nodes in self.model_nodes.values_mut() {
            model_nodes.reserve(nodes);
        }
        self.multiparent_nodes.reserve(nodes);
        self.nonconcatable_nodes.reserve(nodes);
    }
    /// Shrinks the allocated capacity of the Weave as much as possible.
    ///
    /// This will only shrink private fields of the Weave. Public fields must be shrunk manually.
    pub fn shrink_to_fit(&mut self) {
        self.nodes.shrink_to_fit();
        self.models.shrink_to_fit();
        self.model_nodes.shrink_to_fit();
        for model_nodes in self.model_nodes.values_mut() {
            model_nodes.shrink_to_fit();
        }
        self.multiparent_nodes.shrink_to_fit();
        self.nonconcatable_nodes.shrink_to_fit();
    }
    fn build_timelines<'a>(&'a self, timelines: &mut Vec<Vec<&'a Node>>) {
        let mut new_timelines = Vec::new();
        let mut modified = false;

        for timeline in timelines.iter_mut() {
            if let Some(node) = timeline.last() {
                let mut added_node = false;

                for child in node
                    .to
                    .iter()
                    .filter_map(|id| self.nodes.get(id))
                    .filter(|node| node.active)
                {
                    if added_node {
                        let mut new_timeline = timeline.clone();
                        new_timeline.pop();
                        new_timeline.push(child);
                        new_timelines.push(new_timeline);
                    } else {
                        timeline.push(child);
                        added_node = true;
                    }

                    modified = true;
                }
            }
        }
        for timeline in new_timelines {
            timelines.push(timeline);
        }
        if modified {
            self.build_timelines(timelines);
        }
    }
    /// Returns `true` if the Weave contains any nodes with multiple parents. If this is the case, non-concatable nodes cannot be added.
    // Trivial; shouldn't require unit tests
    pub fn is_multiparent_mode(&self) -> bool {
        !self.multiparent_nodes.is_empty()
    }
    /// Returns `true` if the Weave contains any non-concatable nodes. If this is the case, nodes with multiple parents cannot be added.
    // Trivial; shouldn't require unit tests
    pub fn is_nonconcatable_mode(&self) -> bool {
        !self.nonconcatable_nodes.is_empty()
    }
}

impl WeaveView for Weave {
    fn get_node(&self, identifier: &Ulid) -> (Option<&Node>, Option<&Model>) {
        let node = self.nodes.get(identifier);
        let model = node
            .and_then(|node| node.content.model())
            .and_then(|node_model| self.models.get(&node_model.id));

        (node, model)
    }
    fn get_root_nodes(&self) -> impl Iterator<Item = (&Node, Option<&Model>)> {
        self.root_nodes
            .iter()
            .filter_map(|identifier| self.nodes.get(identifier))
            .map(|node| {
                (
                    node,
                    node.content
                        .model()
                        .and_then(|node_model| self.models.get(&node_model.id)),
                )
            })
    }
    fn get_active_timelines(&self) -> Vec<WeaveTimeline> {
        let mut timelines: Vec<Vec<&Node>> = self
            .root_nodes
            .iter()
            .filter_map(|identifier| self.nodes.get(identifier))
            .filter(|node| node.active)
            .map(|node| Vec::from([node]))
            .collect();
        self.build_timelines(&mut timelines);

        let mut hydrated_timelines: Vec<WeaveTimeline<'_>> = timelines
            .iter()
            .map(|timeline| WeaveTimeline {
                timeline: timeline
                    .iter()
                    .map(|node| {
                        (
                            *node,
                            node.content
                                .model()
                                .and_then(|node_model| self.models.get(&node_model.id)),
                        )
                    })
                    .collect(),
            })
            .collect();

        hydrated_timelines.sort_by_key(|timeline| timeline.timeline.len());

        hydrated_timelines
    }
}

/// A immutable reference to a [`Weave`] object.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct WeaveSnapshot<'w> {
    /// A map of [`Node`] objects in the Weave retrievable by their [`Ulid`].
    pub nodes: &'w HashMap<Ulid, Node>,
    /// A map of [`Model`] objects in the Weave retrievable by their [`Ulid`].
    pub models: &'w HashMap<Ulid, Model>,
    /// An ordered set of [`Ulid`] objects which correspond to [`Node`] objects with no parents.
    pub root_nodes: &'w BTreeSet<Ulid>,
    /// If the [`Weave`] contains any nodes with multiple parents. If this is the case, non-concatable nodes cannot be added.
    pub multiparent_mode: bool,
    /// If the [`Weave`] contains any non-concatable nodes. If this is the case, nodes with multiple parents cannot be added.
    pub nonconcatable_mode: bool,
}

// Copied from Weave's implementation of build_timelines(); shouldn't require additional unit tests
impl WeaveSnapshot<'_> {
    fn build_timelines<'a>(&'a self, timelines: &mut Vec<Vec<&'a Node>>) {
        let mut new_timelines = Vec::new();
        let mut modified = false;

        for timeline in timelines.iter_mut() {
            if let Some(node) = timeline.last() {
                let mut added_node = false;

                for child in node
                    .to
                    .iter()
                    .filter_map(|id| self.nodes.get(id))
                    .filter(|node| node.active)
                {
                    if added_node {
                        let mut new_timeline = timeline.clone();
                        new_timeline.pop();
                        new_timeline.push(child);
                        new_timelines.push(new_timeline);
                    } else {
                        timeline.push(child);
                        added_node = true;
                    }

                    modified = true;
                }
            }
        }
        for timeline in new_timelines {
            timelines.push(timeline);
        }
        if modified {
            self.build_timelines(timelines);
        }
    }
}

// Copied from Weave's implementation of WeaveView; shouldn't require additional unit tests
impl WeaveView for WeaveSnapshot<'_> {
    fn get_node(&self, identifier: &Ulid) -> (Option<&Node>, Option<&Model>) {
        let node = self.nodes.get(identifier);
        let model = node
            .and_then(|node| node.content.model())
            .and_then(|node_model| self.models.get(&node_model.id));

        (node, model)
    }
    fn get_root_nodes(&self) -> impl Iterator<Item = (&Node, Option<&Model>)> {
        self.root_nodes
            .iter()
            .filter_map(|identifier| self.nodes.get(identifier))
            .map(|node| {
                (
                    node,
                    node.content
                        .model()
                        .and_then(|node_model| self.models.get(&node_model.id)),
                )
            })
    }
    fn get_active_timelines(&self) -> Vec<WeaveTimeline> {
        let mut timelines: Vec<Vec<&Node>> = self
            .root_nodes
            .iter()
            .filter_map(|identifier| self.nodes.get(identifier))
            .filter(|node| node.active)
            .map(|node| Vec::from([node]))
            .collect();
        self.build_timelines(&mut timelines);

        let mut hydrated_timelines: Vec<WeaveTimeline<'_>> = timelines
            .iter()
            .map(|timeline| WeaveTimeline {
                timeline: timeline
                    .iter()
                    .map(|node| {
                        (
                            *node,
                            node.content
                                .model()
                                .and_then(|node_model| self.models.get(&node_model.id)),
                        )
                    })
                    .collect(),
            })
            .collect();

        hydrated_timelines.sort_by_key(|timeline| timeline.timeline.len());

        hydrated_timelines
    }
}

impl<'w> From<&'w Weave> for WeaveSnapshot<'w> {
    // Trivial; shouldn't require unit tests
    fn from(input: &'w Weave) -> WeaveSnapshot<'w> {
        Self {
            nodes: &input.nodes,
            models: &input.models,
            root_nodes: &input.root_nodes,
            multiparent_mode: !input.multiparent_nodes.is_empty(),
            nonconcatable_mode: !input.nonconcatable_nodes.is_empty(),
        }
    }
}

pub(super) struct OwnedWeaveSnapshot {
    pub(super) nodes: HashMap<Ulid, Node>,
    pub(super) models: HashMap<Ulid, Model>,
    pub(super) root_nodes: BTreeSet<Ulid>,
    pub(super) metadata: HashMap<String, String>,
}

impl From<Weave> for OwnedWeaveSnapshot {
    // Trivial; shouldn't require unit tests
    fn from(input: Weave) -> Self {
        Self {
            nodes: input.nodes,
            models: input.models,
            root_nodes: input.root_nodes,
            metadata: input.metadata,
        }
    }
}
