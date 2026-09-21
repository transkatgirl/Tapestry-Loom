//! [`Node`](universal_weave::Node) content representations.

use std::{borrow::Cow, iter};

use jiff::Zoned;
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};
use universal_weave::{
    DeduplicatableContents, DiscreteContentResult, DiscreteContents, IndependentContents,
    rkyv::{
        Archive, Deserialize, Serialize, niche::niching, option::ArchivedOption, with::NicheInto,
    },
};

use crate::{
    metadata::MetadataMap,
    util::{AsBinaryZoned, Base64Standard, IAsVec},
    weave::LongId,
};

pub mod sort;

/// The 1-byte long ASCII substitution character, used to replace bytes which are invalid UTF-8.
pub const SUBSTITUTION_CHARACTER: &str = "\u{1A}";
const _: () = assert!(SUBSTITUTION_CHARACTER.len() == 1);

/// The contents of a [`TapestryNode`](crate::weave::TapestryNode).
///
/// **This should not be used to store sensitive information**, such as endpoint URLs or API keys, as the user may choose to share documents publicly.
#[derive(
    SerdeSerialize, SerdeDeserialize, Archive, Deserialize, Serialize, Debug, Clone, PartialEq,
)]
pub struct NodeContent {
    /// The instant the node was created.
    #[rkyv(with = AsBinaryZoned)]
    pub timestamp: Zoned,
    /// If the node has been split or merged.
    ///
    /// Other types of content modifications should update the `creator` field and reset this field to `false`.
    pub modified: bool,

    /// The inner contents of the node.
    pub content: InnerNodeContent,

    /// User-readable metadata associated with the node.
    ///
    /// This is typically used to store the following data:
    /// - Generation parameters not currently stored in [`Model`]
    /// - (Atypical) response fields not currently stored in [`Model`]
    /// - Converted node-specific metadata lacking a dedicated field (such as tags or associated media)
    /// - Anything else important to the user
    ///
    /// Split nodes inherit the original node's metadata, and nodes can only be merged if they have identical metadata.
    #[rkyv(with = IAsVec)]
    pub metadata: MetadataMap,

    /// The entity which created this node's contents.
    pub creator: Creator,
}

impl Default for NodeContent {
    fn default() -> Self {
        Self {
            timestamp: Zoned::now(),
            modified: false,
            content: InnerNodeContent::MetadataOnly,
            metadata: MetadataMap::default(),
            creator: Creator::User(None),
        }
    }
}

impl IndependentContents for NodeContent {}

impl DiscreteContents for NodeContent {
    fn len(&self) -> usize {
        self.content.len()
    }
    fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
    fn split(mut self, at: usize) -> DiscreteContentResult<Self> {
        match self.content.split(at) {
            DiscreteContentResult::Two(left, right) => {
                self.content = left;
                self.modified = true;

                let right_content = NodeContent {
                    timestamp: self.timestamp.clone(),
                    modified: true,
                    content: right,
                    metadata: self.metadata.clone(),
                    creator: self.creator.clone(),
                };

                DiscreteContentResult::Two(self, right_content)
            }
            DiscreteContentResult::One(center) => {
                self.content = center;
                DiscreteContentResult::One(self)
            }
        }
    }
    fn merge(mut self, mut value: Self) -> DiscreteContentResult<Self> {
        if self.metadata != value.metadata || !self.creator.is_mergeable_with(&value.creator) {
            return DiscreteContentResult::Two(self, value);
        }

        match self.content.merge(value.content) {
            DiscreteContentResult::Two(left, right) => {
                self.content = left;
                value.content = right;

                DiscreteContentResult::Two(self, value)
            }
            DiscreteContentResult::One(center) => {
                self.content = center;
                self.modified = true;
                self.timestamp = self.timestamp.max(value.timestamp);
                self.creator = self.creator.merge(value.creator).unwrap();
                DiscreteContentResult::One(self)
            }
        }
    }
}

impl NodeContent {
    /// Returns `true` if [`Self::merge`] would succeed.
    pub fn is_mergeable_with(&self, value: &Self) -> bool {
        self.metadata == value.metadata
            && self.creator.is_mergeable_with(&value.creator)
            && self.content.is_mergeable_with(&value.content)
    }
    /// Corrects empty or malformed fields
    pub fn normalize(&mut self) {
        self.content.normalize();
        self.creator.normalize();
    }
}

impl DeduplicatableContents for NodeContent {
    fn is_duplicate_of(&self, value: &Self) -> bool {
        self.metadata == value.metadata
            && self.content.is_duplicate_of(&value.content)
            && self.creator.is_duplicate_of(&value.creator)
    }
}

