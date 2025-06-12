use std::{
    collections::{BTreeSet, HashMap, HashSet},
    iter,
    ops::Range,
};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use ulid::Ulid;

use crate::{Weave, WeaveView};

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

// Copied from Weave's implementation of build_timelines(); shouldn't require additional unit tests
impl<'w> WeaveSnapshot<'w> {
    fn build_timelines<'a>(&'a self, timelines: &mut Vec<Vec<&'a Node>>) {
        let mut new_timelines = Vec::new();
        let mut modified = false;

        for timeline in timelines.iter_mut() {
            if let Some(node) = timeline.last() {
                let mut added_node = false;

                for child in node
                    .to
                    .iter()
                    .filter_map(|id| self.nodes.get(id))
                    .filter(|node| node.active)
                {
                    if !added_node {
                        timeline.push(child);
                        added_node = true;
                    } else {
                        let mut new_timeline = timeline.clone();
                        new_timeline.pop();
                        new_timeline.push(child);
                        new_timelines.push(new_timeline);
                    }

                    modified = true;
                }
            }
        }
        for timeline in new_timelines.into_iter() {
            timelines.push(timeline);
        }
        if modified {
            self.build_timelines(timelines);
        }
    }
}

// Copied from Weave's implementation of WeaveView; shouldn't require additional unit tests
impl<'w> WeaveView for WeaveSnapshot<'w> {
    fn get_node(&self, identifier: &Ulid) -> (Option<&Node>, Option<&Model>) {
        let node = self.nodes.get(identifier);
        let model = node
            .and_then(|node| node.content.model())
            .and_then(|node_model| self.models.get(&node_model.id));

        (node, model)
    }
    fn get_root_nodes(&self) -> impl Iterator<Item = (&Node, Option<&Model>)> {
        self.root_nodes
            .iter()
            .flat_map(|identifier| self.nodes.get(identifier))
            .map(|node| {
                (
                    node,
                    node.content
                        .model()
                        .and_then(|node_model| self.models.get(&node_model.id)),
                )
            })
    }
    fn get_active_timelines(&self) -> Vec<WeaveTimeline> {
        let mut timelines: Vec<Vec<&Node>> = self
            .root_nodes
            .iter()
            .flat_map(|identifier| self.nodes.get(identifier))
            .filter(|node| node.active)
            .map(|node| Vec::from([node]))
            .collect();
        self.build_timelines(&mut timelines);

        let mut hydrated_timelines: Vec<WeaveTimeline<'_>> = timelines
            .iter()
            .map(|timeline| WeaveTimeline {
                timeline: timeline
                    .iter()
                    .map(|node| {
                        (
                            *node,
                            node.content
                                .model()
                                .and_then(|node_model| self.models.get(&node_model.id)),
                        )
                    })
                    .collect(),
            })
            .collect();

        hydrated_timelines.sort_by_key(|timeline| timeline.timeline.len());

        hydrated_timelines
    }
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

pub struct FrozenWeave {
    weave: Weave,
    timeline: usize,
    changes: Diff,
}

impl FrozenWeave {
    pub fn weave(&self) -> WeaveSnapshot {
        WeaveSnapshot::from(&self.weave)
    }
    pub fn text(&self) -> String {
        let text = self.weave.get_active_timelines()[self.timeline].text();
        self.changes.apply(&text)
    }
    pub fn content(&self) -> (WeaveTimeline, &Diff) {
        (
            self.weave.get_active_timelines()[self.timeline].clone(),
            &self.changes,
        )
    }
    pub fn update(&mut self, content: &str) {
        let before = self.text();
        self.changes = Diff::new(&before, content);
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
                NodeContent::Blank => iter::once(AnnotatedSnippet {
                    node,
                    content: "".to_string(),
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
    Blank,
}

impl NodeContent {
    pub fn text(self) -> Option<String> {
        match self {
            Self::Text(content) => Some(content.content),
            Self::Token(content) => Some(content.text()),
            Self::TextToken(content) => Some(content.text()),
            Self::Blank => None,
        }
    }
    pub fn model(&self) -> Option<&NodeModel> {
        match self {
            Self::Text(content) => content.model.as_ref(),
            Self::Token(content) => content.model.as_ref(),
            Self::TextToken(content) => content.model.as_ref(),
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

        into_snippets(data, ranges)
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

        into_snippets(data, ranges)
    }
}

fn into_snippets(data: Vec<u8>, ranges: Vec<(Range<usize>, Option<Decimal>)>) -> Vec<Snippet> {
    let mut snippets: Vec<Snippet> = Vec::with_capacity(ranges.len());
    let mut last_range: Range<usize> = Range::default();

    for (mut range, probability) in ranges.into_iter() {
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
            } else {
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
