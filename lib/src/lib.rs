use std::collections::{HashMap, HashSet, hash_map::Entry};

use ulid::Ulid;

mod content;
mod format;

use crate::content::{Model, Node};

/* TODO:
- Node activation/deactivation
    - Only adjustable on unlocked nodes; Locked nodes *must* either be activated or have activated siblings
    - Activation/deactivation is recursive, similar to node locking
        - Unlike node locking, it will only apply to one parent node per-layer rather than all parent nodes
- Node content deduplication
- Unit tests
    - Node management & update propagation (propagating changes into node children & parents, root_nodes updating)
    - Model management & update propagation (model addition/removal, model_nodes updating)
    - Node loop checking
    - Node locking
    - Node activation
    - Node content deduplication */

#[derive(Default)]
pub struct Weave {
    nodes: HashMap<Ulid, Node>,
    models: HashMap<Ulid, Model>,

    root_nodes: HashSet<Ulid>,
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
            let mut visited = HashSet::from([node.id]);
            for parent in &node.from {
                if self.has_parent_loop(parent, &mut visited) {
                    return false;
                }
            }
            for child in &node.to {
                if self.has_child_loop(child, &mut visited) {
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

        true
    }
    fn has_parent_loop(&self, identifier: &Ulid, visited: &mut HashSet<Ulid>) -> bool {
        if visited.contains(identifier) {
            return true;
        }
        visited.insert(*identifier);
        if let Some(node) = self.nodes.get(identifier) {
            for parent in &node.from {
                if self.has_parent_loop(parent, visited) {
                    return true;
                }
            }

            false
        } else {
            false
        }
    }
    fn has_child_loop(&self, identifier: &Ulid, visited: &mut HashSet<Ulid>) -> bool {
        if visited.contains(identifier) {
            return true;
        }
        visited.insert(*identifier);
        if let Some(node) = self.nodes.get(identifier) {
            for child in &node.to {
                if self.has_child_loop(child, visited) {
                    return true;
                }
            }

            false
        } else {
            false
        }
    }
    pub fn update_node_moveability(&mut self, identifier: &Ulid, moveable: bool) -> bool {
        if let Some(node) = self.nodes.get_mut(identifier) {
            match moveable {
                true => {
                    if !node.moveable && node.content.moveable() {
                        node.moveable = true;
                        for parent in node.from.clone() {
                            self.update_node_moveability(&parent, true);
                        }

                        true
                    } else {
                        node.moveable
                    }
                }
                false => {
                    if node.moveable {
                        node.moveable = false;
                        for parent in node.from.clone() {
                            self.update_node_moveability(&parent, false);
                        }

                        true
                    } else {
                        !node.moveable
                    }
                }
            }
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
            let mut visited = HashSet::from([*identifier]);
            for parent in &parents {
                if self.has_parent_loop(parent, &mut visited) {
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
                let mut visited = HashSet::from([*identifier]);
                for child in &children {
                    if self.has_child_loop(child, &mut visited) {
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
}