/// The inner contents of a [`TapestryNode`](crate::weave::TapestryNode).
///
/// # Text Encoding
///
/// Text is stored as bytes which should be interpreted as UTF-8.
///
/// However, LLM tokenization does not guarantee that individual tokens are UTF-8. As a result, contents may contain invalid UTF-8, or UTF-8 which is only valid when combined with the contents of another node.
#[derive(
    SerdeSerialize, SerdeDeserialize, Archive, Deserialize, Serialize, Debug, Clone, PartialEq,
)]
pub enum InnerNodeContent {
    /// A text segment.
    Snippet(#[serde(with = "Base64Standard")] Vec<u8>),
    /// A tokenized text segment.
    Tokens(Vec<InnerNodeToken>),
    /// A node without any associated contents.
    MetadataOnly,
}

impl InnerNodeContent {
    pub fn token_count(&self) -> Option<usize> {
        if let Self::Tokens(tokens) = self {
            Some(tokens.len())
        } else {
            None
        }
    }
    pub fn contains_modified_tokens(&self) -> Option<bool> {
        if let Self::Tokens(tokens) = self {
            Some(tokens.iter().any(|token| token.is_modified()))
        } else {
            None
        }
    }
    pub fn calculate_average_logprob(&self) -> Option<f32> {
        if let Self::Tokens(tokens) = self
            && !tokens.is_empty()
        {
            let mut logprob_sum = 0.0;

            for token in tokens {
                if token.is_modified() {
                    return None;
                }

                let logprob = normalize_f32(token.logprob)?;
                logprob_sum += logprob as f64;
            }

            normalize_f32(Some((logprob_sum / tokens.len() as f64) as f32))
        } else {
            None
        }
    }
    pub fn calculate_cumulative_logprob(&self) -> Option<f64> {
        if let Self::Tokens(tokens) = self
            && !tokens.is_empty()
        {
            let mut logprob_sum = 0.0;

            for token in tokens {
                if token.is_modified() {
                    return None;
                }

                let logprob = normalize_f32(token.logprob)?;
                logprob_sum += logprob as f64;
            }

            normalize_f64(Some(logprob_sum))
        } else {
            None
        }
    }
    pub fn calculate_confidence(&self) -> Option<(f32, usize, usize)> {
        if let Self::Tokens(tokens) = self
            && !tokens.is_empty()
        {
            let mut confidence_sum = 0.0;
            let mut confidence_k = None;

            for token in tokens {
                if token.is_modified() {
                    return None;
                }

                if let Some((confidence, k)) = token.calculate_confidence_inner() {
                    if let Some(last_k) = confidence_k
                        && last_k != k
                    {
                        return None;
                    } else {
                        confidence_k = Some(k);
                    }

                    confidence_sum += confidence;
                } else {
                    return None;
                }
            }

            let confidence = normalize_f32(Some((confidence_sum / tokens.len() as f64) as f32))?;
            let confidence_k = confidence_k?;

            Some((confidence, confidence_k, tokens.len()))
        } else {
            None
        }
    }
    pub fn calculate_average_entropy(&self) -> Option<f32> {
        if let Self::Tokens(tokens) = self
            && !tokens.is_empty()
        {
            let mut entropy_sum = 0.0;

            for token in tokens.iter() {
                if token.is_modified() {
                    return None;
                }

                let entropy = normalize_f32(token.entropy)?;
                entropy_sum += entropy as f64;
            }

            normalize_f32(Some((entropy_sum / tokens.len() as f64) as f32))
        } else {
            None
        }
    }
    pub fn truncate_tokens(&mut self, len: usize) {
        if let Self::Tokens(tokens) = self {
            tokens.truncate(len);
            tokens.shrink_to_fit();
        }
    }
    pub fn truncate_counterfactual(&mut self, len: usize) {
        if let Self::Tokens(tokens) = self {
            for token in tokens {
                token.truncate_counterfactual(len);
            }
        }
    }
    pub fn sort_counterfactual(&mut self) {
        if let Self::Tokens(tokens) = self {
            for token in tokens {
                token.sort_counterfactual();
            }
        }
    }
    /// Corrects empty or malformed fields
    pub fn normalize(&mut self) {
        if let Self::Tokens(tokens) = self {
            for token in tokens {
                token.normalize();
            }
        }
    }
}

impl ArchivedInnerNodeContent {
    pub fn token_count(&self) -> Option<usize> {
        if let Self::Tokens(tokens) = self {
            Some(tokens.len())
        } else {
            None
        }
    }
    pub fn calculate_average_logprob(&self) -> Option<f32> {
        if let Self::Tokens(tokens) = self
            && !tokens.is_empty()
        {
            let mut logprob_sum = 0.0;

            for token in tokens.iter() {
                if token.is_modified() {
                    return None;
                }

                let logprob =
                    normalize_f32(token.logprob.as_ref().map(|logprob| logprob.to_native()))?;
                logprob_sum += logprob as f64;
            }

            normalize_f32(Some((logprob_sum / tokens.len() as f64) as f32))
        } else {
            None
        }
    }
    pub fn calculate_cumulative_logprob(&self) -> Option<f64> {
        if let Self::Tokens(tokens) = self
            && !tokens.is_empty()
        {
            let mut logprob_sum = 0.0;

            for token in tokens.iter() {
                if token.is_modified() {
                    return None;
                }

                let logprob =
                    normalize_f32(token.logprob.as_ref().map(|logprob| logprob.to_native()))?;
                logprob_sum += logprob as f64;
            }

            normalize_f64(Some(logprob_sum))
        } else {
            None
        }
    }
    pub fn calculate_confidence(&self) -> Option<(f32, usize, usize)> {
        if let Self::Tokens(tokens) = self
            && !tokens.is_empty()
        {
            let mut confidence_sum = 0.0;
            let mut confidence_k = None;

            for token in tokens.iter() {
                if token.is_modified() {
                    return None;
                }

                if let Some((confidence, k)) = token.calculate_confidence_inner() {
                    if let Some(last_k) = confidence_k
                        && last_k != k
                    {
                        return None;
                    } else {
                        confidence_k = Some(k);
                    }

                    confidence_sum += confidence;
                } else {
                    return None;
                }
            }

            let confidence = normalize_f32(Some((confidence_sum / tokens.len() as f64) as f32))?;
            let confidence_k = confidence_k?;

            Some((confidence, confidence_k, tokens.len()))
        } else {
            None
        }
    }
    pub fn calculate_average_entropy(&self) -> Option<f32> {
        if let Self::Tokens(tokens) = self
            && !tokens.is_empty()
        {
            let mut entropy_sum = 0.0;

            for token in tokens.iter() {
                if token.is_modified() {
                    return None;
                }

                let entropy =
                    normalize_f32(token.entropy.as_ref().map(|entropy| entropy.to_native()))?;
                entropy_sum += entropy as f64;
            }

            normalize_f32(Some((entropy_sum / tokens.len() as f64) as f32))
        } else {
            None
        }
    }
}

/// A textual token from [`InnerNodeContent::Tokens`].
///
/// # Text Encoding
///
/// Text is stored as bytes which should be interpreted as UTF-8.
///
/// However, LLM tokenization does not guarantee that individual tokens are UTF-8. As a result, tokens may contain invalid UTF-8, or UTF-8 which is only valid when combined with other tokens.
#[derive(
    SerdeSerialize, SerdeDeserialize, Archive, Deserialize, Serialize, Debug, Clone, PartialEq,
)]
pub struct InnerNodeToken {
    /// The token's textual representation.
    ///
    /// If the token is not printable, this field should be empty.
    #[serde(with = "Base64Standard")]
    pub bytes: Vec<u8>,
    /// The natural logarithm of the probability associated with the token.
    #[rkyv(with = NicheInto<niching::NaN>)]
    pub logprob: Option<f32>,
    /// The generator-specific numeric ID associated with the token.
    pub id: Option<u64>,

