use std::collections::{BTreeSet, HashMap, HashSet};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use ulid::Ulid;

use crate::Weave;

/* TODO:
- Weave node sorting API
- Weave content building/updating
- Node splitting/merging
- Implement Clone on all types in the module
- Unit tests */

#[derive(Serialize, Debug, PartialEq)]
pub struct WeaveContents<'w> {
    pub nodes: &'w HashMap<Ulid, Node>,
    pub models: &'w HashMap<Ulid, Model>,
    pub root_nodes: &'w BTreeSet<Ulid>,
}

impl<'w> From<&'w Weave> for WeaveContents<'w> {
    fn from(input: &'w Weave) -> WeaveContents<'w> {
        Self {
            nodes: &input.nodes,
            models: &input.models,
            root_nodes: &input.root_nodes,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Node {
    pub id: Ulid,
    pub to: HashSet<Ulid>,
    pub from: HashSet<Ulid>,
    pub moveable: bool,
    pub active: bool,
    pub content: NodeContent,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct WeaveTimeline<'w> {
    pub timeline: Vec<(&'w Node, &'w Model)>,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct AnnotatedSnippet<'w> {
    pub node: &'w Node,
    pub content: String,
    pub probability: Option<Decimal>,
    pub model: Option<&'w NodeModel>,
}

impl<'w> WeaveTimeline<'w> {
    pub fn get_text(&self, index: Option<usize>) -> String {
        todo!()
    }
    pub fn get_annotated_text(&self) -> Vec<AnnotatedSnippet> {
        todo!()
    }
}

impl Weave {
    pub fn split_node(&mut self, identifier: &Ulid, index: usize) -> Option<Ulid> {
        todo!()
    }
    pub fn merge_nodes(&mut self, identifiers: &[Ulid]) -> Option<Ulid> {
        todo!()
    }
    pub fn update_content(&mut self, content: String) {
        todo!()
    }

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

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq)]
pub struct Model {
    pub id: Ulid,
    pub label: String,
    pub style: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum NodeContent {
    Text(TextNode),
    Token(TokenNode),
    TextToken(TextTokenNode),
    Diff(DiffNode),
}

impl NodeContent {
    pub fn text(self) -> Option<String> {
        match self {
            NodeContent::Text(content) => Some(content.content),
            NodeContent::Token(content) => Some(content.text()),
            NodeContent::TextToken(content) => Some(content.text()),
            NodeContent::Diff(_content) => None,
        }
    }
    pub fn model(&self) -> Option<&NodeModel> {
        match self {
            NodeContent::Text(content) => content.model.as_ref(),
            NodeContent::Token(content) => content.model.as_ref(),
            NodeContent::TextToken(content) => content.model.as_ref(),
            NodeContent::Diff(_content) => None,
        }
    }
    pub fn moveable(&self) -> bool {
        match self {
            NodeContent::Text(_content) => true,
            NodeContent::Token(_content) => true,
            NodeContent::TextToken(_content) => true,
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

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct NodeModel {
    pub id: Ulid,
    pub parameters: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TextNode {
    pub content: String,
    pub model: Option<NodeModel>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TokenNode {
    pub content: Vec<NodeToken>,
    pub model: Option<NodeModel>,
}

impl TokenNode {
    pub fn text(self) -> String {
        let data: Vec<u8> = self
            .content
            .into_iter()
            .flat_map(|token| token.content)
            .collect();

        String::from_utf8_lossy(&data).to_string()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct NodeToken {
    pub probability: Decimal,
    pub content: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TextTokenNode {
    pub content: Vec<TextOrToken>,
    pub model: Option<NodeModel>,
}

impl TextTokenNode {
    pub fn text(self) -> String {
        let mut data = Vec::new();

        for content in self.content {
            data.append(&mut match content {
                TextOrToken::Text(text) => text.into_bytes(),
                TextOrToken::Token(token) => {
                    token.into_iter().flat_map(|token| token.content).collect()
                }
            });
        }

        String::from_utf8_lossy(&data).to_string()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum TextOrToken {
    Text(String),
    Token(Vec<NodeToken>),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct DiffNode {
    pub content: Vec<Modification>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct Modification {
    pub index: usize,
    pub r#type: ModificationType,
    pub content: String,
}

impl Modification {
    fn moveable(&self) -> bool {
        self.index == 0 && self.r#type == ModificationType::Insertion
    }
    pub fn apply(&self, before: &str) -> String {
        todo!()
    }
    fn apply_annotated(&self, content: &mut Vec<AnnotatedSnippet>) {
        todo!()
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub enum ModificationType {
    Insertion,
    Deletion,
}
