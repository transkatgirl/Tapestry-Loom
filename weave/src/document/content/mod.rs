//! Interactive representations of Weave contents.

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    iter,
    ops::{Range, RangeInclusive},
    vec,
};

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use similar::Instant;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::time::Instant;

use serde::{Deserialize, Serialize};
use similar::{Algorithm, DiffTag, capture_diff_slices_deadline};
use ulid::Ulid;

#[allow(unused_imports)]
use super::{Weave, WeaveView};

#[cfg(test)]
mod tests;

const EMPTY_MESSAGE: &str = "No Content";

/// A unit of content in a [`Weave`].
///
/// Nodes act as containers for [`NodeContent`] objects, allowing them to be connected together.
///
/// Nodes have a directional relationship, with nodes further in the chain being later in the timeline. Nodes can be active or inactive, and can be bookmarked by the user for later reference.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Node {
    /// The unique identifier of the node.
    pub id: Ulid,
    /// The parents of the node.
    pub from: HashSet<Ulid>,
    /// The children of the node.
    pub to: HashSet<Ulid>,
    /// If the node is active or inactive.
    pub active: bool,
    /// If the node is bookmarked.
    pub bookmarked: bool,
    /// The content of the node.
    pub content: NodeContent,
}

/// An ordered list of connected active nodes.
///
/// WeaveTimeline objects are created by the [`WeaveView::get_active_timelines`] function. Each timeline represents one possible linear progression of nodes (and their associated models), starting at the root of the [`Weave`] and progressing outwards.
#[derive(Serialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct WeaveTimeline<'w> {
    #[allow(missing_docs)]
    pub(super) timeline: Vec<(&'w Node, Option<&'w Model>)>,
}

// Trivial; shouldn't require unit tests
impl<'w> AsRef<Vec<(&'w Node, Option<&'w Model>)>> for WeaveTimeline<'w> {
    fn as_ref(&self) -> &Vec<(&'w Node, Option<&'w Model>)> {
        &self.timeline
    }
}

impl<'w> WeaveTimeline<'w> {
    /// Converts the timeline into it's inner [`Vec`] object.
    // Trivial; shouldn't require unit tests
    #[must_use]
    pub fn into_vec(self) -> Vec<(&'w Node, Option<&'w Model>)> {
        self.timeline
    }
    /// Returns the output of the timeline as a set of bytes.
    // Trivial; shouldn't require unit tests
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        for (node, _model) in &self.timeline {
            match node.content.clone() {
                NodeContent::Snippet(snippet) => {
                    bytes.append(&mut snippet.into_bytes());
                }
                NodeContent::Tokens(tokens) => {
                    bytes.append(&mut tokens.into_bytes());
                }
                NodeContent::Diff(diff) => {
                    diff.content.apply(&mut bytes);
                }
                NodeContent::Blank => {}
            }
        }

        bytes
    }
    /// Returns the output of the timeline as an annotated string.
    ///
    /// Bytes which are invalid UTF-8 will be replaced by the character U+001A, keeping the length the same as the original set of bytes.
    #[must_use]
    pub fn annotated_string(&self) -> (String, Vec<TimelineAnnotation<'w>>) {
        let mut bytes = Vec::new();
        let mut annotations = Vec::with_capacity(self.timeline.len());

        for (node, model) in &self.timeline {
            match &node.content {
                NodeContent::Snippet(snippet) => {
                    annotations.append(
                        &mut snippet
                            .annotations()
                            .map(|annotation| TimelineAnnotation {
                                range: Range {
                                    start: annotation.range.start + bytes.len(),
                                    end: annotation.range.end + bytes.len(),
                                },
                                node: Some(node),
                                model: *model,
                                subsection_metadata: annotation.metadata,
                                content_metadata: node.content.metadata(),
                                parameters: node.content.model().map(|model| &model.parameters),
                            })
                            .collect(),
                    );
                    bytes.append(&mut snippet.clone().into_bytes());
                }
                NodeContent::Tokens(tokens) => {
                    annotations.append(
                        &mut tokens
                            .annotations()
                            .map(|annotation| TimelineAnnotation {
                                range: Range {
                                    start: annotation.range.start + bytes.len(),
                                    end: annotation.range.end + bytes.len(),
                                },
                                node: Some(node),
                                model: *model,
                                subsection_metadata: annotation.metadata,
                                content_metadata: node.content.metadata(),
                                parameters: node.content.model().map(|model| &model.parameters),
                            })
                            .collect(),
                    );
                    bytes.append(&mut tokens.clone().into_bytes());
                }
                NodeContent::Diff(diff) => {
                    diff.content.apply_timeline_annotations(
                        node,
                        *model,
                        node.content.metadata(),
                        &mut annotations,
                    );
                    diff.content.clone().apply(&mut bytes);
                }
                NodeContent::Blank => {}
            }
        }

        let mut string = String::with_capacity(bytes.len());

        for chunk in bytes.utf8_chunks() {
            string.push_str(chunk.valid());

            for &_b in chunk.invalid() {
                string.push(''); // Legacy substitution character is used due to it being a single byte in length.
            }
        }

        (string, annotations)
    }
    // Trivial; shouldn't require unit tests
    pub(super) fn ranged_string(self) -> (String, Vec<TimelineNodeRange>) {
        let (content, annotations) = self.annotated_string();
        (
            content,
            annotations
                .into_iter()
                .map(|annotation| TimelineNodeRange {
                    range: annotation.range,
                    node: annotation.node.map(|node| node.id),
                })
                .collect(),
        )
    }
    // Trivial; shouldn't require unit tests
    pub(super) fn build_update(
        self,
        content: String,
        metadata: Option<HashMap<String, String>>,
        deadline: Instant,
    ) -> TimelineUpdate {
        let (before, ranges) = self.ranged_string();

        TimelineUpdate {
            ranges,
            diff: Diff::new(&before.into_bytes(), &content.into_bytes(), deadline),
            metadata,
        }
    }
}

