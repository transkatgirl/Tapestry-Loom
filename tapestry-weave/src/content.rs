use std::{borrow::Cow, num::NonZeroU128};

use jiff::Zoned;
use universal_weave::{
    DeduplicatableContents, DiscreteContentResult, DiscreteContents, IndependentContents,
    rkyv::{
        Archive, Deserialize, Serialize, niche::niching, option::ArchivedOption, with::NicheInto,
    },
};

#[cfg(feature = "serde")]
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

#[cfg(feature = "serde")]
use super::wrappers::Base64Standard;

use super::{
    metadata::{AuxMetadataMap, MetadataMap},
    wrappers::{AsBinaryZoned, IAsVec},
};

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub struct NodeContent {
    #[rkyv(with = AsBinaryZoned)]
    pub timestamp: Zoned,
    pub modified: bool,

    pub content: InnerNodeContent,

    #[rkyv(with = IAsVec)]
    pub metadata: MetadataMap,

    #[rkyv(with = IAsVec)]
    pub aux_metadata: AuxMetadataMap,

    pub creator: Creator,
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
                    aux_metadata: self.aux_metadata.clone(),
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
        if self.timestamp.time_zone() != value.timestamp.time_zone()
            || self.metadata != value.metadata
            || !self.creator.is_mergeable_with(&value.creator)
        {
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
                if self.aux_metadata != value.aux_metadata {
                    self.aux_metadata.clear();
                }
                DiscreteContentResult::One(self)
            }
        }
    }
}

impl NodeContent {
    pub fn is_mergeable_with(&self, value: &Self) -> bool {
        self.timestamp.time_zone() == value.timestamp.time_zone()
            && self.metadata == value.metadata
            && self.creator.is_mergeable_with(&value.creator)
            && self.content.is_mergeable_with(&value.content)
    }
}