    /// The entropy value associated with the current position.
    #[rkyv(with = NicheInto<niching::NaN>)]
    pub entropy: Option<f32>,
    /// The counterfactual tokens for the current position.
    pub counterfactual: Vec<CounterfactualToken>,

    /// The token's original `bytes` value, if any.
    pub original: OriginalToken,
}

impl InnerNodeToken {
    pub fn from_counterfactual_pair(
        token: CounterfactualToken,
        counterfactual: Vec<CounterfactualToken>,
    ) -> Self {
        Self {
            bytes: token.bytes,
            logprob: token.logprob,
            id: token.id,
            entropy: None,
            counterfactual,
            original: OriginalToken::Unmodified,
        }
    }
    pub fn sort_counterfactual(&mut self) {
        self.counterfactual.sort_by(|a, b| {
            normalize_f32(b.logprob)
                .unwrap_or(f32::NEG_INFINITY)
                .total_cmp(&normalize_f32(a.logprob).unwrap_or(f32::NEG_INFINITY))
                .then_with(|| a.bytes.cmp(&b.bytes))
                .then_with(|| a.id.cmp(&b.id))
        });
    }
    pub fn truncate_counterfactual(&mut self, len: usize) {
        self.counterfactual.truncate(len);
        self.counterfactual.shrink_to_fit();
    }
    /// Corrects empty or malformed fields
    pub fn normalize(&mut self) {
        self.logprob = normalize_f32(self.logprob);
        self.entropy = normalize_f32(self.entropy);

        if let OriginalToken::Known { bytes, offset, .. } = &self.original
            && !bytes
                .get(*offset..)
                .is_some_and(|original| original.starts_with(&self.bytes))
        {
            self.original = OriginalToken::Unknown;
        }

        for counterfactual in &mut self.counterfactual {
            counterfactual.normalize();
        }

        self.sort_counterfactual();
    }
    pub fn calculate_confidence(&self) -> Option<(f32, usize)> {
        self.calculate_confidence_inner()
            .and_then(|(confidence, k)| normalize_f32(Some(confidence as f32)).map(|c| (c, k)))
    }
    fn calculate_confidence_inner(&self) -> Option<(f64, usize)> {
        if !self.counterfactual.is_empty() {
            let mut counterfactual_logprob_sum = 0.0;

            for token in &self.counterfactual {
                let logprob = normalize_f32(token.logprob)? as f64;
                counterfactual_logprob_sum += logprob;
            }

            Some((
                counterfactual_logprob_sum / -(self.counterfactual.len() as f64) + 0.0,
                self.counterfactual.len(),
            ))
        } else {
            None
        }
    }
    pub fn is_modified(&self) -> bool {
        self.original.is_modified()
    }
    /// Returns `true` if `self` and `value` should be considered duplicates.
    pub fn is_duplicate_of(&self, value: &Self) -> bool {
        self.bytes == value.bytes
            && self.id == value.id
            && normalize_f32(self.logprob).is_some() == normalize_f32(value.logprob).is_some()
            && normalize_f32(self.entropy).is_some() == normalize_f32(value.entropy).is_some()
            && self.original == value.original
            && self.counterfactual.len() == value.counterfactual.len()
            && self
                .counterfactual
                .iter()
                .zip(value.counterfactual.iter())
                .all(|(left, right)| left.is_duplicate_of(right))
    }
}

