#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::doc_markdown)]

//! An implementation of Tapestry Loom's "weave" document format.

use std::collections::{BTreeSet, HashMap, HashSet, hash_map::Entry};

use ulid::Ulid;

pub mod content;
pub mod format;
pub mod prefix;

use crate::content::{Model, Node, NodeContents, WeaveTimeline};

/* TODO:
- Documentation
- Unit tests
    - Node management & update propagation (propagating changes into node children & parents, root_nodes updating)
    - Model management & update propagation (model addition/removal, model_nodes updating)
    - Node loop checking
    - Node activation
    - Node content deduplication */

pub trait WeaveView {
    fn get_node(&self, identifier: &Ulid) -> (Option<&Node>, Option<&Model>);
    fn get_root_nodes(&self) -> impl Iterator<Item = (&Node, Option<&Model>)>;
    fn get_active_timelines(&self) -> Vec<WeaveTimeline>;
}

#[derive(Default, Debug, PartialEq)]
pub struct Weave {
    nodes: HashMap<Ulid, Node>,
    models: HashMap<Ulid, Model>,

    root_nodes: BTreeSet<Ulid>,
    model_nodes: HashMap<Ulid, HashSet<Ulid>>,
}

impl Weave {
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
                }
                if node.from.len() <= 1 {
                    self.remove_node(child);
                } else if node.active {
                    self.update_removed_child_activity(child);
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

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashSet};

    use ulid::Ulid;

    use crate::{
        Weave,
        content::{Node, NodeContent},
    };

    fn blank_moveable_node<X, Y>(id: Ulid, from: X, to: Y) -> Node
    where
        X: IntoIterator<Item = Ulid>,
        Y: IntoIterator<Item = Ulid>,
    {
        Node {
            id,
            to: HashSet::from_iter(to),
            from: HashSet::from_iter(from),
            active: false,
            bookmarked: false,
            content: NodeContent::Blank,
        }
    }

    #[test]
    fn add_node_propagation() {
        let mut weave = Weave::default();

        let root_node_identifier = Ulid::new();
        let root_node_2_identifier = Ulid::new();
        let child_node_1_identifier = Ulid::new();
        let child_node_2_identifier = Ulid::new();
        let child_node_3_identifier = Ulid::new();
        let child_node_4_identifier = Ulid::new();
        let child_node_5_identifier = Ulid::new();
        let child_node_6_identifier = Ulid::new();
        assert!(
            weave
                .add_node(
                    blank_moveable_node(root_node_identifier, [], []),
                    None,
                    false,
                    false
                )
                .is_some()
        );
        {
            assert!(weave.root_nodes == BTreeSet::from([root_node_identifier]));
            let root_node_1 = weave.nodes.get(&root_node_identifier).unwrap();
            assert!(root_node_1.from.is_empty());
            assert!(root_node_1.to.is_empty());
        }
        assert!(
            weave
                .add_node(
                    blank_moveable_node(root_node_2_identifier, [], []),
                    None,
                    false,
                    false
                )
                .is_some()
        );

        assert!(
            weave
                .add_node(
                    blank_moveable_node(child_node_1_identifier, [root_node_identifier], []),
                    None,
                    false,
                    false
                )
                .is_some()
        );
        assert!(
            weave
                .add_node(
                    blank_moveable_node(child_node_2_identifier, [root_node_identifier], []),
                    None,
                    false,
                    false
                )
                .is_some()
        );
        assert!(
            weave
                .add_node(
                    blank_moveable_node(child_node_3_identifier, [child_node_2_identifier], []),
                    None,
                    false,
                    false
                )
                .is_some()
        );
        assert!(
            weave
                .add_node(
                    blank_moveable_node(child_node_4_identifier, [child_node_3_identifier], []),
                    None,
                    false,
                    false
                )
                .is_some()
        );
        assert!(
            weave
                .add_node(
                    blank_moveable_node(
                        child_node_5_identifier,
                        [child_node_3_identifier, child_node_4_identifier],
                        []
                    ),
                    None,
                    false,
                    false
                )
                .is_some()
        );
        assert!(
            weave
                .add_node(
                    blank_moveable_node(child_node_6_identifier, [child_node_5_identifier], []),
                    None,
                    false,
                    false
                )
                .is_some()
        );
        {
            assert!(
                weave.root_nodes == BTreeSet::from([root_node_identifier, root_node_2_identifier])
            );
            let root_node_1 = weave.nodes.get(&root_node_identifier).unwrap();
            let root_node_2 = weave.nodes.get(&root_node_2_identifier).unwrap();
            let child_node_1 = weave.nodes.get(&child_node_1_identifier).unwrap();
            let child_node_2 = weave.nodes.get(&child_node_2_identifier).unwrap();
            let child_node_3 = weave.nodes.get(&child_node_3_identifier).unwrap();
            let child_node_4 = weave.nodes.get(&child_node_4_identifier).unwrap();
            let child_node_5 = weave.nodes.get(&child_node_5_identifier).unwrap();
            let child_node_6 = weave.nodes.get(&child_node_6_identifier).unwrap();
            assert!(root_node_1.from.is_empty());
            assert!(
                root_node_1.to == HashSet::from([child_node_1_identifier, child_node_2_identifier])
            );
            assert!(root_node_2.from.is_empty());
            assert!(root_node_2.to.is_empty());
            assert!(child_node_1.from == HashSet::from([root_node_identifier]));
            assert!(child_node_1.to.is_empty());

            /*assert!(child_node_1.from == HashSet::from([root_node_identifier]));
            assert!(child_node_1.to.contains(&child_node_2_identifier));
            assert!(child_node_2.from == HashSet::from([child_node_1_identifier]));
            assert!(child_node_2.to == HashSet::from([child_node_3_identifier]));
            assert!(child_node_3.from == HashSet::from([child_node_2_identifier]));
            assert!(child_node_3.to.is_empty());*/

            todo!();
        }
    }

    /*#[test]
    fn remove_node_propagation() {}

    #[test]
    fn add_node_model_propagation() {}

    #[test]
    fn remove_node_model_propagation() {}

    #[test]
    fn check_has_parent_loop() {}

    #[test]
    fn check_has_child_loop() {}

    #[test]
    fn add_node_check_loop() {}

    #[test]
    fn remove_node_check_loop() {}

    #[test]
    fn update_node_activation_propagation() {}

    #[test]
    fn update_node_activation_controlled_propagation() {}

    #[test]
    fn add_node_activation_propagation() {}

    #[test]
    fn remove_node_activation_propagation() {}

    #[test]
    fn add_node_deduplication() {}*/
}