pub(super) struct TimelineUpdate {
    pub(super) ranges: Vec<TimelineNodeRange>,
    pub(super) diff: Diff,
    pub(super) metadata: Option<HashMap<String, String>>,
}

pub(super) struct TimelineNodeRange {
    pub(super) range: Range<usize>,
    pub(super) node: Option<Ulid>,
}

// Trivial; shouldn't require unit tests
impl From<Range<usize>> for TimelineNodeRange {
    fn from(input: Range<usize>) -> Self {
        Self {
            range: input,
            node: None,
        }
    }
}

impl Annotation for TimelineNodeRange {
    // Trivial; shouldn't require unit tests
    #[inline]
    fn range(&self) -> &Range<usize> {
        &self.range
    }
    // Trivial; shouldn't require unit tests
    #[inline]
    fn range_mut(&mut self) -> &mut Range<usize> {
        &mut self.range
    }
    // Copied from ContentAnnotation's implementation of split(); shouldn't require additional unit tests
    fn split(&self, index: usize) -> Option<(Self, Self)> {
        if index == 0 || (self.range.start + index) >= self.range.end {
            return None;
        }

        let mut left = self.range.clone();
        let mut right = self.range.clone();

        left.end = left.start + index;
        right.start += index;

        Some((
            Self {
                range: left,
                node: self.node,
            },
            Self {
                range: right,
                node: self.node,
            },
        ))
    }
}

/// A user-facing label for algorithmic generators of [`NodeContent`] objects.
///
/// NodeContent objects should always be associated with the Model that generated them.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Model {
    /// The unique identifier of the model.
    pub id: Ulid,
    /// The user facing label for the model.
    pub label: String,
    /// Additional metadata associated with the model.
    pub metadata: HashMap<String, String>,
}

/// Isolated sections of content within a [`Weave`] document.
///
/// These sections typically have little meaning on their own, as they are meant to be assembled into a bigger whole.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum NodeContent {
    /// A snippet of text.
    Snippet(SnippetContent),
    /// A snippet of tokenized text.
    Tokens(TokenContent),
    /// A list of modifications to perform on the current text.
    Diff(DiffContent),
    /// An empty object with no content.
    Blank,
}

impl NodeContent {
    /// Returns `true` if the content is concatable.
    // Trivial; shouldn't require unit tests
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub fn is_concatable(&self) -> bool {
        match self {
            Self::Snippet(_) => true,
            Self::Tokens(_) => true,
            Self::Diff(_) => false,
            Self::Blank => true,
        }
    }
    /// Merges two sections of content together.
    ///
    /// This requires both sections to be concatable and contain the same metadata.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn merge(left: Self, right: Self) -> Option<Self> {
        if !NodeContent::is_mergeable(&left, &right) {
            return None;
        }

