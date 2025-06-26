//! Interactive representations of Weave contents.

#![allow(missing_docs)]

use std::{collections::HashSet, fmt::Display, iter, ops::Range, vec};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use similar::{DiffTag, capture_diff_slices};
use ulid::Ulid;

use super::{Weave, WeaveView};

/* TODO:
- Node splitting/merging
- Timeline content building
- Weave content updating
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

// TODO
impl<'w> WeaveTimeline<'w> {
    pub fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        for (node, _model) in &self.timeline {
            match node.content.clone() {
                NodeContent::Snippet(snippet) => {
                    bytes.append(&mut snippet.bytes());
                }
                NodeContent::Tokens(tokens) => {
                    bytes.append(&mut tokens.bytes());
                }
                NodeContent::Diff(diff) => {
                    diff.content.apply(&mut bytes);
                }
                NodeContent::Blank => {}
            }
        }

        bytes
    }
    pub fn annotated_string(&self) -> (String, Vec<TimelineAnnotation<'w>>) {
        todo!()
    }
    pub fn build_update(self, content: String) -> TimelineUpdate {
        todo!()
    }
}

pub struct TimelineUpdate {
    old: Vec<UpdateTimelineContent>,
    new: String,
}

pub struct UpdateTimelineContent {
    content: String,
    node: Ulid,
}

// TODO
impl Weave {
    pub fn split_node(&mut self, identifier: &Ulid, index: usize) -> Option<[Ulid; 2]> {
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
    pub fn update_content(&mut self, update: TimelineUpdate) {
        todo!()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Model {
    pub id: Ulid,
    pub label: String,
    pub style: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum NodeContent {
    Snippet(SnippetNode),
    Tokens(TokenNode),
    Diff(DiffNode),
    Blank,
}

impl NodeContent {
    #[allow(clippy::match_same_arms)]
    pub fn is_concatable(&self) -> bool {
        match self {
            Self::Snippet(_) => true,
            Self::Tokens(_) => true,
            Self::Diff(_) => false,
            Self::Blank => true,
        }
    }
    #[allow(clippy::missing_panics_doc)]
    pub fn merge(left: Self, right: Self) -> Option<Self> {
        if left.model() == right.model() || (left.is_concatable() && right.is_concatable()) {
            Some(
                match left {
                    Self::Snippet(mut left) => match right {
                        Self::Snippet(mut right) => {
                            left.content.append(&mut right.content);
                            Self::Snippet(left)
                        }
                        Self::Tokens(mut right) => {
                            let left_token = NodeToken {
                                probability: None,
                                content: left.content,
                            };
                            right.content.splice(..0, iter::once(left_token));

                            Self::Tokens(TokenNode {
                                content: right.content,
                                model: left.model,
                            })
                        }
                        Self::Diff(_) => panic!(),
                        Self::Blank => Self::Snippet(left),
                    },
                    Self::Tokens(mut left) => match right {
                        Self::Snippet(right) => {
                            left.content.push(NodeToken {
                                probability: None,
                                content: right.content,
                            });
                            Self::Tokens(left)
                        }
                        Self::Tokens(mut right) => {
                            left.content.append(&mut right.content);
                            Self::Tokens(left)
                        }
                        Self::Diff(_) => panic!(),
                        Self::Blank => Self::Tokens(left),
                    },
                    Self::Diff(_) => panic!(),
                    Self::Blank => right,
                }
                .reduce(),
            )
        } else {
            None
        }
    }
    pub fn is_mergeable(left: &Self, right: &Self) -> bool {
        left.model() == right.model() || (left.is_concatable() && right.is_concatable())
    }
    pub fn split(self, index: usize) -> Option<[Self; 2]> {
        match self {
            Self::Snippet(content) => content
                .split(index)
                .map(|[left, right]| [Self::Snippet(left), Self::Snippet(right)]),
            Self::Tokens(content) => content
                .split(index)
                .map(|[left, right]| [Self::Tokens(left), Self::Tokens(right)]),
            Self::Diff(_) => None,
            Self::Blank => Some([Self::Blank, Self::Blank]),
        }
        .map(|[left, right]| [left.reduce(), right.reduce()])
    }
    pub fn is_splitable(&self, index: usize) -> bool {
        match self {
            Self::Snippet(content) => index <= content.len(),
            Self::Tokens(content) => index <= content.len(),
            Self::Diff(_) => false,
            Self::Blank => true,
        }
    }
    fn reduce(self) -> Self {
        if self.model().is_none() && self.is_empty() {
            return Self::Blank;
        }

        match self {
            Self::Snippet(bytes) => Self::Snippet(bytes),
            Self::Tokens(mut tokens) => {
                if tokens.content.is_empty() {
                    Self::Blank
                } else if tokens.content.len() == 1 && tokens.content[0].probability.is_none() {
                    Self::Snippet(SnippetNode {
                        content: tokens.content.pop().unwrap().content,
                        model: tokens.model,
                    })
                } else {
                    Self::Tokens(tokens)
                }
            }
            Self::Diff(diff) => Self::Diff(diff),
            Self::Blank => Self::Blank,
        }
    }
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Snippet(content) => content.is_empty(),
            Self::Tokens(content) => content.is_empty(),
            Self::Diff(diff) => diff.content.is_empty(),
            Self::Blank => true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct ContentAnnotation {
    pub range: Range<usize>,
    pub probability: Option<Decimal>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct TimelineAnnotation<'w> {
    pub range: Range<usize>,
    pub probability: Option<Decimal>,

    pub node: Option<&'w Node>,
    pub model: Option<&'w Model>,
}

pub trait Annotation: Sized + From<Range<usize>> {
    fn range(&self) -> &Range<usize>;
    fn range_mut(&mut self) -> &mut Range<usize>;
    fn split(&self, index: usize) -> Option<[Self; 2]>;
}

impl Annotation for ContentAnnotation {
    fn range(&self) -> &Range<usize> {
        &self.range
    }
    fn range_mut(&mut self) -> &mut Range<usize> {
        &mut self.range
    }
    fn split(&self, index: usize) -> Option<[Self; 2]> {
        if index == 0 || index >= self.range.end {
            return None;
        }

        let mut left = self.range.clone();
        let mut right = self.range.clone();

        left.end -= index;
        right.start += index;

        Some([
            Self {
                range: left,
                probability: self.probability,
            },
            Self {
                range: right,
                probability: self.probability,
            },
        ])
    }
}

impl Annotation for TimelineAnnotation<'_> {
    fn range(&self) -> &Range<usize> {
        &self.range
    }
    fn range_mut(&mut self) -> &mut Range<usize> {
        &mut self.range
    }
    fn split(&self, index: usize) -> Option<[Self; 2]> {
        if index == 0 || index >= self.range.end {
            return None;
        }

        let mut left = self.range.clone();
        let mut right = self.range.clone();

        left.end -= index;
        right.start += index;

        Some([
            Self {
                range: left,
                probability: self.probability,
                node: self.node,
                model: self.model,
            },
            Self {
                range: right,
                probability: self.probability,
                node: self.node,
                model: self.model,
            },
        ])
    }
}

impl From<Range<usize>> for ContentAnnotation {
    fn from(range: Range<usize>) -> Self {
        Self {
            range,
            probability: None,
        }
    }
}

impl From<Range<usize>> for TimelineAnnotation<'_> {
    fn from(range: Range<usize>) -> Self {
        Self {
            range,
            probability: None,
            node: None,
            model: None,
        }
    }
}

impl From<ContentAnnotation> for TimelineAnnotation<'_> {
    fn from(input: ContentAnnotation) -> Self {
        Self {
            range: input.range,
            probability: input.probability,
            node: None,
            model: None,
        }
    }
}

pub trait NodeContents: Display + Sized {
    fn model(&self) -> Option<&NodeModel>;
}

pub trait ConcatableNodeContents: NodeContents {
    fn bytes(self) -> Vec<u8>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn annotations(&self) -> impl Iterator<Item = ContentAnnotation>;
    fn split(self, index: usize) -> Option<[Self; 2]>;
}

impl NodeContents for NodeContent {
    fn model(&self) -> Option<&NodeModel> {
        match self {
            Self::Snippet(content) => content.model(),
            Self::Tokens(content) => content.model(),
            Self::Diff(content) => content.model(),
            Self::Blank => None,
        }
    }
}

impl Display for NodeContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Snippet(content) => write!(f, "{content}"),
            Self::Tokens(content) => write!(f, "{content}"),
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
pub struct SnippetNode {
    pub content: Vec<u8>,
    pub model: Option<NodeModel>,
}

impl NodeContents for SnippetNode {
    fn model(&self) -> Option<&NodeModel> {
        self.model.as_ref()
    }
}

impl Display for SnippetNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.content.is_empty() {
            return write!(f, "No Content");
        }

        for chunk in self.content.utf8_chunks() {
            write!(f, "{}", chunk.valid())?;

            for &b in chunk.invalid() {
                write!(f, "\\x{b:02X}")?;
            }
        }

        Ok(())
    }
}

impl ConcatableNodeContents for SnippetNode {
    fn bytes(self) -> Vec<u8> {
        self.content
    }
    fn len(&self) -> usize {
        self.content.len()
    }
    fn is_empty(&self) -> bool {
        self.content.is_empty()
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
        left.shrink_to_fit();

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

        if bytes.is_empty() {
            return write!(f, "No Content");
        }

        for chunk in bytes.utf8_chunks() {
            write!(f, "{}", chunk.valid())?;

            for &b in chunk.invalid() {
                write!(f, "\\x{b:02X}")?;
            }
        }

        Ok(())
    }
}

impl ConcatableNodeContents for TokenNode {
    fn bytes(self) -> Vec<u8> {
        self.content
            .into_iter()
            .flat_map(|token| token.content)
            .collect()
    }
    fn len(&self) -> usize {
        let mut len = 0;

        for token in &self.content {
            len += token.content.len();
        }

        len
    }
    fn is_empty(&self) -> bool {
        for token in &self.content {
            if !token.content.is_empty() {
                return false;
            }
        }
        true
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
                probability: token.probability,
            }
        })
    }
    fn split(self, index: usize) -> Option<[Self; 2]> {
        let mut content_index = 0;

        let location = self
            .content
            .iter()
            .enumerate()
            .find_map(move |(location, token)| {
                let range = Range {
                    start: content_index,
                    end: content_index + token.content.len(),
                };
                if range.contains(&index) {
                    return Some(location);
                }

                content_index = range.end;
                None
            });

        if location.is_none() && index == content_index {
            return Some([
                Self {
                    content: self.content,
                    model: self.model.clone(),
                },
                Self {
                    content: vec![],
                    model: self.model,
                },
            ]);
        }

        let location = location?;

        let mut left = self.content;
        let mut right = left.split_off(location);
        left.shrink_to_fit();

        let [left_token, right_token] = right[0].clone().split(index - content_index)?;
        if !left_token.content.is_empty() {
            left.push(left_token);
        }
        right[0] = right_token;

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

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct NodeToken {
    pub probability: Option<Decimal>,
    pub content: Vec<u8>,
}

impl NodeToken {
    pub fn split(self, index: usize) -> Option<[Self; 2]> {
        if index > self.content.len() {
            return None;
        }

        let mut left = self.content;
        let right = left.split_off(index);
        left.shrink_to_fit();

        Some([
            Self {
                content: left,
                probability: self.probability,
            },
            Self {
                content: right,
                probability: self.probability,
            },
        ])
    }
}

impl From<SnippetNode> for TokenNode {
    fn from(input: SnippetNode) -> Self {
        Self {
            content: vec![NodeToken {
                content: input.content,
                probability: None,
            }],
            model: input.model,
        }
    }
}

impl From<Vec<u8>> for NodeToken {
    fn from(input: Vec<u8>) -> Self {
        Self {
            content: input,
            probability: None,
        }
    }
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
                    for chunk in text.utf8_chunks() {
                        write!(f, "{}", chunk.valid())?;

                        for &b in chunk.invalid() {
                            write!(f, "\\x{b:02X}")?;
                        }
                    }
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
    pub fn new(before: &[u8], after: &[u8]) -> Self {
        let chunks = capture_diff_slices(similar::Algorithm::Patience, before, after);

        let mut modifications = Vec::with_capacity(chunks.len());

        for (tag, before_range, after_range) in chunks.iter().map(similar::DiffOp::as_tag_tuple) {
            match tag {
                DiffTag::Equal => {}
                DiffTag::Insert => {
                    modifications.push(Modification {
                        index: before_range.start,
                        content: ModificationContent::Insertion(
                            after[after_range.start..after_range.end].to_vec(),
                        ),
                    });
                }
                DiffTag::Delete => {
                    modifications.push(Modification {
                        index: before_range.start,
                        content: ModificationContent::Deletion(before_range.end),
                    });
                }
                DiffTag::Replace => {
                    modifications.push(Modification {
                        index: before_range.start,
                        content: ModificationContent::Deletion(before_range.end),
                    });
                    modifications.push(Modification {
                        index: before_range.start,
                        content: ModificationContent::Insertion(
                            after[after_range.start..after_range.end].to_vec(),
                        ),
                    });
                }
            }
        }

        Self {
            content: modifications,
        }
    }
    pub fn apply(self, data: &mut Vec<u8>) {
        for modification in self.content {
            modification.apply(data);
        }
    }
    fn apply_annotations<T>(&self, annotations: &mut Vec<T>)
    where
        T: Annotation,
    {
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
    pub fn is_empty(&self) -> bool {
        for modification in &self.content {
            if !modification.content.is_empty() {
                return false;
            }
        }
        true
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct Modification {
    pub index: usize,
    pub content: ModificationContent,
}

impl Modification {
    pub fn apply(self, data: &mut Vec<u8>) {
        match self.content {
            ModificationContent::Insertion(content) => data.splice(self.index..self.index, content),
            ModificationContent::Deletion(length) => data.splice(self.index..length, vec![]),
        };
    }
    fn apply_annotations<T>(&self, annotations: &mut Vec<T>)
    where
        T: Annotation,
    {
        let offset = self.content.len();
        let range = Range {
            start: self.index,
            end: self.index + offset,
        };
        let Some(selected) = annotations
            .iter()
            .enumerate()
            .find_map(|(location, annotation)| {
                let annotation = annotation.range();

                if range.contains(&annotation.start) || range.contains(&annotation.end) {
                    return Some(location);
                }

                None
            })
        else {
            return;
        };

        match &self.content {
            ModificationContent::Insertion(_) => {
                if let Some([left, mut right]) = annotations[selected].split(self.index) {
                    let middle = T::from(range);
                    right.range_mut().start += offset;
                    right.range_mut().end += offset;

                    annotations.splice(selected..=selected, vec![left, middle, right]);
                }
                if annotations.len() > selected {
                    for annotation in &mut annotations[selected + 1..] {
                        let annotation = annotation.range_mut();
                        annotation.start += offset;
                        annotation.end += offset;
                    }
                }
            }
            ModificationContent::Deletion(_) => {
                let mut remove = Vec::with_capacity(annotations.len());

                for (index, annotation) in &mut annotations[selected..].iter_mut().enumerate() {
                    let annotation = annotation.range_mut();

                    if annotation.contains(&range.start) && annotation.contains(&range.end) {
                        remove.push(index);
                    } else if annotation.contains(&range.start) {
                        annotation.start = range.end;
                    } else if annotation.contains(&range.end) {
                        annotation.end = range.start;
                    } else {
                        annotation.start -= offset;
                        annotation.end -= offset;
                    }
                }
                if !remove.is_empty() {
                    annotations.splice(remove[0]..=remove[remove.len() - 1], vec![]);
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub enum ModificationContent {
    Insertion(Vec<u8>),
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