impl ArchivedInnerNodeToken {
    pub fn calculate_confidence(&self) -> Option<(f32, usize)> {
        self.calculate_confidence_inner()
            .and_then(|(confidence, k)| normalize_f32(Some(confidence as f32)).map(|c| (c, k)))
    }
    fn calculate_confidence_inner(&self) -> Option<(f64, usize)> {
        if !self.counterfactual.is_empty() {
            let mut counterfactual_logprob_sum = 0.0;

            for token in self.counterfactual.iter() {
                let logprob =
                    normalize_f32(token.logprob.as_ref().map(|logprob| logprob.to_native()))?
                        as f64;
                counterfactual_logprob_sum += logprob;
            }

            Some((
                counterfactual_logprob_sum / -(self.counterfactual.len() as f64) + 0.0,
                self.counterfactual.len(),
            ))
        } else {
            None
        }
    }
    /// Returns `true` if the token's contents were modified.
    pub fn is_modified(&self) -> bool {
        self.original.is_modified()
    }
}

/// The original contents of an [`InnerNodeToken`].
#[derive(
    SerdeSerialize, SerdeDeserialize, Archive, Deserialize, Serialize, Debug, Clone, PartialEq,
)]
pub enum OriginalToken {
    /// The token has not been modified.
    Unmodified,
    /// The token has been modified by a split operation and the original contents are known.
    Known {
        /// The token's textual representation.
        #[serde(with = "Base64Standard")]
        bytes: Vec<u8>,
        /// The generator-specific numeric ID associated with the token.
        id: Option<u64>,
        /// The offset within `bytes` where the [`InnerNodeToken`]'s contents begin.
        offset: usize,
    },
    /// The token has been modified and the original contents are unknown.
    Unknown,
}

impl OriginalToken {
    pub fn is_modified(&self) -> bool {
        match self {
            Self::Known { .. } => true,
            Self::Unknown => true,
            Self::Unmodified => false,
        }
    }
}

impl ArchivedOriginalToken {
    pub fn is_modified(&self) -> bool {
        match self {
            Self::Known { .. } => true,
            Self::Unknown => true,
            Self::Unmodified => false,
        }
    }
}

/// A counterfactual [`InnerNodeToken`].
#[derive(
    SerdeSerialize, SerdeDeserialize, Archive, Deserialize, Serialize, Debug, Clone, PartialEq,
)]
pub struct CounterfactualToken {
    /// The token's textual representation.
    #[serde(with = "Base64Standard")]
    pub bytes: Vec<u8>,
    /// The natural logarithm of the probability associated with the token.
    #[rkyv(with = NicheInto<niching::NaN>)]
    pub logprob: Option<f32>,
    /// The generator-specific numeric ID associated with the token.
    pub id: Option<u64>,
}

impl CounterfactualToken {
    /// Corrects empty or malformed fields
    pub fn normalize(&mut self) {
        self.logprob = normalize_f32(self.logprob);
    }
    /// Calculates an entropy value from an iterator containing all possible tokens for a given position.
    pub fn calculate_entropy<'a>(
        tokens: impl Iterator<Item = &'a CounterfactualToken>,
    ) -> Option<f64> {
        let mut sum = 0.0;
        let mut empty = true;

        for token in tokens {
            let logprob = token.logprob? as f64;

            sum += if logprob.is_finite() {
                logprob.exp() * logprob
            } else {
                0.0
            };

            empty = false;
        }

        if empty {
            return None;
        }

        normalize_f64(Some(0.0 - sum))
    }
    /// Returns `true` if `self` and `value` should be considered duplicates.
    pub fn is_duplicate_of(&self, value: &Self) -> bool {
        self.bytes == value.bytes
            && self.id == value.id
            && normalize_f32(self.logprob).is_some() == normalize_f32(value.logprob).is_some()
    }
}

const EMPTY_VEC_REF: &Vec<u8> = &Vec::new();

