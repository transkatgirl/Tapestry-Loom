use std::{
    collections::{BTreeSet, HashMap, HashSet},
    iter,
    ops::Range,
};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use ulid::Ulid;

use crate::Weave;

/* TODO:
- Weave node sorting API
- Weave content building/updating
- Node splitting/merging
- Implement Clone on all types in the module
- Documentation
- Unit tests */

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct WeaveSnapshot<'w> {
    pub nodes: &'w HashMap<Ulid, Node>,
    pub models: &'w HashMap<Ulid, Model>,
    pub root_nodes: &'w BTreeSet<Ulid>,
}

impl<'w> From<&'w Weave> for WeaveSnapshot<'w> {
    fn from(input: &'w Weave) -> WeaveSnapshot<'w> {
        Self {
            nodes: &input.nodes,
            models: &input.models,
            root_nodes: &input.root_nodes,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: Ulid,
    pub to: HashSet<Ulid>,
    pub from: HashSet<Ulid>,
    pub active: bool,
    pub content: NodeContent,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct WeaveTimeline<'w> {
    pub timeline: Vec<(&'w Node, Option<&'w Model>)>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct AnnotatedSnippet<'w> {
    pub content: String,
    pub probability: Option<Decimal>,

    pub node: &'w Node,
    pub model: Option<&'w Model>,
}

impl<'w> WeaveTimeline<'w> {
    pub fn text(&self) -> String {
        self.timeline
            .iter()
            .filter_map(|(node, _model)| node.content.clone().text())
            .collect::<Vec<String>>()
            .concat()
    }
    pub fn annotated(&self) -> Vec<AnnotatedSnippet> {
        self.timeline
            .iter()
            .flat_map(|(node, model)| match &node.content {
                NodeContent::Text(content) => iter::once(AnnotatedSnippet {
                    node,
                    content: content.content.clone(),
                    probability: None,
                    model: *model,
                })
                .collect::<Vec<_>>(),
                NodeContent::Token(content) => content
                    .clone()
                    .snippets()
                    .into_iter()
                    .map(|snippet| AnnotatedSnippet {
                        node,
                        content: snippet.content,
                        probability: snippet.probability,
                        model: *model,
                    })
                    .collect::<Vec<_>>(),
                NodeContent::TextToken(content) => content
                    .clone()
                    .snippets()
                    .into_iter()
                    .map(|snippet| AnnotatedSnippet {
                        node,
                        content: snippet.content,
                        probability: snippet.probability,
                        model: *model,
                    })
                    .collect::<Vec<_>>(),
            })
            .collect()
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

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
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
}

impl NodeContent {
    pub fn text(self) -> Option<String> {
        match self {
            NodeContent::Text(content) => Some(content.content),
            NodeContent::Token(content) => Some(content.text()),
            NodeContent::TextToken(content) => Some(content.text()),
        }
    }
    pub fn model(&self) -> Option<&NodeModel> {
        match self {
            NodeContent::Text(content) => content.model.as_ref(),
            NodeContent::Token(content) => content.model.as_ref(),
            NodeContent::TextToken(content) => content.model.as_ref(),
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
        String::from_utf8_lossy(&self.bytes()).to_string()
    }
    pub fn bytes(self) -> Vec<u8> {
        self.content
            .into_iter()
            .flat_map(|token| token.content)
            .collect()
    }
    pub fn snippets(self) -> Vec<Snippet> {
        todo!()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct NodeToken {
    pub probability: Decimal,
    pub content: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct Snippet {
    pub probability: Option<Decimal>,
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TextTokenNode {
    pub content: Vec<TextOrToken>,
    pub model: Option<NodeModel>,
}

impl TextTokenNode {
    pub fn text(self) -> String {
        String::from_utf8_lossy(&self.bytes()).to_string()
    }
    pub fn bytes(self) -> Vec<u8> {
        let mut data = Vec::new();

        for content in self.content {
            data.append(&mut match content {
                TextOrToken::Text(text) => text.into_bytes(),
                TextOrToken::Token(token) => {
                    token.into_iter().flat_map(|token| token.content).collect()
                }
            });
        }

        data
    }
    pub fn snippets(self) -> Vec<Snippet> {
        todo!()
    }
}

/*fn into_snippets(tokens: Vec<NodeToken>) -> Vec<Snippet> {
    let mut data = Vec::new();
    let mut ranges = Vec::with_capacity(tokens.len());

    for mut content in tokens {
        ranges.push((
            Range {
                start: data.len(),
                end: data.len() + content.content.len(),
            },
            content.probability,
        ));
        data.append(&mut content.content);
    }

    /*let mut tokens = Vec::with_capacity(ranges.len());
    for range in ranges {

    }*/

    todo!()
}*/

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum TextOrToken {
    Text(String),
    Token(Vec<NodeToken>),
}

/*#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct Diff {
    pub content: Vec<Modification>,
}

impl Diff {
    pub fn apply(&self, before: &str) -> String {
        todo!()
    }
    fn apply_annotated(&self, content: &mut [AnnotatedSnippet]) {
        todo!()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct Modification {
    pub index: usize,
    pub r#type: ModificationType,
    pub content: String,
}

impl Modification {
    fn apply_text(&self, text: &mut str) {
        todo!()
    }
    fn apply_annotated(&self, content: &mut [AnnotatedSnippet]) {
        todo!()
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub enum ModificationType {
    Insertion,
    Deletion,
}
*/
