use std::collections::{BTreeSet, HashMap, HashSet, hash_map::Entry};

use ulid::Ulid;

mod content;
mod format;

use crate::content::{Model, Node, WeaveTimeline};

/* TODO:
- Node content deduplication
- Update API terminology to borrow more terms from actual tapestry making
- Unit tests
    - Node management & update propagation (propagating changes into node children & parents, root_nodes updating)
    - Model management & update propagation (model addition/removal, model_nodes updating)
    - Node loop checking
    - Node locking
    - Node activation
    - Node content deduplication */

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
    ) -> bool {
        if self.nodes.contains_key(&node.id) {
            return false;
        }
        if !skip_loop_check {
            for parent in &node.from {
                if self.has_parent_loop(parent, None) {
                    return false;
                }
            }
            for child in &node.to {
                if self.has_child_loop(child, None) {
                    return false;
                }
            }
        }
        if node.from.is_empty() {
            self.root_nodes.insert(node.id);
        }
        if node.moveable && !node.content.moveable() {
            node.moveable = false;
        }
        for child in node.to.clone() {
            if let Some(child) = self.nodes.get_mut(&child) {
                if child.moveable {
                    child.from.insert(node.id);
                } else {
                    node.to.remove(&child.id);
                }
            }
        }
        for parent in &node.from {
            if let Some(parent) = self.nodes.get_mut(parent) {
                parent.to.insert(node.id);
            }
            if !node.moveable {
                self.update_node_moveability(parent, false);
            }
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
        self.nodes.insert(node.id, node);

        true
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
    pub fn update_node_moveability(&mut self, identifier: &Ulid, moveable: bool) -> bool {
        // ! FIXME: Need to handle propagation to children
        if let Some(node) = self.nodes.get(identifier) {
            if node.moveable == moveable {
                return true;
            }

            if moveable && !node.content.moveable() {
                return false;
            }

            for parent in node.from.clone() {
                if !self.update_node_moveability(&parent, moveable) {
                    return false;
                }
            }
        }
        if let Some(node) = self.nodes.get_mut(identifier) {
            node.moveable = moveable;
            true
        } else {
            false
        }
    }
    pub fn update_node_activity(
        // ! FIXME: Need to propagate deactivation to children
        &mut self,
        identifier: &Ulid,
        active: bool,
        update_parents: bool,
    ) -> bool {
        if let Some(node) = self.nodes.get(identifier) {
            if node.active == active {
                return true;
            }

            if !node.moveable {
                return false;
            }

            let mut is_parent_active = false;

            for parent in &node.from {
                if let Some(parent) = self.nodes.get(parent) {
                    if parent.active {
                        is_parent_active = true;
                        break;
                    }
                }
            }
            if is_parent_active != active && update_parents {
                if active {
                    let mut parents: Vec<Ulid> = node.from.iter().copied().collect();
                    parents.sort();
                    if let Some(parent) = parents.first() {
                        if !self.update_node_activity(parent, true, true) {
                            return false;
                        }
                    }
                } else {
                    for parent in node.from.clone() {
                        self.update_node_activity(&parent, false, true);
                    }
                }
            }
        }
        if let Some(node) = self.nodes.get_mut(identifier) {
            node.active = active;
            true
        } else {
            false
        }
    }
    pub fn update_node_parents(
        &mut self,
        identifier: &Ulid,
        parents: HashSet<Ulid>,
        skip_loop_check: bool,
    ) -> bool {
        let moveable = self
            .nodes
            .get(identifier)
            .map(|node| node.moveable)
            .unwrap_or(false);
        if !moveable {
            return false;
        }
        if !skip_loop_check {
            for parent in &parents {
                if self.has_parent_loop(parent, None) {
                    return false;
                }
            }
        }
        if let Some(old_parents) = self.nodes.get(identifier).map(|node| node.from.clone()) {
            for parent in &old_parents {
                if let Some(parent) = self.nodes.get_mut(parent) {
                    parent.to.remove(identifier);
                }
            }
            for parent in &parents {
                if let Some(parent) = self.nodes.get_mut(parent) {
                    parent.to.insert(*identifier);
                }
            }
            match parents.is_empty() {
                true => self.root_nodes.insert(*identifier),
                false => self.root_nodes.remove(identifier),
            };
            if let Some(node) = self.nodes.get_mut(identifier) {
                node.from = parents;
            }

            true
        } else {
            false
        }
    }
    pub fn update_node_children(
        &mut self,
        identifier: &Ulid,
        mut children: HashSet<Ulid>,
        skip_loop_check: bool,
    ) -> bool {
        if let Some(old_children) = self.nodes.get(identifier).map(|node| node.to.clone()) {
            for child in &old_children {
                if let Some(child) = self.nodes.get(child) {
                    if !child.moveable {
                        return false;
                    }
                }
            }
            if !skip_loop_check {
                for child in &children {
                    if self.has_child_loop(child, None) {
                        return false;
                    }
                }
            }
            for child in &old_children {
                if let Some(child) = self.nodes.get_mut(child) {
                    child.from.remove(identifier);
                }
            }
            for child in children.clone() {
                if let Some(child) = self.nodes.get_mut(&child) {
                    if child.moveable {
                        child.from.insert(*identifier);
                    } else {
                        children.remove(&child.id);
                    }
                }
            }
            if let Some(node) = self.nodes.get_mut(identifier) {
                node.to = children;
            }

            true
        } else {
            false
        }
    }
    pub fn remove_node(
        // ! FIXME: Need to handle removal of active nodes
        &mut self,
        identifier: &Ulid,
        remove_children: bool,
        unlock_parents: bool,
    ) -> bool {
        if !remove_children {
            if let Some(node) = self.nodes.get(identifier) {
                for child in &node.to {
                    if let Some(child) = self.nodes.get(child) {
                        if !child.moveable {
                            return false;
                        }
                    }
                }
            }
        }
        if let Some(node) = self.nodes.remove(identifier) {
            self.root_nodes.remove(&node.id);
            for parent in &node.from {
                if let Some(parent) = self.nodes.get_mut(parent) {
                    parent.to.remove(&node.id);
                }
                if !node.moveable && unlock_parents {
                    self.update_node_moveability(parent, true);
                }
            }
            for child in &node.to {
                if let Some(child) = self.nodes.get_mut(child) {
                    child.from.remove(&node.id);
                }
                if remove_children {
                    self.remove_node(child, true, false);
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

            true
        } else {
            false
        }
    }
    pub fn get_node(&self, identifier: &Ulid) -> (Option<&Node>, Option<&Model>) {
        let node = self.nodes.get(identifier);
        let model = node
            .and_then(|node| node.content.model())
            .and_then(|node_model| self.models.get(&node_model.id));

        (node, model)
    }
    pub fn get_root_nodes(&self) -> impl Iterator<Item = (&Node, Option<&Model>)> {
        self.root_nodes
            .iter()
            .flat_map(|identifier| self.nodes.get(identifier))
            .map(|node| {
                (
                    node,
                    node.content
                        .model()
                        .and_then(|node_model| self.models.get(&node_model.id)),
                )
            })
    }
    pub fn get_active_timelines(&self) -> Vec<WeaveTimeline> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashSet};

    use ulid::Ulid;

    use crate::{
        Weave,
        content::{Node, NodeContent, TextNode},
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
            moveable: true,
            active: false,
            content: NodeContent::Text(TextNode {
                content: String::default(),
                model: None,
            }),
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
        assert!(weave.add_node(
            blank_moveable_node(root_node_identifier, [], []),
            None,
            false,
        ));
        {
            assert!(weave.root_nodes == BTreeSet::from([root_node_identifier]));
            let root_node_1 = weave.nodes.get(&root_node_identifier).unwrap();
            assert!(root_node_1.from.is_empty());
            assert!(root_node_1.to.is_empty());
        }
        assert!(weave.add_node(
            blank_moveable_node(root_node_2_identifier, [], []),
            None,
            false,
        ));

        assert!(weave.add_node(
            blank_moveable_node(child_node_1_identifier, [root_node_identifier], []),
            None,
            false,
        ));
        assert!(weave.add_node(
            blank_moveable_node(child_node_2_identifier, [root_node_identifier], []),
            None,
            false,
        ));
        assert!(weave.add_node(
            blank_moveable_node(child_node_3_identifier, [child_node_2_identifier], []),
            None,
            false,
        ));
        assert!(weave.add_node(
            blank_moveable_node(child_node_4_identifier, [child_node_3_identifier], []),
            None,
            false,
        ));
        assert!(weave.add_node(
            blank_moveable_node(
                child_node_5_identifier,
                [child_node_3_identifier, child_node_4_identifier],
                []
            ),
            None,
            false,
        ));
        assert!(weave.add_node(
            blank_moveable_node(child_node_6_identifier, [child_node_5_identifier], []),
            None,
            false,
        ));
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
    fn update_node_parents_propagation() {}

    #[test]
    fn update_node_children_propagation() {}

    #[test]
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
    fn update_node_parents_check_loop() {}

    #[test]
    fn update_node_children_check_loop() {}

    #[test]
    fn remove_node_check_loop() {}

    #[test]
    fn update_node_lock_propagation() {}

    #[test]
    fn update_node_lock_controlled_propagation() {}

    #[test]
    fn add_node_lock_propagation() {}

    #[test]
    fn update_node_parents_lock_propagation() {}

    #[test]
    fn update_node_children_lock_propagation() {}

    #[test]
    fn remove_node_lock_propagation() {}

    #[test]
    fn update_node_activation_propagation() {}

    #[test]
    fn update_node_activation_controlled_propagation() {}

    #[test]
    fn add_node_activation_propagation() {}

    #[test]
    fn update_node_parents_activation_propagation() {}

    #[test]
    fn update_node_children_activation_propagation() {}

    #[test]
    fn remove_node_activation_propagation() {}

    #[test]
    fn add_node_deduplication() {}

    #[test]
    fn update_node_parents_deduplication() {}

    #[test]
    fn update_node_children_deduplication() {}*/
}
