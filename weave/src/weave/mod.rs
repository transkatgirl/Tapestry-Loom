use std::collections::HashMap;

use ulid::Ulid;

use crate::weave::format::NodeTokens;

mod format;

pub struct Weave {
    nodes: HashMap<Ulid, Node>,
    models: HashMap<Ulid, Model>,
}

pub struct Node {
    pub id: Ulid,
    pub to: Vec<Ulid>,
    pub content: NodeContent,
}

pub struct Model {
    pub label: String,
    pub style: String,
}

pub enum NodeContent {
    Text(TextNode),
    Token(TokenNode),
    Diff(DiffNode),
}

pub struct TextNode {
    pub content: String,
    pub model: Option<NodeModel>,
}

pub struct TokenNode {
    pub content: NodeTokens,
    pub model: Option<NodeModel>,
}

pub struct DiffNode {
    pub content: Vec<Modification>,
}

pub struct Modification {
    pub index: usize,
    pub r#type: ModificationType,
    pub content: String,
}

pub enum ModificationType {
    Insertion,
    Deletion,
}

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
