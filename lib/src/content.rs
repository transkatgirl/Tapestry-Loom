#![allow(missing_docs)]

use std::{collections::HashSet, fmt::Display, ops::Range};

use dissimilar::Chunk;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use ulid::Ulid;

use crate::document::{Weave, WeaveView};

/* TODO:
- Weave content building/updating
- Node splitting/merging
- Documentation */

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
    pub content: TextOrBytes,
    pub probability: Option<Decimal>,

    pub node: Option<&'w Node>,
    pub model: Option<&'w Model>,
}

impl<'w> WeaveTimeline<'w> {
    pub fn text(&self) -> String {
        self.timeline
            .iter()
            .map(|(node, _model)| node.content.clone().text())
            .collect::<Vec<String>>()
            .concat()
    }
    pub fn bytes(&self) -> Vec<u8> {
        self.timeline
            .iter()
            .flat_map(|(node, _model)| node.content.clone().bytes())
            .collect()
    }
    pub fn annotated(&self) -> Vec<AnnotatedSnippet<'w>> {
        self.timeline
            .iter()
            .flat_map(|(node, model)| {
                node.content
                    .clone()
                    .snippets()
                    .into_iter()
                    .map(|snippet| AnnotatedSnippet {
                        node: Some(node),
                        content: snippet.content,
                        probability: snippet.probability,
                        model: *model,
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

impl Weave {
    pub fn split_node(&mut self, identifier: &Ulid, index: usize) -> Option<Ulid> {
        todo!()
    }
    pub fn merge_nodes(&mut self, left: &Ulid, right: &Ulid) -> Option<Ulid> {
        let (Some(left), _) = self.get_node(left) else {
            return None;
        };
        let (Some(right), _) = self.get_node(right) else {
            return None;
        };
        if !(left.to.contains(&right.id) && right.from.contains(&left.id)) {
            return None;
        }

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
    Bytes(ByteNode),
    Token(TokenNode),
    TextToken(TextTokenNode),
    Blank,
}

impl NodeContent {
    pub fn merge(left: Self, right: Self) -> Option<Self> {
        if left.model() == right.model() {
            Some(match left {
                Self::Text(left) => match right {
                    Self::Text(right) => Self::Text(TextNode {
                        content: [left.content, right.content].concat(),
                        model: left.model,
                    }),
                    Self::Bytes(mut right) => {
                        let mut content = left.content.into_bytes();
                        content.append(&mut right.content);

                        Self::Bytes(ByteNode {
                            content,
                            model: left.model,
                        })
                    }
                    Self::Token(right) => {
                        //let left_token = TextOrToken()

                        todo!()
                    }
                    Self::TextToken(right) => {
                        todo!()
                    }
                    Self::Blank => Self::Text(left),
                },
                Self::Bytes(left) => match right {
                    Self::Text(right) => {
                        todo!()
                    }
                    Self::Bytes(right) => {
                        todo!()
                    }
                    Self::Token(right) => {
                        todo!()
                    }
                    Self::TextToken(right) => {
                        todo!()
                    }
                    Self::Blank => Self::Bytes(left),
                },
                Self::Token(left) => match right {
                    Self::Text(right) => {
                        todo!()
                    }
                    Self::Bytes(right) => {
                        todo!()
                    }
                    Self::Token(right) => {
                        todo!()
                    }
                    Self::TextToken(right) => {
                        todo!()
                    }
                    Self::Blank => Self::Token(left),
                },
                Self::TextToken(left) => match right {
                    Self::Text(right) => {
                        todo!()
                    }
                    Self::Bytes(right) => {
                        todo!()
                    }
                    Self::Token(right) => {
                        todo!()
                    }
                    Self::TextToken(right) => {
                        todo!()
                    }
                    Self::Blank => Self::TextToken(left),
                },
                Self::Blank => right,
            })
        } else {
            None
        }
    }
    pub fn split(self, index: usize) -> [Self; 2] {
        match self {
            Self::Text(content) => {
                todo!()
            }
            Self::Bytes(content) => {
                todo!()
            }
            Self::Token(content) => {
                todo!()
            }
            Self::TextToken(content) => {
                todo!()
            }
            Self::Blank => [Self::Blank, Self::Blank],
        }
    }
}

pub trait NodeContents: Display {
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
            Self::Bytes(content) => content.model(),
            Self::Token(content) => content.model(),
            Self::TextToken(content) => content.model(),
            Self::Blank => None,
        }
    }
}

impl Display for NodeContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(content) => write!(f, "{content}"),
            Self::Bytes(content) => write!(f, "{content}"),
            Self::Token(content) => write!(f, "{content}"),
            Self::TextToken(content) => write!(f, "{content}"),
            Self::Blank => write!(f, "No Content"),
        }
    }
}

impl TextualNodeContents for NodeContent {
    fn text(self) -> String {
        match self {
            Self::Text(content) => content.text(),
            Self::Bytes(content) => content.text(),
            Self::Token(content) => content.text(),
            Self::TextToken(content) => content.text(),
            Self::Blank => String::new(),
        }
    }
    fn bytes(self) -> Vec<u8> {
        match self {
            Self::Text(content) => content.bytes(),
            Self::Bytes(content) => content.bytes(),
            Self::Token(content) => content.bytes(),
            Self::TextToken(content) => content.bytes(),
            Self::Blank => Vec::new(),
        }
    }
    fn snippets(self) -> Vec<Snippet> {
        match self {
            Self::Text(content) => content.snippets(),
            Self::Bytes(content) => content.snippets(),
            Self::Token(content) => content.snippets(),
            Self::TextToken(content) => content.snippets(),
            Self::Blank => Vec::new(),
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

impl Display for TextNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = self.clone().text();

        if text.is_empty() {
            return write!(f, "No Content");
        }

        write!(f, "{text}")
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
            content: TextOrBytes::Text(self.content),
        }]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct ByteNode {
    pub content: Vec<u8>,
    pub model: Option<NodeModel>,
}

impl NodeContents for ByteNode {
    fn model(&self) -> Option<&NodeModel> {
        self.model.as_ref()
    }
}

impl Display for ByteNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = self.clone().text();