        Some(
            match left {
                Self::Snippet(mut left) => match right {
                    Self::Snippet(mut right) => {
                        left.content.append(&mut right.content);
                        Self::Snippet(left)
                    }
                    Self::Tokens(mut right) => {
                        let left_token = ContentToken {
                            metadata: None,
                            content: left.content,
                        };
                        right.content.splice(..0, iter::once(left_token));

                        Self::Tokens(TokenContent {
                            content: right.content,
                            model: left.model,
                            metadata: left.metadata,
                        })
                    }
                    Self::Diff(_) => panic!(),
                    Self::Blank => Self::Snippet(left),
                },
                Self::Tokens(mut left) => match right {
                    Self::Snippet(right) => {
                        left.content.push(ContentToken {
                            metadata: None,
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
                Self::Blank => match right {
                    Self::Snippet(right) => Self::Snippet(right),
                    Self::Tokens(right) => Self::Tokens(right),
                    Self::Diff(_) => panic!(),
                    Self::Blank => Self::Blank,
                },
            }
            .reduce(),
        )
    }
    /// Returns `true` if the two sections of content can be merged together.
    // Trivial; shouldn't require unit tests
    #[must_use]
    pub fn is_mergeable(left: &Self, right: &Self) -> bool {
        left.model() == right.model()
            && left.metadata() == right.metadata()
            && (left.is_concatable() && right.is_concatable())
    }
    /// Splits the content in half at the specified index, retaining all associated metadata.
    ///
    /// Some types of content cannot be split in half and will always return [`None`] regardless of the index specified.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn split(self, index: usize) -> Option<(Self, Self)> {
        if !self.is_splitable(index) {
            return None;
        }

        match self {
            Self::Snippet(content) => content
                .split(index)
                .map(|(left, right)| (Self::Snippet(left), Self::Snippet(right))),
            Self::Tokens(content) => content
                .split(index)
                .map(|(left, right)| (Self::Tokens(left), Self::Tokens(right))),
            Self::Diff(_) => panic!(),
            Self::Blank => Some((Self::Blank, Self::Blank)),
        }
        .map(|(left, right)| (left.reduce(), right.reduce()))
    }
    /// Returns `true` if the content can be split.
    // Trivial; shouldn't require unit tests
    #[must_use]
    pub fn is_splitable(&self, index: usize) -> bool {
        match self {
            Self::Snippet(content) => index <= content.len(),
            Self::Tokens(content) => index <= content.len(),
            Self::Diff(_) => false,
            Self::Blank => index == 0,
        }
    }
    /// Converts the content to the simplest variant that can contain it without losing information.
    ///
    /// This function is automatically applied when using [`NodeContent::split`] or [`NodeContent::merge`].
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn reduce(self) -> Self {
        if !self.has_metadata() && self.is_empty() {
            return Self::Blank;
        }

        match self {
            Self::Snippet(bytes) => Self::Snippet(bytes),
            Self::Tokens(mut tokens) => {
                tokens.content.retain(|token| {
                    !token.content.is_empty()
                        || !token.metadata.as_ref().is_none_or(HashMap::is_empty)
                });

                if tokens.content.is_empty() {
                    Self::Snippet(SnippetContent {
                        content: vec![],
                        model: tokens.model,
                        metadata: tokens.metadata,
                    })
                } else if tokens.content.len() == 1
                    && tokens.content[0]
                        .metadata
                        .as_ref()
                        .is_none_or(HashMap::is_empty)
                {
                    Self::Snippet(SnippetContent {
                        content: tokens.content.pop().unwrap().content,
                        model: tokens.model,
                        metadata: tokens.metadata,
                    })
                } else {
                    Self::Tokens(tokens)
                }
            }
            Self::Diff(diff) => Self::Diff(diff),
            Self::Blank => Self::Blank,
        }
    }
    /// Returns `true` if the content is empty (excluding metadata).
    // Trivial; shouldn't require unit tests
    #[must_use]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Snippet(content) => content.is_empty(),
            Self::Tokens(content) => content.is_empty(),
            Self::Diff(diff) => diff.content.is_empty(),
            Self::Blank => true,
        }
    }
    #[must_use]
    pub(super) fn into_diff(self, range: Range<usize>) -> Option<Self> {
        let range_len = range.end - range.start;

        match self {
            Self::Snippet(content) => {
                let bytes = content.content;

                let modifications = if range_len == 0 {
                    vec![Modification {
                        index: range.start,
                        content: ModificationContent::Insertion(bytes),
                    }]
                } else {
                    vec![
                        Modification {
                            index: range.start,
                            content: ModificationContent::Deletion(range_len),
                        },
                        Modification {
                            index: range.start,
                            content: ModificationContent::Insertion(bytes),
                        },
                    ]
                };

                Some(NodeContent::Diff(DiffContent {
                    content: Diff {
                        content: modifications,
                    },
                    model: content.model,
                    metadata: content.metadata,
                }))
            }
            Self::Tokens(content) => {
                let modifications = if range_len == 0 {
                    vec![Modification {
                        index: range.start,
                        content: ModificationContent::TokenInsertion(content.content),
                    }]
                } else {
                    vec![
                        Modification {
                            index: range.start,
                            content: ModificationContent::Deletion(range_len),
                        },
                        Modification {
                            index: range.start,
                            content: ModificationContent::TokenInsertion(content.content),
                        },
                    ]
                };

                Some(NodeContent::Diff(DiffContent {
                    content: Diff {
                        content: modifications,
                    },
                    model: content.model,
                    metadata: content.metadata,
                }))
            }
            Self::Diff(_) | Self::Blank => None,
        }
    }
    // Trivial; shouldn't require unit tests
    pub(super) fn merge_metadata(&mut self, metadata: HashMap<String, String>) {
        match self {
            NodeContent::Snippet(content) => {
                if let Some(existing) = content.metadata.as_mut() {
                    existing.extend(metadata);
                } else {
                    content.metadata = Some(metadata);
                }
            }
            NodeContent::Tokens(content) => {
                if let Some(existing) = content.metadata.as_mut() {
                    existing.extend(metadata);
                } else {
                    content.metadata = Some(metadata);
                }
            }
            NodeContent::Diff(content) => {
                if let Some(existing) = content.metadata.as_mut() {
                    existing.extend(metadata);
                } else {
                    content.metadata = Some(metadata);
                }
            }
            NodeContent::Blank => {}
        }
    }
}

/// An annotation within a section of content.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ContentAnnotation<'w> {
    /// The range of content bytes that the annotation applies to.
    pub range: Range<usize>,

    /// Metadata associated with this set of bytes.
    ///
    /// This field should be used only for metadata associated with a subsection (such as a token), not metadata regarding the entirety of the section.
    pub metadata: Option<&'w HashMap<String, String>>,
}

/// An annotation within the output of a [`WeaveTimeline`].
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct TimelineAnnotation<'w> {
    /// The range of content bytes that the annotation applies to.
    pub range: Range<usize>,

    /// The node that this set of bytes originates from.
    pub node: Option<&'w Node>,
    /// The [`Model`] which generated this set of bytes, if any.
    pub model: Option<&'w Model>,
    /// The parameters used to algorithmically generate this set of bytes, if any.
    pub parameters: Option<&'w Vec<(String, String)>>,
    /// Metadata associated with this set of bytes.
    ///
    /// This field should be used only for metadata associated with a subsection (such as a token). Metadata regarding the entirety of the section should be in `content_metadata`.
    pub subsection_metadata: Option<&'w HashMap<String, String>>,
    /// Metadata associated with the content this set of bytes originated from.
    pub content_metadata: Option<&'w HashMap<String, String>>,
}

/// Types which act as content annotations for sets of bytes.
pub trait Annotation: Sized + From<Range<usize>> {
    /// Returns the range of content bytes that the annotation applies to.
    #[must_use]
    fn range(&self) -> &Range<usize>;
    /// Returns a mutable reference to the annotation's byte range.
    #[must_use]
    fn range_mut(&mut self) -> &mut Range<usize>;
    /// Splits the annotation in half at the specified index, retaining all associated metadata.
    #[must_use]
    fn split(&self, index: usize) -> Option<(Self, Self)>;
}

impl Annotation for ContentAnnotation<'_> {
    // Trivial; shouldn't require unit tests
    #[inline]
    fn range(&self) -> &Range<usize> {
        &self.range
    }
    // Trivial; shouldn't require unit tests
    #[inline]
    fn range_mut(&mut self) -> &mut Range<usize> {
        &mut self.range
    }
    fn split(&self, index: usize) -> Option<(Self, Self)> {
        if index == 0 || (self.range.start + index) >= self.range.end {
            return None;
        }

        let mut left = self.range.clone();
        let mut right = self.range.clone();

        left.end = left.start + index;
        right.start += index;

        Some((
            Self {
                range: left,
                metadata: self.metadata,
            },
            Self {
                range: right,
                metadata: self.metadata,
            },
        ))
    }
}

