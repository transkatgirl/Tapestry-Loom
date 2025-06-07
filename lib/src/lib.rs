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
    pub fn add_node(&mut self, node: Node, parent: Option<Ulid>, model: Option<Model>) -> bool {
        if self.nodes.contains_key(&node.id) {
            return false;
        }
        for identifier in &node.to {
            if !self.nodes.contains_key(identifier) {
                return false;
            }
        }
        if let Some(parent) = parent {
            match self.nodes.entry(parent) {
                Entry::Occupied(mut entry) => {
                    entry.get_mut().to.insert(node.id);
                }
                Entry::Vacant(_entry) => {
                    return false;
                }
            }
        } else {
            self.root_nodes.insert(node.id);
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
    pub fn remove_node(&mut self, identifier: &Ulid) -> bool {
        match self.models.remove(identifier) {
            Some(node) => {
                todo!()
            }
            None => false,
        }
    }
    pub fn get_node(&self, identifier: &Ulid) -> Option<&Node> {
        self.nodes.get(identifier)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Node {
    pub id: Ulid,
    pub to: HashSet<Ulid>,
    pub content: NodeContent,
}

#[derive(Serialize, Deserialize)]
pub struct Model {
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
