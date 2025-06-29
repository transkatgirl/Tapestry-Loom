//! Interactive representations of Weave contents.

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    iter,
    ops::Range,
    time::Instant,
    vec,
};

use serde::{Deserialize, Serialize};
use similar::{Algorithm, DiffTag, capture_diff_slices_deadline};
use ulid::Ulid;

#[allow(unused_imports)]
use super::{Weave, WeaveView};

#[cfg(test)]
mod tests;

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

/// A set of active nodes listed in the order they are connected.
///
/// WeaveTimeline objects are created by the [`WeaveView::get_active_timelines`] function. Each timeline represents one possible linear progression of nodes (and their associated models), starting at the root of the [`Weave`] and progressing outwards.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct WeaveTimeline<'w> {
    #[allow(missing_docs)]
    pub timeline: Vec<(&'w Node, Option<&'w Model>)>,
}

impl<'w> WeaveTimeline<'w> {
    /// Returns the output of the timeline as a set of bytes.
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
    /// Creates a new set of annotations previewing a change to the [`Weave`]'s contents.
    ///
    /// This calculates a [`Diff`] between the current contents of the timeline and the user input, and then creates a new set of annotations reflecting the changes made by the user.
    pub fn preview_update(
        &self,
        (string, mut annotations): (&str, Vec<TimelineAnnotation<'w>>),
        new_content: &str,
        diff_deadline: Instant,
    ) -> Vec<TimelineAnnotation<'w>> {
        Diff::new(string.as_bytes(), new_content.as_bytes(), diff_deadline)
            .apply_annotations(&mut annotations);

        annotations
    }
    pub(super) fn build_update(self, content: String, deadline: Instant) -> TimelineUpdate {
        let (before, annotations) = self.annotated_string();

        let mut last_node = None;
        let mut ranges: Vec<TimelineNodeRange> = Vec::with_capacity(annotations.len());

        for annotation in annotations {
            if let Some(node) = annotation.node {
                let node = node.id;

                if let Some(last_node) = last_node {
                    if node == last_node {
                        if let Some(last) = ranges.last_mut() {
                            last.range.end = annotation.range.end;
                        }
                    }
                }

                ranges.push(TimelineNodeRange {
                    range: annotation.range,
                    node: Some(node),
                });
            }

            last_node = annotation.node.map(|node| node.id);
        }

        TimelineUpdate {
            ranges,
            diff: Diff::new(&before.into_bytes(), &content.into_bytes(), deadline),
        }
    }
}

pub(super) struct TimelineUpdate {
    pub(super) ranges: Vec<TimelineNodeRange>,
    pub(super) diff: Diff,
}

pub(super) struct TimelineNodeRange {
    pub(super) range: Range<usize>,
    pub(super) node: Option<Ulid>,
}

impl From<Range<usize>> for TimelineNodeRange {
    fn from(input: Range<usize>) -> Self {
        Self {
            range: input,
            node: None,
        }
    }
}

