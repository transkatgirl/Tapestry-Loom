use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use ulid::Ulid;

use crate::{Weave, format::NodeTokens};

#[derive(Serialize, Deserialize)]
pub struct Node {
    pub id: Ulid,
    pub to: HashSet<Ulid>,
    pub from: HashSet<Ulid>,
    pub moveable: bool,
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
    pub fn text(&self) -> Option<String> {
        match self {
            NodeContent::Text(content) => Some(content.content.clone()),
            NodeContent::Token(content) => {
                let data: Vec<u8> = content
                    .content
                    .iter()
                    .flat_map(|(token, _probability)| token.clone())
                    .collect();

                Some(String::from_utf8_lossy(&data).to_string())
            }
            NodeContent::Diff(_content) => None,
        }
    }
    pub fn model(&self) -> Option<&NodeModel> {
        match self {
            NodeContent::Text(content) => content.model.as_ref(),
            NodeContent::Token(content) => content.model.as_ref(),
            NodeContent::Diff(_content) => None,
        }
    }
    pub fn moveable(&self) -> bool {
        match self {
            NodeContent::Text(_content) => true,
            NodeContent::Token(_content) => true,
            NodeContent::Diff(content) => {
                for modification in &content.content {
                    if !modification.moveable() {
                        return false;
                    }
                }

                true
            }
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

impl Modification {
    fn moveable(&self) -> bool {
        self.index == 0 && self.r#type == ModificationType::Insertion
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub enum ModificationType {
    Insertion,
    Deletion,
}
