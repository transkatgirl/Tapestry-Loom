//! Interactive representations of Weave documents.

use std::collections::{BTreeSet, HashMap, HashSet, hash_map::Entry};

use serde::Serialize;
use ulid::Ulid;

// TODO: Fix weave.update_node_activity()

use crate::content::{Model, Node, NodeContents, WeaveTimeline};

/// Functions implemented by all Weave documents.
pub trait WeaveView {
    /// Retrieve an [`Node`] by its [`Ulid`].
    fn get_node(&self, identifier: &Ulid) -> (Option<&Node>, Option<&Model>);
    /// Retrieve all [`Node`] objects which do not have any parents.
    fn get_root_nodes(&self) -> impl Iterator<Item = (&Node, Option<&Model>)>;
    /// Retrieve all active nodes as [`WeaveTimeline`] objects.
    fn get_active_timelines(&self) -> Vec<WeaveTimeline>;
}

/// A mutable Weave representation.
#[derive(Default, Debug, PartialEq)]
pub struct Weave {
    nodes: HashMap<Ulid, Node>,
    models: HashMap<Ulid, Model>,

    root_nodes: BTreeSet<Ulid>,
    model_nodes: HashMap<Ulid, HashSet<Ulid>>,
}

impl Weave {
    /// Add a [`Node`] (along with it's corresponding [`Model`]).
    ///
    /// Performs content deduplication if `deduplicate` is true, and performs loop checking (slow, requires recursively checking of all parents & children) if `skip_loop_check` is true.
    ///
    /// Returns the [`Ulid`] of the input node if the node was successfully added. If the node was deduplicated, the returned Ulid will correspond to a node which was already in the document. Returns [`None`] if the node could not be added due to having a duplicate identifier.
    pub fn add_node(
        &mut self,
        mut node: Node,
        model: Option<Model>,
        skip_loop_check: bool,
        deduplicate: bool,
    ) -> Option<Ulid> {
        if self.nodes.contains_key(&node.id) {
            return None;
        }
        if !skip_loop_check {
            for parent in &node.from {
                if self.has_parent_loop(parent, None) {
                    return None;
                }
            }
            for child in &node.to {
                if self.has_child_loop(child, None) {
                    return None;
                }
            }
        }
        if deduplicate {
            for parent in node.from.iter().filter_map(|id| self.nodes.get(id)) {
                for parent_child in parent.to.iter().filter_map(|id| self.nodes.get(id)) {
                    if parent_child.content == node.content {
                        let identifier = parent_child.id;
                        if parent_child.active != node.active {
                            self.update_node_activity(&identifier, node.active);
                        }
                        return Some(identifier);
                    }
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
                self.update_node_activity(&parent, true);
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
    fn has_parent_loop(&self, identifier: &Ulid, start: Option<&Ulid>) -> bool {
        let start = start.unwrap_or(identifier);
        if let Some(node) = self.nodes.get(identifier) {
            for parent in &node.from {
                if parent == start || self.has_parent_loop(parent, Some(start)) {
                    return true;
                }
            }

            false
        } else {
            false
        }
    }
    fn has_child_loop(&self, identifier: &Ulid, start: Option<&Ulid>) -> bool {
        let start = start.unwrap_or(identifier);
        if let Some(node) = self.nodes.get(identifier) {
            for child in &node.to {
                if child == start || self.has_child_loop(child, Some(start)) {
                    return true;
                }
            }

            false
        } else {
            false
        }
    }
    // ! FIXME
    pub fn update_node_activity(&mut self, identifier: &Ulid, active: bool) {
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
                        self.update_node_activity(parent, true);
                    }
                } else {
                    let children = node.to.clone();
                    let parents = node.from.clone();
                    for child in children {
                        self.update_node_activity(&child, false);
                    }
                    for parent in parents {
                        self.update_node_activity(&parent, false);
                    }
                }
            }
        }
        if let Some(node) = self.nodes.get_mut(identifier) {
            node.active = active;
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
    /// Remove a [`Node`] by it's [`Ulid`].
    ///
    /// All child nodes orphaned by removing the node will be removed. Non-orphaned child nodes will have their active status updated based on if they have other active parents.
    ///
    /// All [`Model`] objects orphaned by removing the node will be removed.
    pub fn remove_node(&mut self, identifier: &Ulid) {
        if let Some(node) = self.nodes.remove(identifier) {
            self.root_nodes.remove(&node.id);
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
        }
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

/// A immutable view of a [`Weave`] object.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct WeaveSnapshot<'w> {
    /// A map of [`Node`] objects in the Weave retrievable by their [`Ulid`].
    pub nodes: &'w HashMap<Ulid, Node>,
    /// A map of [`Model`] objects in the Weave retrievable by their [`Ulid`].
    pub models: &'w HashMap<Ulid, Model>,
    /// An ordered set of [`Ulid`] objects which correspond to [`Node`] objects with no parents.
    pub root_nodes: &'w BTreeSet<Ulid>,
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
    fn from(input: &'w Weave) -> WeaveSnapshot<'w> {
        Self {
            nodes: &input.nodes,
            models: &input.models,
            root_nodes: &input.root_nodes,
        }
    }
}
