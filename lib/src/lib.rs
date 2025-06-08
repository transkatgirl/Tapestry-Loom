use std::collections::{HashMap, HashSet, hash_map::Entry};

use serde::{Deserialize, Serialize};

use ulid::Ulid;

mod format;

use crate::format::CompactWeave;

use self::format::NodeTokens;

#[derive(Default)]
pub struct Weave {
    nodes: HashMap<Ulid, Node>,
    models: HashMap<Ulid, Model>,

    root_nodes: HashSet<Ulid>,
    model_nodes: HashMap<Ulid, HashSet<Ulid>>,
}

impl Weave {
    pub fn add_node(&mut self, node: Node, model: Option<Model>) -> bool {
        if self.nodes.contains_key(&node.id) {
            return false;
        }
        if node.from.is_empty() {
            self.root_nodes.insert(node.id);
        }
        for child in &node.to {
            if let Some(child) = self.nodes.get_mut(child) {
                child.from.insert(node.id);
            }
        }
        for parent in &node.from {
            if let Some(parent) = self.nodes.get_mut(parent) {
                parent.to.insert(node.id);
            }
        }
        if let Some(node_model) = node.content.model() {
            if let Some(model) = model {
                self.models.insert(node_model.id, model);
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
    pub fn update_node_parents(&mut self, identifier: &Ulid, parents: HashSet<Ulid>) {
        if let Some(old_parents) = self.nodes.get_mut(identifier).map(|node| node.from.clone()) {
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
        }
    }
    pub fn update_node_children(&mut self, identifier: &Ulid, children: HashSet<Ulid>) {
        if let Some(old_children) = self.nodes.get_mut(identifier).map(|node| node.to.clone()) {
            for child in &old_children {
                if let Some(child) = self.nodes.get_mut(child) {
                    child.from.remove(identifier);
                }
            }
            for child in &children {
                if let Some(child) = self.nodes.get_mut(child) {
                    child.from.insert(*identifier);
                }
            }
            if let Some(node) = self.nodes.get_mut(identifier) {
                node.to = children;
            }
        }
    }
    pub fn remove_node(&mut self, identifier: &Ulid, remove_children: bool) {
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
                if remove_children {
                    self.remove_node(child, true);
                }
            }
            if let Some(node_model) = node.content.model() {
                if let Some(model_nodes) = self.model_nodes.get_mut(&node_model.id) {
                    model_nodes.remove(&node.id);
                    if model_nodes.is_empty() {
                        self.models.remove(&node_model.id);
                    }
                }
            }
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

#[derive(Serialize, Deserialize)]
pub struct Node {
    pub id: Ulid,
    pub to: HashSet<Ulid>,
    pub from: HashSet<Ulid>,
    pub active: bool,
    pub content: NodeContent,
}

#[derive(Serialize, Deserialize)]
pub struct Model {
    pub id: Ulid,
    pub label: String,
    pub style: String,
}

#[derive(Serialize, Deserialize)]
pub enum NodeContent {
    Text(TextNode),
    Token(TokenNode),
    Diff(DiffNode),
}

impl NodeContent {
    pub fn model(&self) -> Option<&NodeModel> {
        match self {
            NodeContent::Text(content) => content.model.as_ref(),
            NodeContent::Token(content) => content.model.as_ref(),
            NodeContent::Diff(_content) => None,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct NodeModel {
    pub id: Ulid,
    pub parameters: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct TextNode {
    pub content: String,
    pub model: Option<NodeModel>,
}

#[derive(Serialize, Deserialize)]
pub struct TokenNode {
    pub content: NodeTokens,
    pub model: Option<NodeModel>,
}

#[derive(Serialize, Deserialize)]
pub struct DiffNode {
    pub content: Vec<Modification>,
}

#[derive(Serialize, Deserialize)]
pub struct Modification {
    pub index: usize,
    pub r#type: ModificationType,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub enum ModificationType {
    Insertion,
    Deletion,
}

impl Weave {}

impl From<CompactWeave> for Weave {
    fn from(input: CompactWeave) -> Self {
        /*let weave = Self::default();


        for (raw_identifier, value) in input.models {
            weave.models.get()
        }
        for (raw_identifier, value) in input.nodes {

        }*/

        todo!()
    }
}

impl From<Weave> for CompactWeave {
    fn from(input: Weave) -> Self {
        todo!()
    }
}
