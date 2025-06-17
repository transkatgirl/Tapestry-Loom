#![allow(missing_docs)]

use std::{collections::HashSet, iter, ops::Range};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use ulid::Ulid;

use crate::document::{Weave, WeaveSnapshot, WeaveView};

/* TODO:
- Diff creation & application
- Weave content building/updating
- Node splitting/merging
- Documentation */

pub struct FrozenWeave {
    weave: Weave,
    timeline: usize,
    changes: Diff,
}

impl FrozenWeave {
    pub fn new(weave: Weave, timeline: usize, changes: Diff) -> Option<Self> {
        weave.get_active_timelines().get(timeline)?;

        Some(Self {
            weave,
            timeline,
            changes,
        })
    }
    pub fn weave(&self) -> WeaveSnapshot {
        WeaveSnapshot::from(&self.weave)
    }
    pub fn diff(&self) -> &Diff {
        &self.changes
    }
    pub fn text(&self) -> String {
        let mut text = self.weave.get_active_timelines()[self.timeline].text();
        self.changes.apply(&mut text);
        text
    }
    pub fn content(&self) -> Vec<AnnotatedSnippet> {
        let mut annotations = self
            .weave
            .get_active_timelines()
            .remove(self.timeline)
            .annotated();
        self.changes.apply_annotated(&mut annotations);
        annotations
    }
    pub fn update(&mut self, content: &str) {
        let before = self.text();
        self.changes = Diff::new(&before, content);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: Ulid,
    pub from: HashSet<Ulid>,
    pub to: HashSet<Ulid>,
    pub active: bool,
    pub bookmarked: bool,
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

    pub node: Option<&'w Node>,
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
    pub fn bytes(&self) -> Vec<u8> {
        self.timeline
            .iter()
            .filter_map(|(node, _model)| node.content.clone().bytes())
            .flatten()
            .collect()
    }
    pub fn annotated(&self) -> Vec<AnnotatedSnippet<'w>> {
        self.timeline
            .iter()
            .flat_map(|(node, model)| match &node.content {
                NodeContent::Text(content) => iter::once(AnnotatedSnippet {
                    node: Some(node),
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
                        node: Some(node),
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
                        node: Some(node),
                        content: snippet.content,
                        probability: snippet.probability,
                        model: *model,
                    })
                    .collect::<Vec<_>>(),
                NodeContent::Blank => iter::once(AnnotatedSnippet {
                    node: Some(node),
                    content: String::new(),
                    probability: None,
                    model: None,
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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Model {
    pub id: Ulid,
    pub label: String,
    pub color: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum NodeContent {
    Text(TextNode),
    Token(TokenNode),
    TextToken(TextTokenNode),
    Blank,
}

pub trait NodeContents {
    fn model(&self) -> Option<&NodeModel>;
}

pub trait TextualNodeContents: NodeContents {
    fn text(self) -> String;
    fn bytes(self) -> Vec<u8>;
    fn snippets(self) -> Vec<Snippet>;
}

impl NodeContents for NodeContent {
    fn model(&self) -> Option<&NodeModel> {
        match self {
            Self::Text(content) => content.model(),
            Self::Token(content) => content.model(),
            Self::TextToken(content) => content.model(),
            Self::Blank => None,
        }
    }
}

impl NodeContent {
    pub fn text(self) -> Option<String> {
        match self {
            Self::Text(content) => Some(content.text()),
            Self::Token(content) => Some(content.text()),
            Self::TextToken(content) => Some(content.text()),
            Self::Blank => None,
        }
    }
    pub fn bytes(self) -> Option<Vec<u8>> {
        match self {
            Self::Text(content) => Some(content.bytes()),
            Self::Token(content) => Some(content.bytes()),
            Self::TextToken(content) => Some(content.bytes()),
            Self::Blank => None,
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

impl NodeContents for TextNode {
    fn model(&self) -> Option<&NodeModel> {
        self.model.as_ref()
    }
}

impl TextualNodeContents for TextNode {
    fn text(self) -> String {
        self.content
    }
    fn bytes(self) -> Vec<u8> {
        self.content.into_bytes()
    }
    fn snippets(self) -> Vec<Snippet> {
        vec![Snippet {
            probability: None,
            content: self.content,
        }]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TokenNode {
    pub content: Vec<NodeToken>,
    pub model: Option<NodeModel>,
}

impl NodeContents for TokenNode {
    fn model(&self) -> Option<&NodeModel> {
        self.model.as_ref()
    }
}

impl TextualNodeContents for TokenNode {
    fn text(self) -> String {
        String::from_utf8_lossy(&self.bytes()).to_string()
    }
    fn bytes(self) -> Vec<u8> {
        self.content
            .into_iter()
            .flat_map(|token| token.content)
            .collect()
    }
    fn snippets(self) -> Vec<Snippet> {
        let mut index = 0;
        let mut ranges = Vec::with_capacity(self.content.len());

        for token in &self.content {
            let range = Range {
                start: index,
                end: index + token.content.len(),
            };
            index = range.end;

            ranges.push((range, Some(token.probability)));
        }

        let data = self.bytes();

        into_snippets(&data, ranges)
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

impl NodeContents for TextTokenNode {
    fn model(&self) -> Option<&NodeModel> {
        self.model.as_ref()
    }
}

impl TextualNodeContents for TextTokenNode {
    fn text(self) -> String {
        String::from_utf8_lossy(&self.bytes()).to_string()
    }
    fn bytes(self) -> Vec<u8> {
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
    fn snippets(self) -> Vec<Snippet> {
        let mut index = 0;
        let mut ranges = Vec::with_capacity(self.content.len());

        for segment in &self.content {
            match segment {
                TextOrToken::Text(text) => {
                    let range = Range {
                        start: index,
                        end: index + text.len(),
                    };
                    index = range.end;

                    ranges.push((range, None));
                }
                TextOrToken::Token(tokens) => {
                    for token in tokens {
                        let range = Range {
                            start: index,
                            end: index + token.content.len(),
                        };
                        index = range.end;

                        ranges.push((range, Some(token.probability)));
                    }
                }
            }
        }

        let data = self.bytes();

        into_snippets(&data, ranges)
    }
}

fn into_snippets(data: &[u8], ranges: Vec<(Range<usize>, Option<Decimal>)>) -> Vec<Snippet> {
    let mut snippets: Vec<Snippet> = Vec::with_capacity(ranges.len());
    let mut last_range: Range<usize> = Range::default();

    for (mut range, probability) in ranges {
        if last_range.end >= range.end {
            if let Some(snippet) = snippets.last_mut() {
                if let (Some(last_probability), Some(current_probability)) =
                    (snippet.probability, probability)
                {
                    snippet.probability = Some(last_probability * current_probability);
                }
            }
            continue;
        } else if last_range.end >= range.start {
            range.start = last_range.end;
        }

        let original_range = range.clone();

        loop {
            if let Ok(text) = str::from_utf8(&data[range.start..range.end]) {
                snippets.push(Snippet {
                    probability,
                    content: text.to_string(),
                });
                break;
            }

            range.end += 1;
            if range.end >= data.len() {
                range = original_range;
                snippets.push(Snippet {
                    probability,
                    content: String::from_utf8_lossy(&data[range.start..range.end]).to_string(),
                });
                break;
            }
        }

        last_range = range;
    }

    snippets
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum TextOrToken {
    Text(String),
    Token(Vec<NodeToken>),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct Diff {
    pub content: Vec<Modification>,
}

impl Diff {
    pub fn new(before: &str, after: &str) -> Self {
        todo!()
    }
    pub fn apply(&self, text: &mut String) {
        for modification in &self.content {
            modification.apply_text(text);
        }
    }
    pub fn apply_annotated(&self, content: &mut Vec<AnnotatedSnippet>) {
        for modification in &self.content {
            modification.apply_annotated(content);
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct Modification {
    pub index: usize,
    pub r#type: ModificationType,
    pub content: String,
}

impl Modification {
    fn apply_text(&self, text: &mut String) {
        match self.r#type {
            ModificationType::Insertion => text.insert_str(self.index, &self.content),
            ModificationType::Deletion => text.replace_range(self.index..self.content.len(), ""),
        }
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