impl Annotation for TimelineAnnotation<'_> {
    // Trivial; shouldn't require unit tests
    #[inline]
    fn range(&self) -> &Range<usize> {
        &self.range
    }
    // Trivial; shouldn't require unit tests
    #[inline]
    fn range_mut(&mut self) -> &mut Range<usize> {
        &mut self.range
    }
    // Copied from ContentAnnotation's implementation of split(); shouldn't require additional unit tests
    fn split(&self, index: usize) -> Option<(Self, Self)> {
        if index == 0 || (self.range.start + index) >= self.range.end {
            return None;
        }

        let mut left = self.range.clone();
        let mut right = self.range.clone();

        left.end = left.start + index;
        right.start += index;

        Some((
            Self {
                range: left,
                node: self.node,
                model: self.model,
                subsection_metadata: self.subsection_metadata,
                content_metadata: self.content_metadata,
                parameters: self.parameters,
            },
            Self {
                range: right,
                node: self.node,
                model: self.model,
                subsection_metadata: self.subsection_metadata,
                content_metadata: self.content_metadata,
                parameters: self.parameters,
            },
        ))
    }
}

// Trivial; shouldn't require unit tests
impl From<Range<usize>> for ContentAnnotation<'_> {
    fn from(range: Range<usize>) -> Self {
        Self {
            range,
            metadata: None,
        }
    }
}

// Trivial; shouldn't require unit tests
impl From<Range<usize>> for TimelineAnnotation<'_> {
    fn from(range: Range<usize>) -> Self {
        Self {
            range,
            node: None,
            model: None,
            subsection_metadata: None,
            content_metadata: None,
            parameters: None,
        }
    }
}

// Trivial; shouldn't require unit tests
impl<'w> From<ContentAnnotation<'w>> for TimelineAnnotation<'w> {
    fn from(input: ContentAnnotation<'w>) -> Self {
        Self {
            range: input.range,
            node: None,
            model: None,
            subsection_metadata: input.metadata,
            content_metadata: None,
            parameters: None,
        }
    }
}

/// Types which are intended to be used as content for a [`Node`] object.
pub trait NodeContents: Display + Sized {
    /// Returns metadata about the algorithmic process which generated the content, if any.
    #[must_use]
    fn model(&self) -> Option<&ContentModel>;
    /// Returns metadata associated with the content.
    #[must_use]
    fn metadata(&self) -> Option<&HashMap<String, String>>;
    /// Returns if the content has any metadata (including an associated [`ContentModel`]).
    ///
    /// This will return `true` if the object has metadata associated with part of the content but not all of it.
    // Trivial; shouldn't require unit tests
    #[must_use]
    #[inline]
    fn has_metadata(&self) -> bool {
        self.model().is_some() || !self.metadata().is_none_or(HashMap::is_empty)
    }
}

/// Concatable types which are intended to be used as content for a [`Node`] object.
pub trait ConcatableNodeContents: NodeContents {
    /// Converts the content into a set of bytes.
    #[must_use]
    fn into_bytes(self) -> Vec<u8>;
    /// Returns the length from the content in bytes.
    #[must_use]
    fn len(&self) -> usize;
    /// Returns `true` if the content has a length of zero bytes (excluding metadata).
    #[must_use]
    fn is_empty(&self) -> bool;
    /// Returns annotations for the content.
    #[must_use]
    fn annotations(&self) -> impl Iterator<Item = ContentAnnotation>;
    /// Splits the content in half at the specified index.
    ///
    /// If the content has metadata, both sides of the split retain that metadata.
    #[must_use]
    fn split(self, index: usize) -> Option<(Self, Self)>;
}

// Trivial; shouldn't require unit tests
impl NodeContents for NodeContent {
    fn model(&self) -> Option<&ContentModel> {
        match self {
            Self::Snippet(content) => content.model(),
            Self::Tokens(content) => content.model(),
            Self::Diff(content) => content.model(),
            Self::Blank => None,
        }
    }
    fn metadata(&self) -> Option<&HashMap<String, String>> {
        match self {
            Self::Snippet(content) => content.metadata(),
            Self::Tokens(content) => content.metadata(),
            Self::Diff(content) => content.metadata(),
            Self::Blank => None,
        }
    }
    fn has_metadata(&self) -> bool {
        match self {
            Self::Snippet(content) => content.has_metadata(),
            Self::Tokens(content) => content.has_metadata(),
            Self::Diff(diff) => diff.has_metadata(),
            Self::Blank => false,
        }
    }
}

// Trivial; shouldn't require unit tests
impl Display for NodeContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Snippet(content) => write!(f, "{content}"),
            Self::Tokens(content) => write!(f, "{content}"),
            Self::Diff(content) => write!(f, "{content}"),
            Self::Blank => write!(f, "{EMPTY_MESSAGE}"),
        }
    }
}

/// Metadata about the algorithmic process which generated a section of content.
///
/// This should only be used if the algorithmic process generated the content itself, not just the metadata associated with the content.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ContentModel {
    /// The identifier of the [`Model`] that generated the content.
    pub id: Ulid,
    /// The parameters used to generate the content.
    ///
    /// This should not be used to store general metadata about the [`Model`]; It should only contain the tunable parameters used for generation.
    pub parameters: Vec<(String, String)>,
}

/// A wrapper around a snippet of UTF-8 encoded text.
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct SnippetContent {
    /// The text being stored.
    ///
    /// This may not be valid UTF-8 on it's own, so it is stored as an array of bytes rather than a [`String`].
    pub content: Vec<u8>,
    /// Metadata about the algorithmic process which generated the snippet, if any.
    pub model: Option<ContentModel>,
    /// Metadata associated with the content.
    pub metadata: Option<HashMap<String, String>>,
}

