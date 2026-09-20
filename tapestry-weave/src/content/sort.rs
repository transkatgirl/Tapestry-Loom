use std::cmp::Ordering;

use crate::TapestryNode;

pub fn by_model(a: &TapestryNode, b: &TapestryNode) -> Ordering {
    a.contents.creator.label().cmp(&b.contents.creator.label())
}

pub fn by_confidence(a: &TapestryNode, b: &TapestryNode) -> Ordering {
    b.contents
        .content
        .calculate_confidence()
        .map(|(confidence, _, _)| confidence)
        .filter(|confidence| confidence.is_finite())
        .unwrap_or(f32::NEG_INFINITY)
        .total_cmp(
            &a.contents
                .content
                .calculate_confidence()
                .map(|(confidence, _, _)| confidence)
                .filter(|confidence| confidence.is_finite())
                .unwrap_or(f32::NEG_INFINITY),
        )
}

pub fn by_single_token(a: &TapestryNode, b: &TapestryNode) -> Ordering {
    let a_single_token = a.contents.content.token_count() == Some(1)
        && a.contents.content.contains_modified_tokens() == Some(false);
    let b_single_token = b.contents.content.token_count() == Some(1)
        && b.contents.content.contains_modified_tokens() == Some(false);

    if a_single_token && b_single_token {
        b.contents
            .content
            .calculate_average_logprob()
            .filter(|logprob| logprob.is_finite())
            .unwrap_or(f32::NEG_INFINITY)
            .total_cmp(
                &a.contents
                    .content
                    .calculate_average_logprob()
                    .filter(|logprob| logprob.is_finite())
                    .unwrap_or(f32::NEG_INFINITY),
            )
    } else {
        b_single_token.cmp(&a_single_token)
    }
}

pub fn by_timestamp(a: &TapestryNode, b: &TapestryNode) -> Ordering {
    a.contents
        .timestamp
        .cmp(&b.contents.timestamp)
        .then_with(|| a.id.cmp(&b.id))
}

pub fn grouped(a: &TapestryNode, b: &TapestryNode) -> Ordering {
    by_model(a, b)
        .then_with(|| by_single_token(a, b))
        .then_with(|| by_timestamp(a, b))
}
