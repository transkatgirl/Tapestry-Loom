use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use ulid::Ulid;

mod format;

use self::format::NodeTokens;

pub struct Weave {
    nodes: HashMap<Ulid, Node>,
    models: HashMap<Ulid, Model>,
}

impl Weave {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            models: HashMap::new(),
        }
    }
    pub fn get_node(&self, identifier: &Ulid) -> Option<&Node> {
        self.nodes.get(identifier)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Node {
    pub id: Ulid,
    pub to: Vec<Ulid>,
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

#[derive(Serialize, Deserialize)]
pub struct NodeModel {
    pub id: Ulid,
    pub parameters: HashMap<String, String>,
}

impl Weave {}

impl From<format::Weave> for Weave {
    fn from(input: format::Weave) -> Self {
        todo!()
    }
}

impl From<Weave> for format::Weave {
    fn from(input: Weave) -> Self {
        todo!()
    }
}