impl Annotation for TimelineNodeRange {
    fn range(&self) -> &Range<usize> {
        &self.range
    }
    fn range_mut(&mut self) -> &mut Range<usize> {
        &mut self.range
    }
    fn split(&self, index: usize) -> Option<(Self, Self)> {
        if index == 0 || index >= self.range.end {
            return None;
        }

        let mut left = self.range.clone();
        let mut right = self.range.clone();

        left.end -= index;
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
    /// Merges two sections of content together.
    ///
    /// This requires both sections to be concatable and contain the same metadata.
    pub fn merge(left: Self, right: Self) -> Option<Self> {
        if left.model() == right.model()
            && left.metadata() == right.metadata()
            && (left.is_concatable() && right.is_concatable())
        {
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
        } else {
            None
        }
    }
    /// Returns `true` if the two sections of content can be merged together.
    pub fn is_mergeable(left: &Self, right: &Self) -> bool {
        left.model() == right.model()
            && left.metadata() == right.metadata()
            && (left.is_concatable() && right.is_concatable())
    }
    /// Splits the content in half at the specified index, retaining all associated metadata.
    ///
    /// Some types of content cannot be split in half and will always return [`None`] regardless of the index specified.
    pub fn split(self, index: usize) -> Option<(Self, Self)> {
        match self {
            Self::Snippet(content) => content
                .split(index)
                .map(|(left, right)| (Self::Snippet(left), Self::Snippet(right))),
            Self::Tokens(content) => content
                .split(index)
                .map(|(left, right)| (Self::Tokens(left), Self::Tokens(right))),
            Self::Diff(_) => None,
            Self::Blank => Some((Self::Blank, Self::Blank)),
        }
        .map(|(left, right)| (left.reduce(), right.reduce()))
    }
    /// Returns `true` if the content can be split.
    pub fn is_splitable(&self, index: usize) -> bool {
        match self {
            Self::Snippet(content) => index <= content.len(),
            Self::Tokens(content) => index <= content.len(),
            Self::Diff(_) => false,
            Self::Blank => true,
        }
    }
    fn reduce(self) -> Self {
        if self.model().is_none() && self.metadata().is_none() && self.is_empty() {
            return Self::Blank;
        }

        match self {
            Self::Snippet(bytes) => Self::Snippet(bytes),
            Self::Tokens(mut tokens) => {
                if tokens.content.is_empty() {
                    Self::Blank
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
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Snippet(content) => content.is_empty(),
            Self::Tokens(content) => content.is_empty(),
            Self::Diff(diff) => diff.content.is_empty(),
            Self::Blank => true,
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
    fn range(&self) -> &Range<usize>;
    /// Returns a mutable reference to the annotation's byte range.
    fn range_mut(&mut self) -> &mut Range<usize>;
    /// Splits the annotation in half at the specified index, retaining all associated metadata.
    fn split(&self, index: usize) -> Option<(Self, Self)>;
}

impl Annotation for ContentAnnotation<'_> {
    fn range(&self) -> &Range<usize> {
        &self.range
    }
    fn range_mut(&mut self) -> &mut Range<usize> {
        &mut self.range
    }
    fn split(&self, index: usize) -> Option<(Self, Self)> {
        if index == 0 || index >= self.range.end {
            return None;
        }

        let mut left = self.range.clone();
        let mut right = self.range.clone();

        left.end -= index;
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
    fn range(&self) -> &Range<usize> {
        &self.range
    }
    fn range_mut(&mut self) -> &mut Range<usize> {
        &mut self.range
    }
    fn split(&self, index: usize) -> Option<(Self, Self)> {
        if index == 0 || index >= self.range.end {
            return None;
        }

        let mut left = self.range.clone();
        let mut right = self.range.clone();

        left.end -= index;
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

impl From<Range<usize>> for ContentAnnotation<'_> {
    fn from(range: Range<usize>) -> Self {
        Self {
            range,
            metadata: None,
        }
    }
}

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
    fn model(&self) -> Option<&ContentModel>;
    /// Returns metadata associated with the content.
    fn metadata(&self) -> Option<&HashMap<String, String>>;
}

/// Concatable types which are intended to be used as content for a [`Node`] object.
pub trait ConcatableNodeContents: NodeContents {
    /// Converts the content into a set of bytes.
    fn into_bytes(self) -> Vec<u8>;
    /// Returns the length from the content in bytes.
    fn len(&self) -> usize;
    /// Returns `true` if the content has a length of zero bytes (excluding metadata).
    fn is_empty(&self) -> bool;
    /// Returns annotations for the content.
    fn annotations(&self) -> impl Iterator<Item = ContentAnnotation>;
    /// Splits the content in half at the specified index.
    ///
    /// If the content has metadata, both sides of the split retain that metadata.
    fn split(self, index: usize) -> Option<(Self, Self)>;
}

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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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

impl NodeContents for SnippetContent {
    fn model(&self) -> Option<&ContentModel> {
        self.model.as_ref()
    }
    fn metadata(&self) -> Option<&HashMap<String, String>> {
        self.metadata.as_ref()
    }
}

impl Display for SnippetContent {
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

impl ConcatableNodeContents for SnippetContent {
    fn into_bytes(self) -> Vec<u8> {
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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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

impl NodeContents for TokenContent {
    fn model(&self) -> Option<&ContentModel> {
        self.model.as_ref()
    }
    fn metadata(&self) -> Option<&HashMap<String, String>> {
        self.metadata.as_ref()
    }
}

impl Display for TokenContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bytes = self.clone().into_bytes();

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

impl ConcatableNodeContents for TokenContent {
    fn into_bytes(self) -> Vec<u8> {
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
                metadata: token.metadata.as_ref(),
            }
        })
    }
    fn split(self, index: usize) -> Option<(Self, Self)> {
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
            return Some((
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
            ));
        }

        let location = location?;

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
}

/// A single UTF-8 token from a tokenized piece of text.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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

impl From<Vec<u8>> for ContentToken {
    fn from(input: Vec<u8>) -> Self {
        Self {
            content: input,
            metadata: None,
        }
    }
}

/// A container for a set of modifications to perform on the current text.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DiffContent {
    /// The modification set being stored.
    pub content: Diff,
    /// Metadata about the algorithmic process which generated the modification set, if any.
    pub model: Option<ContentModel>,
    /// Metadata associated with the content.
    pub metadata: Option<HashMap<String, String>>,
}

impl NodeContents for DiffContent {
    fn model(&self) -> Option<&ContentModel> {
        self.model.as_ref()
    }
    fn metadata(&self) -> Option<&HashMap<String, String>> {
        self.metadata.as_ref()
    }
}

impl Display for DiffContent {
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

/// A list of modifications to perform on a set of bytes.
///
/// This has little meaning on its own, as it is meant to be paired with the text being modified.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, PartialOrd, Ord, Eq)]
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
    /// Applies the diff to a set of bytes.
    pub fn apply(self, data: &mut Vec<u8>) {
        for modification in self.content {
            modification.apply(data);
        }
    }
    pub(super) fn apply_annotations<T>(&self, annotations: &mut Vec<T>)
    where
        T: Annotation,
    {
        for modification in &self.content {
            modification.apply_annotations(annotations);
        }
    }
    pub(super) fn apply_timeline_annotations<'w>(
        &self,
        node: &'w Node,
        model: Option<&'w Model>,
        content_metadata: Option<&'w HashMap<String, String>>,
        annotations: &mut Vec<TimelineAnnotation<'w>>,
    ) {
        for modification in &self.content {
            if let Some(index) = modification.apply_annotations(annotations) {
                annotations[index].node = Some(node);
                annotations[index].model = model;
                annotations[index].content_metadata = content_metadata;
            }
        }
    }
    /// Calculates the total number of modifications in the [`Diff`] by type.
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
    /// Returns `true` if the sum of all modification lengths in the diff is equal to zero bytes.
    pub fn is_empty(&self) -> bool {
        for modification in &self.content {
            if !modification.content.is_empty() {
                return false;
            }
        }
        true
    }
}

/// A modification to perform on a set of bytes.
///
/// This has little meaning on its own, as it is meant to be paired with the text being modified.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub struct Modification {
    /// The index where the modification should be applied.
    pub index: usize,
    /// The content of the modification.
    pub content: ModificationContent,
}

impl Modification {
    /// Applies the modification to a set of bytes.
    pub fn apply(self, data: &mut Vec<u8>) {
        match self.content {
            ModificationContent::Insertion(content) => data.splice(self.index..self.index, content),
            ModificationContent::Deletion(length) => data.splice(self.index..length, vec![]),
        };
    }
    /// Returns the range of bytes that the modification will be performed on.
    ///
    /// If this is an insertion modification, the end of the range represents the end of the inserted content.
    pub fn range(&self) -> Range<usize> {
        Range {
            start: self.index,
            end: self.index + self.content.len(),
        }
    }
    pub(super) fn apply_annotations<T>(&self, annotations: &mut Vec<T>) -> Option<usize>
    where
        T: Annotation,
    {
        ModificationRange::from(self).apply_annotations(annotations)
    }
}

/// The content of a [`Modification`] object.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum ModificationContent {
    /// Bytes to be inserted into the set.
    Insertion(Vec<u8>),
    /// The number of bytes to be removed from the set.
    Deletion(usize),
}

impl ModificationContent {
    /// The length in bytes of the modification being performed.
    pub fn len(&self) -> usize {
        match self {
            ModificationContent::Insertion(content) => content.len(),
            ModificationContent::Deletion(length) => *length,
        }
    }
    /// Returns `true` if the modification has a length of zero bytes.
    pub fn is_empty(&self) -> bool {
        match self {
            ModificationContent::Insertion(content) => content.is_empty(),
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(super) enum ModificationRange {
    Insertion(Range<usize>),
    Deletion(Range<usize>),
}

impl From<&Modification> for ModificationRange {
    fn from(input: &Modification) -> Self {
        match &input.content {
            ModificationContent::Insertion(_) => Self::Insertion(input.range()),
            ModificationContent::Deletion(_) => Self::Deletion(input.range()),
        }
    }
}

impl ModificationRange {
    pub(super) fn range(&self) -> &Range<usize> {
        match self {
            Self::Deletion(range) | Self::Insertion(range) => range,
        }
    }
    pub(super) fn len(&self) -> usize {
        let range = self.range();

        range.end - range.start
    }
    pub(super) fn apply_annotations<T>(self, annotations: &mut Vec<T>) -> Option<usize>
    where
        T: Annotation,
    {
        let offset = self.len();
        let selected = annotations
            .iter()
            .enumerate()
            .find_map(|(location, annotation)| {
                let annotation = annotation.range();
                let range = self.range();

                if range.contains(&annotation.start) || range.contains(&annotation.end) {
                    return Some(location);
                }

                None
            })?;

        match self {
            Self::Insertion(range) => {
                if let Some((left, mut right)) = annotations[selected].split(range.start) {
                    let middle = T::from(range);
                    right.range_mut().start += offset;
                    right.range_mut().end += offset;

                    annotations.splice(selected..=selected, vec![left, middle, right]);
                } else {
                    let middle = T::from(range);
                    annotations.splice(selected..selected, vec![middle]);
                }
                if annotations.len() > selected {
                    for annotation in &mut annotations[selected + 2..] {
                        let annotation = annotation.range_mut();
                        annotation.start += offset;
                        annotation.end += offset;
                    }
                }

                Some(selected + 1)
            }
            Self::Deletion(range) => {
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

                None
            }
        }
    }
}