impl InnerNodeContent {
    /// Splits the item at the specified index.
    ///
    /// If `at` is not inside the item, the original contents are returned.
    pub fn split(self, at: usize) -> DiscreteContentResult<Self> {
        if at == 0 {
            return DiscreteContentResult::One(self);
        }

        match self {
            Self::Snippet(mut snippet) => {
                if snippet.len() <= at {
                    return DiscreteContentResult::One(Self::Snippet(snippet));
                }

                let right = snippet.split_off(at);
                snippet.shrink_to_fit();

                DiscreteContentResult::Two(Self::Snippet(snippet), Self::Snippet(right))
            }
            Self::Tokens(tokens) => {
                if tokens.iter().map(|token| token.bytes.len()).sum::<usize>() <= at {
                    return DiscreteContentResult::One(Self::Tokens(tokens));
                }

                let mut content_index = 0;

                let location = tokens.iter().enumerate().find_map(|(location, token)| {
                    if content_index + token.bytes.len() > at {
                        return Some(location);
                    }
                    content_index += token.bytes.len();

                    None
                });

                if let Some(location) = location {
                    let mut left = tokens;
                    let mut right = left.split_off(location);
                    let mut left_token = right[0].bytes.clone();
                    let right_token = left_token.split_off(at - content_index);

                    debug_assert!(!right_token.is_empty() || left_token.is_empty());

                    if !left_token.is_empty() {
                        if !right[0].original.is_modified() {
                            right[0].original = OriginalToken::Known {
                                bytes: right[0].bytes.clone(),
                                id: right[0].id,
                                offset: 0,
                            };
                        }

                        left_token.shrink_to_fit();
                        let left_len = left_token.len();
                        left.push(InnerNodeToken {
                            bytes: left_token,
                            id: None,
                            entropy: right[0].entropy,
                            logprob: right[0].logprob,
                            counterfactual: right[0].counterfactual.clone(),
                            original: right[0].original.clone(),
                        });
                        right[0].id = None;
                        if let OriginalToken::Known { offset, .. } = &mut right[0].original {
                            *offset = offset.saturating_add(left_len);
                        }
                    }
                    right[0].bytes = right_token;

                    left.shrink_to_fit();

                    DiscreteContentResult::Two(Self::Tokens(left), Self::Tokens(right))
                } else {
                    DiscreteContentResult::One(Self::Tokens(tokens))
                }
            }
            Self::MetadataOnly => DiscreteContentResult::One(Self::MetadataOnly),
        }
    }
    /// Merges two items together.
    ///
    /// If merging the two items fails, the original contents are returned in the order they were specified.
    pub fn merge(self, value: Self) -> DiscreteContentResult<Self> {
        match self {
            Self::Snippet(mut left_snippet) => match value {
                Self::Snippet(mut right_snippet) => {
                    left_snippet.append(&mut right_snippet);
                    DiscreteContentResult::One(Self::Snippet(left_snippet))
                }
                Self::Tokens(right_tokens) => DiscreteContentResult::Two(
                    Self::Snippet(left_snippet),
                    Self::Tokens(right_tokens),
                ),
                Self::MetadataOnly => {
                    DiscreteContentResult::Two(Self::Snippet(left_snippet), Self::MetadataOnly)
                }
            },
            Self::Tokens(mut left_tokens) => match value {
                Self::Snippet(right_snippet) => DiscreteContentResult::Two(
                    Self::Tokens(left_tokens),
                    Self::Snippet(right_snippet),
                ),
                Self::Tokens(mut right_tokens) => {
                    left_tokens.append(&mut right_tokens);
                    restore_split_tokens(&mut left_tokens);
                    DiscreteContentResult::One(Self::Tokens(left_tokens))
                }
                Self::MetadataOnly => {
                    DiscreteContentResult::Two(Self::Tokens(left_tokens), Self::MetadataOnly)
                }
            },
            Self::MetadataOnly => match value {
                Self::Snippet(right_snippet) => {
                    DiscreteContentResult::Two(Self::MetadataOnly, Self::Snippet(right_snippet))
                }
                Self::Tokens(right_tokens) => {
                    DiscreteContentResult::Two(Self::MetadataOnly, Self::Tokens(right_tokens))
                }
                Self::MetadataOnly => DiscreteContentResult::One(Self::MetadataOnly),
            },
        }
    }
    pub fn force_merge(self, value: Self) -> Self {
        match self {
            Self::Snippet(mut left_snippet) => match value {
                Self::Snippet(mut right_snippet) => {
                    left_snippet.append(&mut right_snippet);
                    Self::Snippet(left_snippet)
                }
                Self::Tokens(right_tokens) => {
                    left_snippet.extend(right_tokens.into_iter().flat_map(|t| t.bytes));
                    Self::Snippet(left_snippet)
                }
                Self::MetadataOnly => Self::Snippet(left_snippet),
            },
            Self::Tokens(mut left_tokens) => match value {
                Self::Snippet(mut right_snippet) => {
                    let mut content = Vec::with_capacity(
                        left_tokens.iter().map(|t| t.bytes.len()).sum::<usize>()
                            + right_snippet.len(),
                    );
                    content.extend(left_tokens.into_iter().flat_map(|t| t.bytes));
                    content.append(&mut right_snippet);
                    Self::Snippet(content)
                }
                Self::Tokens(mut right_tokens) => {
                    left_tokens.append(&mut right_tokens);
                    restore_split_tokens(&mut left_tokens);
                    Self::Tokens(left_tokens)
                }
                Self::MetadataOnly => Self::Tokens(left_tokens),
            },
            Self::MetadataOnly => match value {
                Self::Snippet(right_snippet) => Self::Snippet(right_snippet),
                Self::Tokens(right_tokens) => Self::Tokens(right_tokens),
                Self::MetadataOnly => Self::MetadataOnly,
            },
        }
    }
    /// Returns `true` if `self` and `value` should be considered duplicates.
    pub fn is_duplicate_of(&self, value: &Self) -> bool {
        if let Self::Tokens(left) = self
            && let Self::Tokens(right) = value
        {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|(left, right)| left.is_duplicate_of(right))
        } else {
            self == value
        }
    }
    /// Returns `true` if [`Self::merge`] would succeed.
    pub fn is_mergeable_with(&self, value: &Self) -> bool {
        match self {
            Self::Snippet(_) => match value {
                Self::Snippet(_) => true,
                Self::Tokens(_) => false,
                Self::MetadataOnly => false,
            },
            Self::Tokens(_) => match value {
                Self::Snippet(_) => false,
                Self::Tokens(_) => true,
                Self::MetadataOnly => false,
            },
            Self::MetadataOnly => match value {
                Self::Snippet(_) => false,
                Self::Tokens(_) => false,
                Self::MetadataOnly => true,
            },
        }
    }
    pub fn as_bytes(&'_ self) -> Cow<'_, Vec<u8>> {
        match self {
            Self::Snippet(snippet) => Cow::Borrowed(snippet),
            Self::Tokens(tokens) => Cow::Owned(
                tokens
                    .iter()
                    .flat_map(|token| token.bytes.iter().copied())
                    .collect(),
            ),
            Self::MetadataOnly => Cow::Borrowed(EMPTY_VEC_REF),
        }
    }
    pub fn iter_bytes(&self) -> Box<dyn Iterator<Item = u8> + '_> {
        match self {
            Self::Snippet(snippet) => Box::new(snippet.iter().copied()),
            Self::Tokens(tokens) => {
                Box::new(tokens.iter().flat_map(|token| token.bytes.iter().copied()))
            }
            Self::MetadataOnly => Box::new(iter::empty()),
        }
    }
    pub fn len(&self) -> usize {
        match self {
            Self::Snippet(snippet) => snippet.len(),
            Self::Tokens(tokens) => tokens.iter().map(|token| token.bytes.len()).sum(),
            Self::MetadataOnly => 0,
        }
    }
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Snippet(snippet) => snippet.is_empty(),
            Self::Tokens(tokens) => tokens.iter().all(|token| token.bytes.is_empty()),
            Self::MetadataOnly => true,
        }
    }
}

