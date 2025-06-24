#![allow(missing_docs)]

use std::{collections::HashSet, fmt::Display, iter, ops::Range};

use dissimilar::Chunk;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use ulid::Ulid;

use crate::document::{Weave, WeaveView};

/* TODO:
- Node splitting/merging
- Rewrite timeline code
    - Rewrite annotations to use content position references
- Weave content building/updating
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
pub struct Annotation<'w> {
    pub range: Range<usize>,
    pub probability: Option<Decimal>,

    pub node: Option<&'w Node>,
    pub model: Option<&'w Model>,
}

impl<'w> WeaveTimeline<'w> {
    /*pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes()).to_string()
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
    }*/
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
    Diff(DiffNode),
    Blank,
}

impl NodeContent {
    #[allow(clippy::match_same_arms)]
    pub fn linear(&self) -> bool {
        match self {
            Self::Text(_) => true,
            Self::Bytes(_) => true,
            Self::Token(_) => true,
            Self::TextToken(_) => true,
            Self::Diff(_) => false,
            Self::Blank => true,
        }
    }
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::missing_panics_doc)]
    pub fn merge(left: Self, right: Self) -> Option<Self> {
        if left.model() == right.model() || (left.linear() && right.linear()) {
            match left {
                Self::Text(left) => match right {
                    Self::Text(right) => Some(Self::Text(TextNode {
                        content: [left.content, right.content].concat(),
                        model: left.model,
                    })),
                    Self::Bytes(mut right) => {
                        let mut content = left.content.into_bytes();
                        content.append(&mut right.content);

                        Some(Self::Bytes(ByteNode {
                            content,
                            model: left.model,
                        }))
                    }
                    Self::Token(right) => {
                        //let left_token = TextOrToken()

                        todo!()
                    }
                    Self::TextToken(right) => {
                        todo!()
                    }
                    Self::Diff(_) => panic!(),
                    Self::Blank => Some(Self::Text(left)),
                },
                Self::Bytes(left) => match right {
                    Self::Text(right) => {
                        let mut content = left.content;
                        content.append(&mut right.content.into_bytes());

                        Some(Self::Bytes(ByteNode {
                            content,
                            model: left.model,
                        }))
                    }
                    Self::Bytes(mut right) => {
                        let mut content = left.content;
                        content.append(&mut right.content);

                        Some(Self::Bytes(ByteNode {
                            content,
                            model: left.model,
                        }))
                    }
                    Self::Token(right) => {
                        todo!()
                    }
                    Self::TextToken(right) => {
                        todo!()
                    }
                    Self::Diff(_) => panic!(),
                    Self::Blank => Some(Self::Bytes(left)),
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
                    Self::Diff(_) => panic!(),
                    Self::Blank => Some(Self::Token(left)),
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
                    Self::Diff(_) => panic!(),
                    Self::Blank => Some(Self::TextToken(left)),
                },
                Self::Diff(_) => panic!(),
                Self::Blank => Some(right),
            }
        } else {
            None
        }
    }
    pub fn split(self, index: usize) -> Option<[Self; 2]> {
        match self {
            Self::Text(content) => content
                .split(index)
                .map(|[left, right]| [Self::Text(left), Self::Text(right)]),
            Self::Bytes(content) => content
                .split(index)
                .map(|[left, right]| [Self::Bytes(left), Self::Bytes(right)]),
            Self::Token(content) => content
                .split(index)
                .map(|[left, right]| [Self::Token(left), Self::Token(right)]),
            Self::TextToken(content) => content
                .split(index)
                .map(|[left, right]| [Self::TextToken(left), Self::TextToken(right)]),
            Self::Diff(_) => None,
            Self::Blank => Some([Self::Blank, Self::Blank]),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct ContentAnnotation {
    pub range: Range<usize>,
    pub probability: Option<Decimal>,
}

impl ContentAnnotation {
    pub fn offset_forwards(&mut self, offset: usize) {
        self.range.start += offset;
        self.range.end += offset;
    }
    pub fn offset_backwards(&mut self, offset: usize) {
        self.range.start -= offset;
        self.range.end -= offset;
    }
    pub fn split(&self, index: usize) -> Option<[ContentAnnotation; 2]> {
        if index == 0 || index >= self.range.end {
            return None;
        }

        let mut left = self.range.clone();
        let mut right = self.range.clone();

        left.end -= index;
        right.start += index;

        Some([
            ContentAnnotation {
                range: left,
                probability: self.probability,
            },
            ContentAnnotation {
                range: right,
                probability: self.probability,
            },
        ])
    }
}

pub trait NodeContents: Display + Sized {
    fn model(&self) -> Option<&NodeModel>;
}

pub trait LinearNodeContents: NodeContents {
    fn bytes(self) -> Vec<u8>;
    fn annotations(&self) -> impl Iterator<Item = ContentAnnotation>;
    fn split(self, index: usize) -> Option<[Self; 2]>;
}

impl NodeContents for NodeContent {
    fn model(&self) -> Option<&NodeModel> {
        match self {
            Self::Text(content) => content.model(),
            Self::Bytes(content) => content.model(),
            Self::Token(content) => content.model(),
            Self::TextToken(content) => content.model(),
            Self::Diff(content) => content.model(),
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
            Self::Diff(content) => write!(f, "{content}"),
            Self::Blank => write!(f, "No Content"),
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
        if self.content.is_empty() {
            return write!(f, "No Content");
        }

        write!(f, "{}", self.content)
    }
}

impl LinearNodeContents for TextNode {
    fn bytes(self) -> Vec<u8> {
        self.content.into_bytes()
    }
    fn annotations(&self) -> impl Iterator<Item = ContentAnnotation> {
        iter::once(ContentAnnotation {
            probability: None,
            range: Range {
                start: 0,
                end: self.content.len(),
            },
        })
    }
    fn split(self, index: usize) -> Option<[Self; 2]> {
        if !self.content.is_char_boundary(index) {
            return None;
        }

        let mut left = self.content;
        let right = left.split_off(index);

        Some([
            Self {
                content: left,
                model: self.model.clone(),
            },
            Self {
                content: right,
                model: self.model,
            },
        ])
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
        let text = String::from_utf8_lossy(&self.content);

        if text.is_empty() {
            return write!(f, "No Content");
        }

        write!(f, "{text}")
    }
}

impl LinearNodeContents for ByteNode {
    fn bytes(self) -> Vec<u8> {
        self.content
    }
    fn annotations(&self) -> impl Iterator<Item = ContentAnnotation> {
        iter::once(ContentAnnotation {
            probability: None,
            range: Range {
                start: 0,
                end: self.content.len(),
            },
        })
    }
    fn split(self, index: usize) -> Option<[Self; 2]> {
        if index > self.content.len() {
            return None;
        }

        let mut left = self.content;
        let right = left.split_off(index);

        Some([
            Self {
                content: left,
                model: self.model.clone(),
            },
            Self {
                content: right,
                model: self.model,
            },
        ])
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
        let bytes = self.clone().bytes();
        let text = String::from_utf8_lossy(&bytes);

        if text.is_empty() {
            return write!(f, "No Content");
        }

        write!(f, "{text}")
    }
}

impl LinearNodeContents for TokenNode {
    fn bytes(self) -> Vec<u8> {
        self.content
            .into_iter()
            .flat_map(|token| token.content)
            .collect()
    }
    fn annotations(&self) -> impl Iterator<Item = ContentAnnotation> {
        let mut index = 0;

        self.content.iter().map(move |token| {
            let range = Range {
                start: index,
                end: index + token.content.len(),
            };
            index = range.end;

            ContentAnnotation {
                range,
                probability: Some(token.probability),
            }
        })
    }
    fn split(self, index: usize) -> Option<[Self; 2]> {
        let annotations = self.annotations();

        let mut split = None;

        for (location, annotation) in annotations.enumerate() {
            if annotation.range.contains(&index) {
                split = Some((location, annotation));
                break;
            }
        }

        let split = split?;

        let mut left = self.content;
        let right = left.split_off(split.0);

        todo!()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct NodeToken {
    pub probability: Decimal,
    pub content: Vec<u8>,
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
        let bytes = self.clone().bytes();
        let text = String::from_utf8_lossy(&bytes);

        if text.is_empty() {
            return write!(f, "No Content");
        }

        write!(f, "{text}")
    }
}

impl LinearNodeContents for TextTokenNode {
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
    fn annotations(&self) -> impl Iterator<Item = ContentAnnotation> {
        let mut index = 0;

        self.content.iter().flat_map(
            move |segment| -> Box<dyn Iterator<Item = ContentAnnotation>> {
                match segment {
                    TextOrToken::Text(text) => {
                        let range = Range {
                            start: index,
                            end: index + text.len(),
                        };
                        index = range.end;

                        Box::new(iter::once(ContentAnnotation {
                            range,
                            probability: None,
                        }))
                    }
                    TextOrToken::Bytes(bytes) => {
                        let range = Range {
                            start: index,
                            end: index + bytes.len(),
                        };
                        index = range.end;

                        Box::new(iter::once(ContentAnnotation {
                            range,
                            probability: None,
                        }))
                    }
                    TextOrToken::Token(tokens) => Box::new(tokens.iter().map(move |token| {
                        let range = Range {
                            start: index,
                            end: index + token.content.len(),
                        };
                        index = range.end;

                        ContentAnnotation {
                            range,
                            probability: Some(token.probability),
                        }
                    })),
                }
            },
        )
    }
    fn split(self, index: usize) -> Option<[Self; 2]> {
        todo!()
    }
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
    pub fn apply_annotations(&self, annotations: &mut Vec<ContentAnnotation>) {
        for modification in &self.content {
            modification.apply_annotations(annotations);
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
    pub fn apply_annotations(&self, annotations: &mut Vec<ContentAnnotation>) {
        todo!()
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