// Trivial; shouldn't require unit tests
impl NodeContents for SnippetContent {
    #[inline]
    fn model(&self) -> Option<&ContentModel> {
        self.model.as_ref()
    }
    #[inline]
    fn metadata(&self) -> Option<&HashMap<String, String>> {
        self.metadata.as_ref()
    }
}

// Trivial; shouldn't require unit tests
impl Display for SnippetContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.content.is_empty() {
            return write!(f, "{EMPTY_MESSAGE}");
        }

        for chunk in self.content.utf8_chunks() {
            write!(f, "{}", chunk.valid())?;

            for &b in chunk.invalid() {
                write!(f, "\\x{b:02x}")?;
            }
        }

        Ok(())
    }
}

impl ConcatableNodeContents for SnippetContent {
    // Trivial; shouldn't require unit tests
    #[inline]
    fn into_bytes(self) -> Vec<u8> {
        self.content
    }
    // Trivial; shouldn't require unit tests
    #[inline]
    fn len(&self) -> usize {
        self.content.len()
    }
    // Trivial; shouldn't require unit tests
    #[inline]
    fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
    // Trivial; shouldn't require unit tests
    fn annotations(&self) -> impl Iterator<Item = ContentAnnotation> {
        iter::once(ContentAnnotation {
            metadata: None,
            range: Range {
                start: 0,
                end: self.content.len(),
            },
        })
    }
    fn split(self, index: usize) -> Option<(Self, Self)> {
        if index > self.content.len() {
            return None;
        }

        let mut left = self.content;
        let right = left.split_off(index);
        left.shrink_to_fit();

        Some((
            Self {
                content: left,
                model: self.model.clone(),
                metadata: self.metadata.clone(),
            },
            Self {
                content: right,
                model: self.model,
                metadata: self.metadata,
            },
        ))
    }
}

/// A container for tokenized UTF-8 encoded text.
///
/// Tokenization is a popular technique used by text generation algorithms to handle text in larger chunks than individual characters.
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct TokenContent {
    /// The tokens being stored, listed in the order they should be displayed.
    pub content: Vec<ContentToken>,
    /// Metadata about the algorithmic process which generated the tokens, if any.
    ///
    /// This should be left blank if the text was tokenized by the algorithm but not generated by it.
    pub model: Option<ContentModel>,
    /// Metadata associated with the content.
    pub metadata: Option<HashMap<String, String>>,
}

// Trivial; shouldn't require unit tests
impl NodeContents for TokenContent {
    #[inline]
    fn model(&self) -> Option<&ContentModel> {
        self.model.as_ref()
    }
    #[inline]
    fn metadata(&self) -> Option<&HashMap<String, String>> {
        self.metadata.as_ref()
    }
    fn has_metadata(&self) -> bool {
        if self.model.is_some() || !self.metadata().is_none_or(HashMap::is_empty) {
            return true;
        }

        self.content
            .iter()
            .any(|token| !token.metadata.as_ref().is_none_or(HashMap::is_empty))
    }
}

// Trivial; shouldn't require unit tests
impl Display for TokenContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bytes = self.clone().into_bytes();

        if bytes.is_empty() {
            return write!(f, "{EMPTY_MESSAGE}");
        }

        for chunk in bytes.utf8_chunks() {
            write!(f, "{}", chunk.valid())?;

            for &b in chunk.invalid() {
                write!(f, "\\x{b:02x}")?;
            }
        }

        Ok(())
    }
}

impl ConcatableNodeContents for TokenContent {
    // Trivial; shouldn't require unit tests
    fn into_bytes(self) -> Vec<u8> {
        self.content
            .into_iter()
            .flat_map(|token| token.content)
            .collect()
    }
    // Trivial; shouldn't require unit tests
    fn len(&self) -> usize {
        self.content.iter().map(|token| token.content.len()).sum()
    }
    // Trivial; shouldn't require unit tests
    fn is_empty(&self) -> bool {
        self.content.iter().all(|token| token.content.is_empty())
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
                metadata: token.metadata.as_ref(),
            }
        })
    }
    fn split(self, index: usize) -> Option<(Self, Self)> {
        let mut content_index = 0;

        let location = self
            .annotations()
            .enumerate()
            .find_map(|(location, annotation)| {
                if annotation.range.contains(&index) {
                    return Some(location);
                }
                content_index = annotation.range.end;

                None
            });

        match location {
            Some(location) => {
                let mut left = self.content;
                let mut right = left.split_off(location);
                left.shrink_to_fit();

                let (left_token, right_token) = right[0].clone().split(index - content_index)?;
                if !left_token.content.is_empty() {
                    left.push(left_token);
                }
                right[0] = right_token;

                Some((
                    Self {
                        content: left,
                        model: self.model.clone(),
                        metadata: self.metadata.clone(),
                    },
                    Self {
                        content: right,
                        model: self.model,
                        metadata: self.metadata,
                    },
                ))
            }
            None => {
                if index == content_index {
                    Some((
                        Self {
                            content: self.content,
                            model: self.model.clone(),
                            metadata: self.metadata.clone(),
                        },
                        Self {
                            content: vec![],
                            model: self.model,
                            metadata: self.metadata,
                        },
                    ))
                } else {
                    None
                }
            }
        }
    }
}

/// A single UTF-8 token from a tokenized piece of text.
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct ContentToken {
    /// The textual content of the token.
    ///
    /// This may not be valid UTF-8 on it's own, so it is stored as an array of bytes rather than a [`String`].
    pub content: Vec<u8>,
    /// Metadata associated with the token.
    pub metadata: Option<HashMap<String, String>>,
}