impl DeduplicatableContents for NodeContent {
    fn is_duplicate_of(&self, value: &Self) -> bool {
        self.modified == value.modified
            && self.metadata == value.metadata
            && self.content.is_duplicate_of(&value.content)
            && self.creator.is_duplicate_of(&value.creator)
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub enum InnerNodeContent {
    Snippet(#[cfg_attr(feature = "serde", serde(with = "Base64Standard"))] Vec<u8>),
    Tokens(Vec<InnerNodeToken>),
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
    pub fn calculate_average_logprob(&self) -> Option<f32> {
        if let Self::Tokens(tokens) = self
            && !tokens.is_empty()
        {
            Some(
                (tokens.iter().map(|token| token.logprob as f64).sum::<f64>() / tokens.len() as f64)
                    as f32,
            )
        } else {
            None
        }
    }
    pub fn calculate_cumulative_logprob(&self) -> Option<f32> {
        if let Self::Tokens(tokens) = self
            && !tokens.is_empty()
        {
            Some(tokens.iter().map(|token| token.logprob as f64).sum::<f64>() as f32)
        } else {
            None
        }
    }
    pub fn calculate_confidence(&self) -> Option<(f32, usize, usize)> {
        if let Self::Tokens(tokens) = self {
            let mut confidence_sum = 0.0;
            let mut confidence_k = None;

            for token in tokens {
                if let Some((confidence, k)) = token.calculate_confidence_f64() {
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

            confidence_k.map(|confidence_k| {
                (
                    (confidence_sum / tokens.len() as f64) as f32,
                    confidence_k,
                    tokens.len(),
                )
            })
        } else {
            None
        }
    }
    pub fn calculate_average_entropy(&self) -> Option<f32> {
        if let Self::Tokens(tokens) = self
            && !tokens.is_empty()
        {
            let mut entropy_sum = 0.0;

            for token in tokens {
                let entropy = token.entropy?;
                entropy_sum += entropy as f64;
            }

            Some((entropy_sum / tokens.len() as f64) as f32)
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
            Some(
                (tokens
                    .iter()
                    .map(|token| token.logprob.to_native() as f64)
                    .sum::<f64>()
                    / tokens.len() as f64) as f32,
            )
        } else {
            None
        }
    }
    pub fn calculate_cumulative_logprob(&self) -> Option<f32> {
        if let Self::Tokens(tokens) = self
            && !tokens.is_empty()
        {
            Some(
                tokens
                    .iter()
                    .map(|token| token.logprob.to_native() as f64)
                    .sum::<f64>() as f32,
            )
        } else {
            None
        }
    }
    pub fn calculate_confidence(&self) -> Option<(f32, usize, usize)> {
        if let Self::Tokens(tokens) = self {
            let mut confidence_sum = 0.0;
            let mut confidence_k = None;

            for token in tokens.iter() {
                if let Some((confidence, k)) = token.calculate_confidence_f64() {
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

            confidence_k.map(|confidence_k| {
                (
                    (confidence_sum / tokens.len() as f64) as f32,
                    confidence_k,
                    tokens.len(),
                )
            })
        } else {
            None
        }
    }
    pub fn calculate_average_entropy(&self) -> Option<f32> {
        if let Self::Tokens(tokens) = self {
            let (count, sum) = tokens
                .iter()
                .filter_map(|token| token.entropy.as_ref().map(|e| e.to_native() as f64))
                .fold((0usize, 0.0), |acc, x| (acc.0 + 1, acc.1 + x));

            if count > 0 {
                Some((sum / count as f64) as f32)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub struct InnerNodeToken {
    #[cfg_attr(feature = "serde", serde(with = "Base64Standard"))]
    pub bytes: Vec<u8>,
    pub logprob: f32,
    pub id: Option<u64>,

    #[rkyv(with = IAsVec)]
    pub metadata: MetadataMap,

    #[rkyv(with = NicheInto<niching::NaN>)]
    pub entropy: Option<f32>,
    pub counterfactual: Vec<CounterfactualToken>,

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
            metadata: token.metadata,
            entropy: None,
            counterfactual,
            original: OriginalToken::Unmodified,
        }
    }
    pub fn sort_counterfactual(&mut self) {
        self.counterfactual
            .sort_unstable_by(|a, b| b.logprob.total_cmp(&a.logprob));
    }
    pub fn truncate_counterfactual(&mut self, len: usize) {
        self.counterfactual.truncate(len);
        self.counterfactual.shrink_to_fit();
    }
    pub fn calculate_confidence(&self) -> Option<(f32, usize)> {
        self.calculate_confidence_f64()
            .map(|(confidence, k)| (confidence as f32, k))
    }
    fn calculate_confidence_f64(&self) -> Option<(f64, usize)> {
        if !self.counterfactual.is_empty() {
            Some((
                self.counterfactual
                    .iter()
                    .map(|token| token.logprob as f64)
                    .sum::<f64>()
                    / -(self.counterfactual.len() as f64),
                self.counterfactual.len(),
            ))
        } else {
            None
        }
    }
    pub fn is_modified(&self) -> bool {
        self.original.is_modified()
    }
    pub fn is_duplicate_of(&self, value: &Self) -> bool {
        self.bytes == value.bytes
            && self.id == value.id
            && self.metadata == value.metadata
            && self.entropy.is_some() == value.entropy.is_some()
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
        self.calculate_confidence_f64()
            .map(|(confidence, k)| (confidence as f32, k))
    }
    fn calculate_confidence_f64(&self) -> Option<(f64, usize)> {
        if !self.counterfactual.is_empty() {
            Some((
                self.counterfactual
                    .iter()
                    .map(|token| token.logprob.to_native() as f64)
                    .sum::<f64>()
                    / -(self.counterfactual.len() as f64),
                self.counterfactual.len(),
            ))
        } else {
            None
        }
    }
    pub fn is_modified(&self) -> bool {
        self.original.is_modified()
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub enum OriginalToken {
    Unmodified,
    Known(#[cfg_attr(feature = "serde", serde(with = "Base64Standard"))] Vec<u8>),
    Unknown, // Necessary for backwards compatibility with v0 format
}

impl OriginalToken {
    pub fn is_modified(&self) -> bool {
        match self {
            Self::Known(_) => true,
            Self::Unknown => true,
            Self::Unmodified => false,
        }
    }
}

impl ArchivedOriginalToken {
    pub fn is_modified(&self) -> bool {
        match self {
            Self::Known(_) => true,
            Self::Unknown => true,
            Self::Unmodified => false,
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub struct CounterfactualToken {
    #[cfg_attr(feature = "serde", serde(with = "Base64Standard"))]
    pub bytes: Vec<u8>,
    pub logprob: f32,
    pub id: Option<u64>,

    #[rkyv(with = IAsVec)]
    pub metadata: MetadataMap,
}

impl CounterfactualToken {
    pub fn calculate_entropy<'a>(tokens: impl Iterator<Item = &'a CounterfactualToken>) -> f64 {
        -tokens
            .map(|token| (token.logprob as f64).exp() * (token.logprob as f64))
            .sum::<f64>()
    }
    pub fn is_duplicate_of(&self, value: &Self) -> bool {
        self.bytes == value.bytes && self.id == value.id && self.metadata == value.metadata
    }
}

const EMPTY_VEC_REF: &Vec<u8> = &Vec::new();

impl InnerNodeContent {
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
                            right[0].original = OriginalToken::Known(right[0].bytes.clone());
                        }

                        left_token.shrink_to_fit();
                        left.push(InnerNodeToken {
                            bytes: left_token,
                            id: None,
                            entropy: right[0].entropy,
                            logprob: right[0].logprob,
                            metadata: right[0].metadata.clone(),
                            counterfactual: right[0].counterfactual.clone(),
                            original: right[0].original.clone(),
                        });
                        right[0].id = None;
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
                    .flat_map(|token| token.bytes.clone())
                    .collect(),
            ),
            Self::MetadataOnly => Cow::Borrowed(EMPTY_VEC_REF),
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

impl ArchivedInnerNodeContent {
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
            Self::MetadataOnly => false,
        }
    }
    pub fn as_bytes(&'_ self) -> Vec<u8> {
        match self {
            Self::Snippet(snippet) => snippet.to_vec(),
            Self::Tokens(tokens) => tokens
                .iter()
                .flat_map(|token| token.bytes.to_vec())
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

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub enum Creator {
    Model(Option<Model>),
    User(Option<Author>),
    Unknown, // Necessary for backwards compatibility with v0 format
}

impl Creator {
    pub fn label(&self) -> Option<&String> {
        match &self {
            Self::Model(Some(model)) => Some(&model.label),
            Self::User(Some(user)) => Some(&user.label),
            _ => None,
        }
    }
    pub fn color(&self) -> Option<&String> {
        match &self {
            Self::Model(Some(model)) => model.color.as_ref(),
            Self::User(Some(user)) => user.color.as_ref(),
            _ => None,
        }
    }
    pub fn identifier(&self) -> Option<NonZeroU128> {
        match &self {
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

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub struct Model {
    pub label: String,
    pub color: Option<String>,

    #[rkyv(with = NicheInto<niching::Zero>)]
    pub identifier: Option<NonZeroU128>,

    pub seed: Option<u32>,
    pub system_fingerprint: Option<String>,
    pub finish_reason: Option<String>,

    #[rkyv(with = IAsVec)]
    pub metadata: MetadataMap,
}

impl Model {
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
    pub fn is_mergeable_with(&self, value: &Self) -> bool {
        self.label == value.label
            && (self.color == value.color || self.color.is_none() || value.color.is_none())
            && self.identifier == value.identifier
            && self.metadata == value.metadata
    }
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
                if self.finish_reason != value.finish_reason {
                    self.finish_reason = None;
                }
                Ok(self)
            } else if self.color.is_none() {
                if self.seed != value.seed {
                    value.seed = None;
                }
                if self.system_fingerprint != value.system_fingerprint {
                    value.system_fingerprint = None;
                }
                if self.finish_reason != value.finish_reason {
                    value.finish_reason = None;
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

pub const UNKNOWN_MODEL_LABEL: &str = "Unknown Model";

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub struct Author {
    pub label: String,
    pub color: Option<String>,

    #[rkyv(with = NicheInto<niching::Zero>)]
    pub identifier: Option<NonZeroU128>,

    #[rkyv(with = IAsVec)]
    pub metadata: MetadataMap,
}

impl Author {
    pub fn is_duplicate_of(&self, value: &Self) -> bool {
        ((self.identifier.is_some()
            && value.identifier.is_some()
            && self.identifier == value.identifier)
            || (self.identifier.is_none()
                && value.identifier.is_none()
                && self.label == value.label))
            && self.metadata == value.metadata
    }
    pub fn is_mergeable_with(&self, value: &Self) -> bool {
        self.label == value.label
            && (self.color == value.color || self.color.is_none() || value.color.is_none())
            && self.identifier == value.identifier
            && self.metadata == value.metadata
    }
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