        if text.is_empty() {
            return write!(f, "No Content");
        }

        write!(f, "{text}")
    }
}

impl TextualNodeContents for ByteNode {
    fn text(self) -> String {
        String::from_utf8_lossy(&self.bytes()).to_string()
    }
    fn bytes(self) -> Vec<u8> {
        self.content
    }
    fn snippets(self) -> Vec<Snippet> {
        if let Ok(text) = str::from_utf8(&self.content) {
            return vec![Snippet {
                probability: None,
                content: TextOrBytes::Text(text.to_string()),
            }];
        }

        vec![Snippet {
            probability: None,
            content: TextOrBytes::Bytes(self.content),
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

impl Display for TokenNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = self.clone().text();

        if text.is_empty() {
            return write!(f, "No Content");
        }

        write!(f, "{text}")
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
    pub content: TextOrBytes,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub enum TextOrBytes {
    Text(String),
    Bytes(Vec<u8>),
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

impl Display for TextTokenNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = self.clone().text();

        if text.is_empty() {
            return write!(f, "No Content");
        }

        write!(f, "{text}")
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
                TextOrToken::Bytes(bytes) => bytes,
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
                TextOrToken::Bytes(bytes) => {
                    let range = Range {
                        start: index,
                        end: index + bytes.len(),
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
                    content: TextOrBytes::Text(text.to_string()),
                });
                break;
            }

            range.end += 1;
            if range.end >= data.len() {
                range = original_range;
                snippets.push(Snippet {
                    probability,
                    content: TextOrBytes::Bytes(data[range.start..range.end].to_vec()),
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
    Bytes(Vec<u8>),
    Token(Vec<NodeToken>),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct DiffNode {
    pub content: Diff,
    pub model: Option<NodeModel>,
}

impl NodeContents for DiffNode {
    fn model(&self) -> Option<&NodeModel> {
        self.model.as_ref()
    }
}

impl Display for DiffNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self.content.count();

        if count.insertions == 1 && count.deletions < 2 {
            for modification in &self.content.content {
                if let ModificationContent::Insertion(text) = &modification.content {
                    return write!(f, "{text}");
                }
            }
        }

        write!(f, "{count}")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct Diff {
    pub content: Vec<Modification>,
}

impl Diff {
    pub fn new(before: &str, after: &str) -> Self {
        let chunks = dissimilar::diff(before, after);

        let mut index = 0;
        let mut modifications = Vec::with_capacity(chunks.len());

        for chunk in chunks {
            match chunk {
                Chunk::Equal(content) => index += content.len(),
                Chunk::Insert(content) => {
                    modifications.push(Modification {
                        index,
                        content: ModificationContent::Insertion(content.to_string()),
                    });
                    index += content.len();
                }
                Chunk::Delete(content) => {
                    modifications.push(Modification {
                        index,
                        content: ModificationContent::Deletion(content.len()),
                    });
                    index += content.len();
                }
            }
        }

        Self {
            content: modifications,
        }
    }
    pub fn apply(&self, text: &mut String) {
        for modification in &self.content {
            modification.apply(text);
        }
    }
    pub fn count(&self) -> ModificationCount {
        let mut insertions: usize = 0;
        let mut deletions: usize = 0;

        for modification in &self.content {
            match modification.content {
                ModificationContent::Insertion(_) => insertions += 1,
                ModificationContent::Deletion(_) => deletions += 1,
            }
        }

        ModificationCount {
            total: insertions + deletions,
            insertions,
            deletions,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct Modification {
    pub index: usize,
    pub content: ModificationContent,
}

impl Modification {
    pub fn apply(&self, text: &mut String) {
        match &self.content {
            ModificationContent::Insertion(content) => text.insert_str(self.index, content),
            ModificationContent::Deletion(length) => text.replace_range(self.index..*length, ""),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub enum ModificationContent {
    Insertion(String),
    Deletion(usize),
}

impl ModificationContent {
    pub fn len(&self) -> usize {
        match self {
            ModificationContent::Insertion(content) => content.len(),
            ModificationContent::Deletion(length) => *length,
        }
    }
    pub fn is_empty(&self) -> bool {
        match self {
            ModificationContent::Insertion(content) => content.is_empty(),
            ModificationContent::Deletion(length) => *length == 0,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct ModificationCount {
    pub total: usize,
    pub insertions: usize,
    pub deletions: usize,
}

impl Display for ModificationCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.deletions > 0 {
            if self.insertions > 0 {
                write!(
                    f,
                    "{} Insertions, {} Deletions",
                    self.insertions, self.deletions
                )
            } else {
                write!(f, "{} Deletions", self.deletions)
            }
        } else if self.insertions > 0 {
            write!(f, "{} Insertions", self.insertions)
        } else {
            write!(f, "No Changes")
        }
    }
}