impl ContentToken {
    /// Splits the token in half at the specified index, retaining all associated metadata.
    // Copied from SnippetContent's implementation of split(); shouldn't require additional unit tests
    pub fn split(self, index: usize) -> Option<(Self, Self)> {
        if index > self.content.len() {
            return None;
        }

        let mut left = self.content;
        let right = left.split_off(index);
        left.shrink_to_fit();

        Some((
            Self {
                content: left,
                metadata: self.metadata.clone(),
            },
            Self {
                content: right,
                metadata: self.metadata,
            },
        ))
    }
}

// Trivial; shouldn't require unit tests
impl From<SnippetContent> for TokenContent {
    fn from(input: SnippetContent) -> Self {
        Self {
            content: vec![ContentToken {
                content: input.content,
                metadata: None,
            }],
            model: input.model,
            metadata: input.metadata,
        }
    }
}

// Trivial; shouldn't require unit tests
impl From<Vec<u8>> for ContentToken {
    fn from(content: Vec<u8>) -> Self {
        Self {
            content,
            metadata: None,
        }
    }
}

/// A container for a set of modifications to perform on the current text.
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DiffContent {
    /// The modification set being stored.
    pub content: Diff,
    /// Metadata about the algorithmic process which generated the modification set, if any.
    pub model: Option<ContentModel>,
    /// Metadata associated with the content.
    pub metadata: Option<HashMap<String, String>>,
}

// Trivial; shouldn't require unit tests
impl NodeContents for DiffContent {
    #[inline]
    fn model(&self) -> Option<&ContentModel> {
        self.model.as_ref()
    }
    #[inline]
    fn metadata(&self) -> Option<&HashMap<String, String>> {
        self.metadata.as_ref()
    }
    fn has_metadata(&self) -> bool {
        if self.model.is_some() || !self.metadata().is_none_or(HashMap::is_empty) {
            return true;
        }

        self.content.has_metadata()
    }
}

// Trivial; shouldn't require unit tests
impl Display for DiffContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self.content.count();

        if count.total == 0 {
            write!(f, "{EMPTY_MESSAGE}")
        } else {
            write!(f, "{count}")
        }
    }
}

/// A list of modifications to perform on a set of bytes.
///
/// This has little meaning on its own, as it is meant to be paired with the text being modified.
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct Diff {
    /// A list of modifications in the order they should be performed.
    pub content: Vec<Modification>,
}

impl Diff {
    /// Calculates a diff between two sets of bytes.
    ///
    /// The `deadline` option sets a constraint on the time spent calculating the diff. If the `deadline` is exceeded, the diff algorithm will aim to finish as soon as possible, returning a suboptimal diff.
    ///
    /// The specific algorithm used to calculate the diff is subject to change.
    #[must_use]
    pub fn new(before: &[u8], after: &[u8], deadline: Instant) -> Self {
        let chunks = capture_diff_slices_deadline(Algorithm::Myers, before, after, Some(deadline));

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
                        content: ModificationContent::Deletion(
                            before_range.end - before_range.start,
                        ),
                    });
                }
                DiffTag::Replace => {
                    modifications.push(Modification {
                        index: before_range.start,
                        content: ModificationContent::Deletion(
                            before_range.end - before_range.start,
                        ),
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

        modifications.shrink_to_fit();

        Self {
            content: modifications,
        }
    }
    /// Applies the diff to a set of bytes.
    ///
    /// # Panics
    /// Panics if the diff contains any modifications with bounds outside of the byte set.
    // Trivial; shouldn't require unit tests
    pub fn apply(self, data: &mut Vec<u8>) {
        for modification in self.content {
            modification.apply(data);
        }
    }
    // Trivial; shouldn't require unit tests
    pub(super) fn apply_timeline_annotations<'w>(
        &'w self,
        node: &'w Node,
        model: Option<&'w Model>,
        content_metadata: Option<&'w HashMap<String, String>>,
        annotations: &mut Vec<TimelineAnnotation<'w>>,
    ) {
        for modification in &self.content {
            let updates = modification.apply_annotations(annotations);

            if let Some(index) = updates.inserted_bytes {
                annotations[index].node = Some(node);
                annotations[index].model = model;
                annotations[index].parameters = node.content.model().map(|model| &model.parameters);
                annotations[index].content_metadata = content_metadata;
            }
            if let Some(indices) = updates.inserted_tokens {
                if let ModificationContent::TokenInsertion(content) = &modification.content {
                    for (modification_index, annotation_index) in indices.into_iter().enumerate() {
                        annotations[annotation_index].node = Some(node);
                        annotations[annotation_index].model = model;
                        annotations[annotation_index].parameters =
                            node.content.model().map(|model| &model.parameters);
                        annotations[annotation_index].subsection_metadata =
                            content[modification_index].metadata.as_ref();
                        annotations[annotation_index].content_metadata = content_metadata;
                    }
                } else {
                    panic!() // Should never happen
                }
            }
        }
    }
    /// Calculates the total number of non-empty modifications in the [`Diff`] by type.
    // Trivial; shouldn't require unit tests
    #[must_use]
    pub fn count(&self) -> ModificationCount {
        let mut insertions: usize = 0;
        let mut deletions: usize = 0;

        for modification in &self.content {
            if !modification.content.is_empty() {
                match modification.content {
                    ModificationContent::Insertion(_) | ModificationContent::TokenInsertion(_) => {
                        insertions += 1;
                    }
                    ModificationContent::Deletion(_) => deletions += 1,
                }
            }
        }

        ModificationCount {
            total: insertions + deletions,
            insertions,
            deletions,
        }
    }
    /// Returns `true` if any of the modifications within the [`Diff`] contain metadata.
    // Trivial; shouldn't require unit tests
    #[must_use]
    pub fn has_metadata(&self) -> bool {
        self.content.iter().any(Modification::has_metadata)
    }
    /// Returns `true` if the sum of all modification lengths in the [`Diff`] is equal to zero bytes.
    // Trivial; shouldn't require unit tests
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.content.iter().all(|token| token.content.is_empty())
    }
}