fn restore_split_tokens(tokens: &mut Vec<InnerNodeToken>) {
    let mut index = 0;

    while index < tokens.len() {
        let current = &tokens[index];

        if let OriginalToken::Known {
            bytes: original_bytes,
            id: original_id,
            offset: original_offset,
        } = &current.original
            && original_bytes.ends_with(&current.bytes)
            && *original_offset == original_bytes.len() - current.bytes.len()
        {
            let orig_index = index;
            let mut remaining = *original_offset;

            while remaining != 0 && index != 0 {
                let prev = &tokens[index - 1];

                if let OriginalToken::Known {
                    bytes: prev_bytes,
                    id: prev_id,
                    offset: prev_offset,
                } = &prev.original
                    && prev_bytes == original_bytes
                    && prev_id == original_id
                    && original_bytes[..remaining].ends_with(&prev.bytes)
                    && *prev_offset == remaining - prev.bytes.len()
                    && prev.logprob.map(f32::to_bits) == current.logprob.map(f32::to_bits)
                    && prev.id == current.id
                    && prev.entropy.map(f32::to_bits) == current.entropy.map(f32::to_bits)
                    && prev.counterfactual.len() == current.counterfactual.len()
                    && prev
                        .counterfactual
                        .iter()
                        .zip(current.counterfactual.iter())
                        .all(|(left, right)| {
                            left.bytes == right.bytes
                                && left.id == right.id
                                && left.logprob.map(f32::to_bits) == right.logprob.map(f32::to_bits)
                        })
                {
                    remaining -= prev.bytes.len();
                    index -= 1;
                } else {
                    break;
                }
            }

            if remaining == 0 {
                let (bytes, id) = (original_bytes.clone(), *original_id);
                let current = &mut tokens[orig_index];

                current.original = OriginalToken::Unmodified;
                current.bytes = bytes;
                current.id = id;

                tokens.drain(index..orig_index);
            } else {
                index = orig_index;
            }
        }

        index += 1;
    }
}

impl ArchivedInnerNodeContent {
    /// Returns `true` if [`InnerNodeContent::merge`] would succeed.
    pub fn is_mergeable_with(&self, value: &Self) -> bool {
        match self {
            Self::Snippet(_) => match value {
                Self::Snippet(_) => true,
                Self::Tokens(_) => false,
                Self::MetadataOnly => false,
            },
            Self::Tokens(_) => match value {
                Self::Snippet(_) => false,
                Self::Tokens(_) => true,
                Self::MetadataOnly => false,
            },
            Self::MetadataOnly => match value {
                Self::Snippet(_) => false,
                Self::Tokens(_) => false,
                Self::MetadataOnly => true,
            },
        }
    }
    pub fn as_bytes(&'_ self) -> Vec<u8> {
        match self {
            Self::Snippet(snippet) => snippet.to_vec(),
            Self::Tokens(tokens) => tokens
                .iter()
                .flat_map(|token| token.bytes.iter().copied())
                .collect(),
            Self::MetadataOnly => Vec::new(),
        }
    }
    pub fn len(&self) -> usize {
        match self {
            Self::Snippet(snippet) => snippet.len(),
            Self::Tokens(tokens) => tokens.iter().map(|token| token.bytes.len()).sum(),
            Self::MetadataOnly => 0,
        }
    }
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Snippet(snippet) => snippet.is_empty(),
            Self::Tokens(tokens) => tokens.iter().all(|token| token.bytes.is_empty()),
            Self::MetadataOnly => true,
        }
    }
}

/// The entity which produced an [`InnerNodeContent`]'s value.
#[derive(
    SerdeSerialize, SerdeDeserialize, Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq,
)]
pub enum Creator {
    /// The content was produced by a generative model *which is not the user*.
    Model(Option<Model>),
    /// The content was produced by the user.
    User(Option<Author>),
    /// It is unknown or uncertain what type of entity produced the content.
    Unknown,
}

