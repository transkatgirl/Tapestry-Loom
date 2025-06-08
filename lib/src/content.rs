use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use ulid::Ulid;

use crate::{Weave, format::NodeTokens};

#[derive(Serialize, Deserialize)]
pub struct Node {
    pub id: Ulid,
    pub to: HashSet<Ulid>,
    pub from: HashSet<Ulid>,
    pub active: bool,
    pub content: NodeContent,
}

impl Weave {}

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