/// A modification to perform on a set of bytes.
///
/// This has little meaning on its own, as it is meant to be paired with the text being modified.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Modification {
    /// The index where the modification should be applied.
    pub index: usize,
    /// The content of the modification.
    pub content: ModificationContent,
}

impl Modification {
    /// Applies the modification to a set of bytes.
    ///
    /// # Panics
    /// Panics if the modification's bounds are outside of the byte set.
    pub fn apply(self, data: &mut Vec<u8>) {
        match self.content {
            ModificationContent::Insertion(content) => data.splice(self.index..self.index, content),
            ModificationContent::Deletion(length) => {
                data.splice(self.index..(self.index + length), vec![])
            }
            ModificationContent::TokenInsertion(content) => {
                let content: Vec<u8> = content
                    .into_iter()
                    .flat_map(|token| token.content)
                    .collect();
                data.splice(self.index..self.index, content)
            }
        };
    }
    /// Returns the range of bytes that the modification will be performed on.
    ///
    /// If this is an insertion modification, the end of the range represents the end of the inserted content.
    // Trivial; shouldn't require unit tests
    #[must_use]
    #[inline]
    pub fn range(&self) -> Range<usize> {
        Range {
            start: self.index,
            end: self.index + self.content.len(),
        }
    }
    /// Returns if the modification contains any metadata.
    // Trivial; shouldn't require unit tests
    #[must_use]
    pub fn has_metadata(&self) -> bool {
        match &self.content {
            ModificationContent::Insertion(_) | ModificationContent::Deletion(_) => false,
            ModificationContent::TokenInsertion(tokens) => tokens
                .iter()
                .any(|token| !token.metadata.as_ref().is_none_or(HashMap::is_empty)),
        }
    }
    // Trivial; shouldn't require unit tests
    fn apply_annotations<T>(&self, annotations: &mut Vec<T>) -> ModificationIndices
    where
        T: Annotation,
    {
        ModificationRange::from(self).apply_annotations(annotations)
    }
}

/// The content of a [`Modification`] object.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ModificationContent {
    /// Bytes to be inserted into the set.
    Insertion(Vec<u8>),
    /// Tokens to be inserted into the set.
    TokenInsertion(Vec<ContentToken>),
    /// The number of bytes to be removed from the set.
    Deletion(usize),
}

// Trivial; shouldn't require unit tests
impl ModificationContent {
    /// The length in bytes of the modification being performed.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            ModificationContent::Insertion(content) => content.len(),
            ModificationContent::TokenInsertion(content) => {
                content.iter().map(|token| token.content.len()).sum()
            }
            ModificationContent::Deletion(length) => *length,
        }
    }
    /// Returns `true` if the modification has a length of zero bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        match self {
            ModificationContent::Insertion(content) => content.is_empty(),
            ModificationContent::TokenInsertion(content) => {
                content.iter().all(|token| token.content.is_empty())
            }
            ModificationContent::Deletion(length) => *length == 0,
        }
    }
}

/// The modification count for a [`Diff`] object.
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub struct ModificationCount {
    /// The total number of modifications.
    pub total: usize,
    /// The number of insertion modifications.
    pub insertions: usize,
    /// The number of deletion modifications.
    pub deletions: usize,
}

impl Display for ModificationCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.deletions > 0 {
            if self.insertions > 0 {
                if self.deletions == 1 && self.insertions == 1 {
                    write!(f, "1 Insertion, 1 Deletion")
                } else {
                    write!(
                        f,
                        "{} Insertions, {} Deletions",
                        self.insertions, self.deletions
                    )
                }
            } else if self.deletions == 1 {
                write!(f, "1 Deletion")
            } else {
                write!(f, "{} Deletions", self.deletions)
            }
        } else if self.insertions > 0 {
            if self.insertions == 1 {
                write!(f, "1 Insertion")
            } else {
                write!(f, "{} Insertions", self.insertions)
            }
        } else {
            write!(f, "No Changes")
        }
    }
}

pub(super) enum ModificationRange {
    Insertion(Range<usize>),
    TokenInsertion(ModificationRangeTokens),
    Deletion(Range<usize>),
}

pub(super) struct ModificationRangeTokens {
    range: Range<usize>,
    tokens: Vec<(usize, Option<HashMap<String, String>>)>,
}

// Trivial; shouldn't require unit tests
impl From<&Modification> for ModificationRange {
    fn from(input: &Modification) -> Self {
        match &input.content {
            ModificationContent::Insertion(_) => Self::Insertion(input.range()),
            ModificationContent::TokenInsertion(tokens) => {
                Self::TokenInsertion(ModificationRangeTokens {
                    range: input.range(),
                    tokens: tokens
                        .iter()
                        .map(|token| (token.content.len(), token.metadata.clone()))
                        .collect(),
                })
            }
            ModificationContent::Deletion(_) => Self::Deletion(input.range()),
        }
    }
}

