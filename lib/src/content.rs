use std::collections::HashSet;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use ulid::Ulid;

use crate::Weave;

/* TODO:
- Node sorting API
- Weave content building/updating
- Unit tests */

#[derive(Serialize, Deserialize, PartialEq)]
pub struct Node {
    pub id: Ulid,
    pub to: HashSet<Ulid>,
    pub from: HashSet<Ulid>,
    pub moveable: bool,
    pub active: bool,
    pub content: NodeContent,
}

impl Weave {
    /*pub fn add_node_deduplicated(
        &mut self,
        node: Node,
        model: Option<Model>,
        skip_loop_check: bool,
    ) -> Option<Ulid> {
        for parent in &node.from {
            if let Some(parent) = self.nodes.get(parent) {
                for child in parent.to.clone() {
                    if let Some(child) = self.nodes.get_mut(&child) {
                        if child.content == node.content {
                            if node.active {
                                child.active = node.active;
                            }
                            let identifier = child.id;
                            if !node.moveable {
                                self.update_node_moveability(&identifier, false);
                            }
                            return Some(identifier);
                        }
                    }
                }
            }
        }
        for child in &node.to {
            if let Some(child) = self.nodes.get(child) {
                for parent in child.from.clone() {
                    if let Some(parent) = self.nodes.get_mut(&parent) {
                        if parent.content == node.content {
                            if node.active {
                                parent.active = node.active;
                            }
                            let identifier = parent.id;
                            if !node.moveable {
                                self.update_node_moveability(&identifier, false);
                            }
                            return Some(identifier);
                        }
                    }
                    if node.active {
                        self.update_node_activity(&parent, true);
                    }
                }
            }
        }
        let identifier = node.id;
        match self.add_node(node, model, skip_loop_check) {
            true => Some(identifier),
            false => None,
        }
    }
    pub fn update_node_activity(&mut self, identifier: &Ulid, active: bool) {
        if let Some(node) = self.nodes.get_mut(identifier) {
            if node.moveable {
                node.active = active;
                for parent in node.from.clone() {
                    self.update_node_activity(&parent, active);
                }
            }
        }
    }*/
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Model {
    pub id: Ulid,
    pub label: String,
    pub style: String,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum NodeContent {
    Text(TextNode),
    Token(TokenNode),
    Diff(DiffNode),
}

impl NodeContent {
    pub fn text(&self) -> Option<String> {
        match self {
            NodeContent::Text(content) => Some(content.content.clone()),
            NodeContent::Token(content) => Some(content.text()),
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

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct NodeModel {
    pub id: Ulid,
    pub parameters: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct TextNode {
    pub content: String,
    pub model: Option<NodeModel>,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct TokenNode {
    pub content: Vec<NodeToken>,
    pub model: Option<NodeModel>,
}

impl TokenNode {
    pub fn text(&self) -> String {
        let data: Vec<u8> = self
            .content
            .iter()
            .flat_map(|token| token.content.clone())
            .collect();

        String::from_utf8_lossy(&data).to_string()
    }
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct NodeToken {
    pub content: Vec<u8>,
    pub probability: Decimal,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct DiffNode {
    pub content: Vec<Modification>,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
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

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum ModificationType {
    Insertion,
    Deletion,
}