impl Creator {
    pub fn label(&self) -> Option<&String> {
        match self {
            Self::Model(Some(model)) => Some(&model.label),
            Self::User(Some(user)) => Some(&user.label),
            _ => None,
        }
    }
    pub fn color(&self) -> Option<&String> {
        match self {
            Self::Model(Some(model)) => model.color.as_ref(),
            Self::User(Some(user)) => user.color.as_ref(),
            _ => None,
        }
    }
    pub fn identifier(&self) -> Option<LongId> {
        match self {
            Self::Model(Some(model)) => model.identifier,
            Self::User(Some(user)) => user.identifier,
            _ => None,
        }
    }
    pub fn is_model(&self) -> bool {
        matches!(self, Self::Model(_))
    }
    pub fn is_user(&self) -> bool {
        matches!(self, Self::User(_))
    }
    pub fn as_model(&self) -> Option<&Option<Model>> {
        if let Self::Model(model) = self {
            Some(model)
        } else {
            None
        }
    }
    pub fn as_user(&self) -> Option<&Option<Author>> {
        if let Self::User(user) = self {
            Some(user)
        } else {
            None
        }
    }
    /// Corrects empty or malformed fields
    pub fn normalize(&mut self) {
        match self {
            Self::Model(Some(model)) => model.normalize(),
            Self::User(Some(user)) => user.normalize(),
            _ => {}
        }
    }
    /// Returns `true` if `self` and `value` should be considered duplicates.
    pub fn is_duplicate_of(&self, value: &Self) -> bool {
        match self {
            Self::Model(Some(left)) => {
                if let Self::Model(Some(right)) = value {
                    left.is_duplicate_of(right)
                } else {
                    false
                }
            }
            Self::User(Some(left)) => {
                if let Self::User(Some(right)) = value {
                    left.is_duplicate_of(right)
                } else {
                    false
                }
            }
            _ => self == value,
        }
    }
    /// Returns `true` if [`Self::merge`] would succeed.
    pub fn is_mergeable_with(&self, value: &Self) -> bool {
        match self {
            Self::Model(Some(left)) => {
                if let Self::Model(Some(right)) = value {
                    left.is_mergeable_with(right)
                } else {
                    false
                }
            }
            Self::User(Some(left)) => {
                if let Self::User(Some(right)) = value {
                    left.is_mergeable_with(right)
                } else {
                    false
                }
            }
            _ => self == value,
        }
    }
    /// Merges two items together.
    ///
    /// If merging the two items fails, the original items are returned in the order they were specified.
    #[allow(clippy::result_large_err)]
    pub fn merge(self, value: Self) -> Result<Self, (Self, Self)> {
        match self {
            Self::Model(Some(left)) => {
                if let Self::Model(Some(right)) = value {
                    match left.merge(right) {
                        Ok(combined) => Ok(Self::Model(Some(combined))),
                        Err((left, right)) => {
                            Err((Self::Model(Some(left)), Self::Model(Some(right))))
                        }
                    }
                } else {
                    Err((Self::Model(Some(left)), value))
                }
            }
            Self::User(Some(left)) => {
                if let Self::User(Some(right)) = value {
                    match left.merge(right) {
                        Ok(combined) => Ok(Self::User(Some(combined))),
                        Err((left, right)) => {
                            Err((Self::User(Some(left)), Self::User(Some(right))))
                        }
                    }
                } else {
                    Err((Self::User(Some(left)), value))
                }
            }
            _ => {
                if self == value {
                    Ok(self)
                } else {
                    Err((self, value))
                }
            }
        }
    }
}

impl ArchivedCreator {
    pub fn is_model(&self) -> bool {
        matches!(self, Self::Model(_))
    }
    pub fn is_user(&self) -> bool {
        matches!(self, Self::User(_))
    }
    pub fn as_model(&self) -> Option<&ArchivedOption<ArchivedModel>> {
        if let Self::Model(model) = self {
            Some(model)
        } else {
            None
        }
    }
    pub fn as_user(&self) -> Option<&ArchivedOption<ArchivedAuthor>> {
        if let Self::User(user) = self {
            Some(user)
        } else {
            None
        }
    }
}

/// A label used to represent an unknown model name.
pub const UNKNOWN_MODEL_LABEL: &str = "Unknown Model";

/// Information about a generative model which produced an [`InnerNodeContent`]'s value.
///
/// *This should not be used to represent a user, regardless of their identity.*
#[derive(
    SerdeSerialize, SerdeDeserialize, Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq,
)]
pub struct Model {
    /// A name associated with the model.
    ///
    /// If the model's name is unknown, this should be set to [`UNKNOWN_MODEL_LABEL`].
    pub label: String,
    /// An optional color associated with the model.
    pub color: Option<String>,

    /// A unique identifier for the model.
    #[rkyv(with = NicheInto<niching::Zero>)]
    pub identifier: Option<LongId>,

    /// The seed used to generate the content.
    pub seed: Option<u64>,
    /// A string which identifies the backend configuration used to generate the content.
    pub system_fingerprint: Option<String>,
    /// The reason the content finished being generated.
    pub finish_reason: Option<String>,

    /// Additional user-readable information about the model used to generate the content.
    ///
    /// For example, this could be used to store:
    /// - Hugging Face repo_id
    /// - Quantization metadata
    /// - Backend type
    /// - Request template (such as a prefix for doing "fake-base" ChatCompletions requests)
    ///
    /// **This should not contain sensitive information**, such as endpoint URLs or API keys, as documents may be shared publicly.
    #[rkyv(with = IAsVec)]
    pub metadata: MetadataMap,
}