impl ModificationRange {
    // Trivial; shouldn't require unit tests
    #[inline]
    pub(super) fn range(&self) -> &Range<usize> {
        match self {
            Self::Deletion(range) | Self::Insertion(range) => range,
            Self::TokenInsertion(token_set) => &token_set.range,
        }
    }
    // assumes annotations are sorted and contigious starting at 0
    #[allow(clippy::too_many_lines)]
    pub(super) fn apply_annotations<T>(self, annotations: &mut Vec<T>) -> ModificationIndices
    where
        T: Annotation,
    {
        let range = self.range();
        let offset = range.end - range.start;
        if offset == 0 {
            return ModificationIndices::default();
        }
        let end = annotations
            .last()
            .map(|annotation| annotation.range().end)
            .unwrap_or_default();
        assert!((range.start <= end));
        let selected = annotations
            .iter()
            .enumerate()
            .find_map(|(location, annotation)| {
                let annotation = annotation.range();

                if annotation.end >= range.start && annotation.start < range.end {
                    return Some(location);
                }

                None
            })
            .or(if annotations.is_empty() && range.start == 0 {
                Some(0)
            } else {
                None
            })
            .unwrap();

        let mut split = (None, None);

        match self {
            Self::Insertion(range) => {
                let annotation = annotations
                    .get(selected)
                    .map_or(&Range { start: 0, end: 0 }, |annotation| annotation.range());
                #[allow(unused_assignments)]
                let mut insertion = None;
                if range.start == annotation.start {
                    let middle = T::from(range);
                    annotations.splice(selected..selected, vec![middle]);
                    insertion = Some(selected);
                } else if range.start == annotation.end {
                    let middle = T::from(range);
                    annotations.splice((selected + 1)..=selected, vec![middle]);
                    insertion = Some(selected + 1);
                } else {
                    let (left, mut right) = annotations[selected]
                        .split(range.start - annotation.start)
                        .unwrap();
                    let middle = T::from(range);
                    right.range_mut().start += offset;
                    right.range_mut().end += offset;
                    annotations.splice(selected..=selected, vec![left, middle, right]);
                    split = (Some(selected), Some(selected + 2));
                    insertion = Some(selected + 1);
                }

                let modification_ending = split.1.unwrap_or(insertion.unwrap());
                if annotations.len() > modification_ending {
                    for annotation in &mut annotations[modification_ending + 1..] {
                        let annotation = annotation.range_mut();
                        annotation.start += offset;
                        annotation.end += offset;
                    }
                }

                ModificationIndices {
                    inserted_bytes: insertion,
                    inserted_tokens: None,
                    left_split: split.0,
                    right_split: split.1,
                }
            }
            Self::TokenInsertion(tokens) => {
                let annotation = annotations
                    .get(selected)
                    .map_or(&Range { start: 0, end: 0 }, |annotation| annotation.range());
                let mut next_token_start = tokens.range.start;
                let mut token_annotations: Vec<T> = tokens
                    .tokens
                    .into_iter()
                    .map(|(token_length, _)| {
                        let range = Range {
                            start: next_token_start,
                            end: next_token_start + token_length,
                        };

                        next_token_start = range.end;

                        T::from(range)
                    })
                    .collect();
                assert!(token_annotations.last().unwrap().range().end == tokens.range.end);
                let token_count = token_annotations.len();
                #[allow(unused_assignments)]
                let mut insertion = None;

                #[allow(clippy::range_minus_one)]
                if tokens.range.start == annotation.start {
                    annotations.splice(selected..selected, token_annotations);
                    insertion = Some(selected..=(selected + token_count - 1));
                } else if tokens.range.start == annotation.end {
                    annotations.splice((selected + 1)..=selected, token_annotations);
                    insertion = Some(selected + 1..=(selected + token_count));
                } else {
                    let (left, mut right) = annotations[selected]
                        .split(tokens.range.start - annotation.start)
                        .unwrap();
                    token_annotations.splice(0..0, vec![left]);
                    right.range_mut().start += offset;
                    right.range_mut().end += offset;
                    token_annotations.push(right);

                    annotations.splice(selected..=selected, token_annotations);
                    insertion = Some((selected + 1)..=(selected + token_count));
                    split = (Some(selected), Some(selected + token_count + 1));
                }

                let modification_ending = split.1.unwrap_or(*insertion.as_ref().unwrap().end());
                if annotations.len() > modification_ending {
                    for annotation in &mut annotations[modification_ending + 1..] {
                        let annotation = annotation.range_mut();
                        annotation.start += offset;
                        annotation.end += offset;
                    }
                }

                ModificationIndices {
                    inserted_bytes: None,
                    inserted_tokens: insertion,
                    left_split: split.0,
                    right_split: split.1,
                }
            }
            Self::Deletion(range) => {
                assert!((range.end <= end));
                let mut remove = Vec::with_capacity(annotations.len());
                let mut index_offset = 0;

                for (index, annotation) in &mut annotations[selected..].iter_mut().enumerate() {
                    let annotation = annotation.range_mut();

                    if annotation.end == range.start {
                        continue;
                    }

                    if annotation.start >= range.start && annotation.end <= range.end {
                        remove.push(index + selected);
                        index_offset += 1;
                    } else if range.start > annotation.start && range.end < annotation.end {
                        let index = index + selected;
                        let end = annotation.end - offset;
                        let split_position = range.start - annotation.start;
                        let (left, mut right) = annotations[index].split(split_position).unwrap();

                        right.range_mut().end = end;

                        annotations.splice(index..=index, vec![left, right]);
                        split.0 = Some(index);
                        split.1 = Some(index + 1);

                        if annotations.len() > index + 1 {
                            for annotation in &mut annotations[index + 2..].iter_mut() {
                                let annotation = annotation.range_mut();

                                annotation.start -= offset;
                                annotation.end -= offset;
                            }
                        }

                        break;
                    } else if annotation.start >= range.start && annotation.start < range.end {
                        annotation.start = range.start;
                        annotation.end -= offset;
                        split.1 = Some(index + selected - index_offset);
                    } else if annotation.start < range.end {
                        annotation.end = range.start;
                        split.0 = Some(index + selected - index_offset);
                    } else {
                        annotation.start -= offset;
                        annotation.end -= offset;
                    }
                }
                if !remove.is_empty() {
                    annotations.splice(remove.first().unwrap()..=remove.last().unwrap(), vec![]);
                }

                ModificationIndices {
                    inserted_bytes: None,
                    inserted_tokens: None,
                    left_split: split.0,
                    right_split: split.1,
                }
            }
        }
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
pub(super) struct ModificationIndices {
    pub(super) inserted_bytes: Option<usize>,
    pub(super) inserted_tokens: Option<RangeInclusive<usize>>,
    pub(super) left_split: Option<usize>,
    pub(super) right_split: Option<usize>,
}