impl Model {
    /// Corrects empty or malformed fields
    pub fn normalize(&mut self) {
        /*if self.label.is_empty() {
            self.label = UNKNOWN_MODEL_LABEL.to_string();
        }*/

        if self.color.as_ref().is_some_and(|color| color.is_empty()) {
            self.color = None;
        }

        if self
            .system_fingerprint
            .as_ref()
            .is_some_and(|fingerprint| fingerprint.is_empty())
        {
            self.system_fingerprint = None;
        }

        if self
            .finish_reason
            .as_ref()
            .is_some_and(|finish_reason| finish_reason.is_empty())
        {
            self.finish_reason = None;
        }
    }
    /// Returns `true` if `self` and `value` should be considered duplicates.
    pub fn is_duplicate_of(&self, value: &Self) -> bool {
        ((self.identifier.is_some()
            && value.identifier.is_some()
            && self.identifier == value.identifier)
            || (self.identifier.is_none()
                && value.identifier.is_none()
                && self.label == value.label))
            && self.seed == value.seed
            && self.finish_reason == value.finish_reason
            && self.metadata == value.metadata
    }
    /// Returns `true` if [`Self::merge`] would succeed.
    pub fn is_mergeable_with(&self, value: &Self) -> bool {
        self.label == value.label
            && (self.color == value.color || self.color.is_none() || value.color.is_none())
            && self.identifier == value.identifier
            && self.metadata == value.metadata
    }
    /// Merges two items together.
    ///
    /// If merging the two items fails, the original items are returned in the order they were specified.
    #[allow(clippy::result_large_err)]
    pub fn merge(mut self, mut value: Self) -> Result<Self, (Self, Self)> {
        if self.label == value.label
            && self.identifier == value.identifier
            && self.metadata == value.metadata
        {
            if self.color == value.color || value.color.is_none() {
                if self.seed != value.seed {
                    self.seed = None;
                }
                if self.system_fingerprint != value.system_fingerprint {
                    self.system_fingerprint = None;
                }
                self.finish_reason = value.finish_reason;
                Ok(self)
            } else if self.color.is_none() {
                if self.seed != value.seed {
                    value.seed = None;
                }
                if self.system_fingerprint != value.system_fingerprint {
                    value.system_fingerprint = None;
                }
                Ok(value)
            } else {
                Err((self, value))
            }
        } else {
            Err((self, value))
        }
    }
}

/// Information about a user which produced an [`InnerNodeContent`]'s value.
#[derive(
    SerdeSerialize, SerdeDeserialize, Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq,
)]
pub struct Author {
    /// The user's preferred name.
    pub label: String,
    /// An optional color associated with the user.
    pub color: Option<String>,

    /// A unique identifier for the user.
    #[rkyv(with = NicheInto<niching::Zero>)]
    pub identifier: Option<LongId>,

    /// Additional user-readable information about the user.
    ///
    /// For example, this could be used to store:
    /// - Usernames
    /// - Pronoun preferences
    /// - Alternative names / plural system metadata
    ///
    /// **This should not contain sensitive information**, such as email addresses or legal names, as documents may be shared publicly.
    #[rkyv(with = IAsVec)]
    pub metadata: MetadataMap,
}

impl Author {
    /// Corrects empty or malformed fields
    pub fn normalize(&mut self) {
        if self.color.as_ref().is_some_and(|color| color.is_empty()) {
            self.color = None;
        }
    }
    /// Returns `true` if `self` and `value` should be considered duplicates.
    pub fn is_duplicate_of(&self, value: &Self) -> bool {
        ((self.identifier.is_some()
            && value.identifier.is_some()
            && self.identifier == value.identifier)
            || (self.identifier.is_none()
                && value.identifier.is_none()
                && self.label == value.label))
            && self.metadata == value.metadata
    }
    /// Returns `true` if [`Self::merge`] would succeed.
    pub fn is_mergeable_with(&self, value: &Self) -> bool {
        self.label == value.label
            && (self.color == value.color || self.color.is_none() || value.color.is_none())
            && self.identifier == value.identifier
            && self.metadata == value.metadata
    }
    /// Merges two items together.
    ///
    /// If merging the two items fails, the original items are returned in the order they were specified.
    #[allow(clippy::result_large_err)]
    pub fn merge(self, value: Self) -> Result<Self, (Self, Self)> {
        if self.label == value.label
            && self.identifier == value.identifier
            && self.metadata == value.metadata
        {
            if self.color == value.color || value.color.is_none() {
                Ok(self)
            } else if self.color.is_none() {
                Ok(value)
            } else {
                Err((self, value))
            }
        } else {
            Err((self, value))
        }
    }
}

#[inline]
/// Normalizes infinite and NaN values to None and Some(-0.0) to Some(0.0).
pub fn normalize_f32(item: Option<f32>) -> Option<f32> {
    item.filter(|item| item.is_finite()).map(|item| item + 0.0)
}

#[inline]
/// Normalizes infinite and NaN values to None and Some(-0.0) to Some(0.0).
pub fn normalize_f64(item: Option<f64>) -> Option<f64> {
    item.filter(|item| item.is_finite()).map(|item| item + 0.0)
}

/// Modified version of String::from_utf8_lossy() which uses [`SUBSTITUTION_CHARACTER`].
///
/// Because [`SUBSTITUTION_CHARACTER`] is 1 byte long and is emitted for every invalid byte, the converted string always has the same length as the input bytes.
#[inline]
pub fn from_utf8_lossy(v: &[u8]) -> Cow<'_, str> {
    let mut iter = v.utf8_chunks();

    let (first_valid, first_invalid) = if let Some(chunk) = iter.next() {
        let valid = chunk.valid();
        let invalid = chunk.invalid();
        if invalid.is_empty() {
            return Cow::Borrowed(valid);
        }
        (valid, invalid)
    } else {
        return Cow::Borrowed("");
    };

    let mut res = String::with_capacity(v.len());
    res.push_str(first_valid);
    for _ in first_invalid {
        res.push_str(SUBSTITUTION_CHARACTER);
    }

    for chunk in iter {
        res.push_str(chunk.valid());
        for _ in chunk.invalid() {
            res.push_str(SUBSTITUTION_CHARACTER);
        }
    }

    debug_assert_eq!(v.len(), res.len());

    Cow::Owned(res)
}
