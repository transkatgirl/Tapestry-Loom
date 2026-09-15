#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::{collections::HashSet, hash::BuildHasherDefault};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use stacksafe::stacksafe;
use tapestry_weave::{
    TapestryNode, TapestryWeave,
    content::{
        CounterfactualToken, Creator, InnerNodeContent, InnerNodeToken, Model, NodeContent,
        OriginalToken,
    },
    hashers::{RandomIdHasher, RandomState},
    jiff::{Zoned, civil::DateTime, tz::TimeZone},
    nanorand::{Rng, WyRand},
    universal_weave::{
        MetadataWeave, Weave,
        indexmap::{IndexMap, IndexSet},
    },
    wrappers::UniqueIdentifierRemapper,
};

use crate::new_weave;

pub fn migrate(input: &str, created: Zoned) -> anyhow::Result<Option<TapestryWeave>> {
    let Ok(value) = serde_json::from_str::<Value>(input) else {
        return Ok(None);
    };

    let data = if value.is_array() {
        serde_json::from_value::<Vec<PyloomNode>>(value).map(|children| {
            PyloomWeave::from_root(PyloomNode {
                children,
                ..PyloomNode::default()
            })
        })
    } else if value.get("root").is_some() {
        serde_json::from_value::<PyloomWeave>(value)
    } else if value.get("text").is_some() {
        serde_json::from_value::<PyloomNode>(value).map(PyloomWeave::from_root)
    } else {
        return Ok(None);
    };

    let Ok(mut data) = data else {
        return Ok(None);
    };

    let root = std::mem::take(&mut data.root);
    data.root = unzip_masks(root, &mut data.selected_node_id);

    let node_count = assign_missing_identifiers(&mut data.root, &mut HashSet::new(), &mut 0);

    let selected = data
        .selected_node_id
        .or_else(|| data.root.children.first().map(|child| child.id.clone()))
        .unwrap_or_default();

    let chapters: IndexMap<String, String> = data
        .chapters
        .into_iter()
        .map(|(id, chapter)| (id, chapter.title))
        .collect();

    let mut hidden_tags = vec!["archived".to_string(), "note".to_string()];

    for (name, definition) in &data.tags {
        match definition.get("hide").and_then(Value::as_bool) {
            Some(true) if !hidden_tags.contains(name) => hidden_tags.push(name.clone()),
            Some(false) => hidden_tags.retain(|tag| tag != name),
            _ => {}
        }
    }

    let mut output = new_weave(node_count, created, "PyLoom", None);

    let context = Context {
        chapters,
        responses: data.model_responses,
        hidden_tags,
        canonical: data.canonical.into_iter().collect(),
        time_zone: output.metadata().created.time_zone().clone(),
    };

    let mut mapper: UniqueIdentifierRemapper<
        String,
        u64,
        RandomState,
        BuildHasherDefault<RandomIdHasher>,
    > = UniqueIdentifierRemapper::with_capacity(node_count);

    let mut rng = WyRand::new();

    let mut convert_old_identifier = move |id| *mapper.map(id, || rng.generate()).get();

    convert_node(
        &mut output,
        &mut convert_old_identifier,
        data.root,
        None,
        &context,
    )?;

    if !selected.is_empty() {
        let selected = convert_old_identifier(selected);
        if output.contains(&selected) {
            output.set_active_tree_semantics(&selected, true);
        }
    }

    Ok(Some(output))
}

struct Context {
    chapters: IndexMap<String, String>,
    responses: IndexMap<String, PyloomModelResponse>,
    hidden_tags: Vec<String>,
    canonical: HashSet<String>,
    time_zone: TimeZone,
}

#[stacksafe]
fn assign_missing_identifiers(
    node: &mut PyloomNode,
    seen: &mut HashSet<String>,
    counter: &mut usize,
) -> usize {
    if node.id.is_empty() {
        node.id = generated_identifier(counter);
    } else if seen.contains(&node.id) {
        eprintln!(
            "Warning: Duplicate node id {:?}; assigning a new identifier",
            node.id
        );

        node.id = generated_identifier(counter);
    }

    seen.insert(node.id.clone());

    let mut count = 1;

    for child in &mut node.children {
        count += assign_missing_identifiers(child, seen, counter);
    }

    count
}

fn generated_identifier(counter: &mut usize) -> String {
    let id = format!("\0generated:{counter}");
    *counter += 1;
    id
}

#[stacksafe]
fn unzip_masks(mut node: PyloomNode, selected: &mut Option<String>) -> PyloomNode {
    node.children = node
        .children
        .into_iter()
        .map(|child| unzip_masks(child, selected))
        .collect();

    let Some(head) = node.masked_head.take() else {
        return node;
    };

    let mut head = unzip_masks(*head, selected);

    let tail = node.tail_id.as_deref().and_then(|tail_id| {
        let mut stack = vec![&mut head];

        while let Some(node) = stack.pop() {
            if node.id == tail_id {
                return Some(node);
            }

            stack.extend(node.children.iter_mut());
        }

        None
    });

    let merged = match tail {
        Some(tail) => {
            tail.children.append(&mut node.children);
            merge_mask_attributes(&mut node, tail);

            if selected.as_deref() == Some(node.id.as_str()) {
                *selected = Some(tail.id.clone());
            }

            true
        }
        None => false,
    };

    match merged {
        true => {
            if head.chapter_id.is_none() {
                head.chapter_id = node.chapter_id.take();
            }

            head
        }
        false => {
            eprintln!(
                "Warning: Missing tail {:?} for node {:?}; keeping masked subtree as a child",
                node.tail_id, node.id
            );

            node.children.push(head);

            node
        }
    }
}

fn merge_mask_attributes(mask: &mut PyloomNode, target: &mut PyloomNode) {
    if let Some(tags) = mask.tags.take() {
        let target_tags = target.tags.get_or_insert_with(Vec::new);

        for tag in tags {
            if !target_tags.contains(&tag) {
                target_tags.push(tag);
            }
        }
    }

    for (flag, target_flag) in [
        (mask.bookmark, &mut target.bookmark),
        (mask.archived, &mut target.archived),
        (mask.canonical, &mut target.canonical),
    ] {
        if flag.unwrap_or_default() {
            *target_flag = Some(true);
        }
    }

    let notes = mask.notes.take().map(parse_notes).unwrap_or_default();

    if !notes.is_empty() {
        let mut target_notes = target.notes.take().map(parse_notes).unwrap_or_default();
        target_notes.extend(notes);
        target.notes = Some(Value::Array(
            target_notes.into_iter().map(Value::String).collect(),
        ));
    }

    if let Some(Value::Array(mut items)) = mask.multimedia.take() {
        match target.multimedia.as_mut() {
            Some(Value::Array(target_items)) => target_items.append(&mut items),
            _ => target.multimedia = Some(Value::Array(items)),
        }
    }

    if let Some(attributes) = mask.text_attributes.take() {
        let target_attributes = target.text_attributes.get_or_insert_with(Default::default);

        if target_attributes.active_append.is_none() {
            target_attributes.active_append = attributes.active_append;
        }

        if target_attributes.child_preview.is_none() {
            target_attributes.child_preview = attributes.child_preview;
        }
    }
}

#[stacksafe]
fn convert_node(
    weave: &mut TapestryWeave,
    convert_old_identifier: &mut impl FnMut(String) -> u64,
    node: PyloomNode,
    parent: Option<u64>,
    context: &Context,
) -> anyhow::Result<()> {
    let response = node
        .generation
        .as_ref()
        .and_then(|generation| context.responses.get(&generation.id));

    let generated = node.generation.is_some()
        || node
            .meta
            .as_ref()
            .is_some_and(|meta| meta.generation.is_some());

    let timestamp = parse_timestamp(
        node.meta
            .as_ref()
            .and_then(|meta| meta.creation_timestamp.as_deref()),
        &context.time_zone,
    )
    .or_else(|| {
        parse_timestamp(
            response.and_then(|response| response.timestamp.as_deref()),
            &context.time_zone,
        )
    })
    .unwrap_or_default();

    let source = node.meta.as_ref().and_then(|meta| meta.source.clone());

    let modified = node
        .meta
        .as_ref()
        .map(|meta| source.as_deref() == Some("mixed") || meta.modified.unwrap_or_default())
        .unwrap_or_default();

    let id = convert_old_identifier(node.id.clone());

    let chapter = node
        .chapter_id
        .and_then(|chapter| context.chapters.get(&chapter));

    let mut tags = node.tags.unwrap_or_default();

    let canonical = node.canonical.unwrap_or_default() || context.canonical.contains(&node.id);

    for (flag, tag) in [
        (node.bookmark.unwrap_or_default(), "bookmark"),
        (node.archived.unwrap_or_default(), "archived"),
        (canonical, "canonical"),
    ] {
        if flag && !tags.iter().any(|t| t == tag) {
            tags.push(tag.to_string());
        }
    }

    let bookmarked = chapter.is_some() || tags.iter().any(|tag| tag == "bookmark");
    let hidden = tags.iter().any(|tag| context.hidden_tags.contains(tag));

    let mut metadata = IndexMap::with_capacity_and_hasher(6, RandomState::default());

    if let Some(attributes) = node.text_attributes {
        if let Some(preview) = attributes.child_preview {
            metadata.insert("child_preview".to_string(), preview);
        }

        if let Some(preview) = attributes.nav_preview {
            metadata.insert("nav_preview".to_string(), preview);
        }

        if let Some(append) = attributes.active_append {
            metadata.insert("active_append".to_string(), append);
        }
    }

    if let Some(chapter) = chapter {
        metadata.insert("chapter".to_string(), chapter.clone());
    }

    if !tags.is_empty() {
        metadata.insert("tags".to_string(), serde_json::to_string(&tags)?);
    }

    if hidden {
        metadata.insert("pruned".to_string(), "true".to_string());
    }

    if let Some(origin) = node.meta.as_ref().and_then(|meta| meta.origin.clone()) {
        metadata.insert("origin".to_string(), origin);
    }

    if node.template.unwrap_or_default() {
        metadata.insert("template".to_string(), "true".to_string());
    }

    let notes = node.notes.map(parse_notes).unwrap_or_default();

    if !notes.is_empty() {
        metadata.insert("notes".to_string(), serde_json::to_string(&notes)?);
    }

    if let Some(multimedia) = node.multimedia
        && multimedia.as_array().is_some_and(|items| !items.is_empty())
    {
        metadata.insert(
            "multimedia".to_string(),
            serde_json::to_string(&multimedia)?,
        );
    }

    let text = node.text;

    let mut model_label = None;
    let mut prompt = None;
    let mut finish_reason = None;
    let mut tokens = None;

    if let Some(generation) = &node.generation {
        if let Some(response) = response {
            model_label = response.model.clone();
            prompt = response
                .prompt
                .as_ref()
                .and_then(|prompt| prompt.text.clone());

            if let Some(completion) = response.completions.get(generation.index) {
                finish_reason = parse_finish_reason(completion.finishReason.as_ref());
                tokens = build_tokens(
                    &text,
                    completion
                        .tokens
                        .iter()
                        .map(|token| {
                            (
                                decode_token(&token.generatedToken.token),
                                token.generatedToken.logprob,
                                parse_counterfactuals(token.counterfactuals.as_ref()),
                                OriginalToken::Unmodified,
                            )
                        })
                        .collect(),
                    true,
                );
            } else {
                eprintln!(
                    "Warning: Node {:?} is missing completion {}",
                    node.id, generation.index
                );
            }
        }
    } else if let Some(legacy) = node.meta.as_ref().and_then(|meta| meta.generation.as_ref()) {
        model_label = legacy.model.clone();
        prompt = legacy.prompt.clone();
        finish_reason = legacy.finish_reason.clone();
        tokens = build_legacy_tokens(&text, legacy);
    }

    if tokens.is_none()
        && generated
        && let Some(meta) = &node.meta
        && let Some(diffs) = &meta.diffs
    {
        tokens = build_diff_tokens(&text, diffs, meta.generation.as_ref());
    }

    let content = match tokens {
        Some(tokens) => InnerNodeContent::Tokens(tokens),
        None => InnerNodeContent::Snippet(text.into_bytes()),
    };

    let model = model_label.map(|label| Model {
        label,
        color: None,
        identifier: None,
        seed: None,
        system_fingerprint: None,
        finish_reason,
        metadata: prompt
            .into_iter()
            .map(|prompt| ("prompt".to_string(), prompt))
            .collect(),
    });

    let creator = match source.as_deref() {
        Some("AI") => Creator::Model(model),
        Some("mixed") | Some("prompt") => Creator::User(None),
        _ if generated => Creator::Model(model),
        _ => Creator::Unknown,
    };

    assert!(weave.insert(TapestryNode {
        id,
        from: IndexSet::from_iter(parent),
        to: IndexSet::default(),
        active: false,
        bookmarked,
        contents: NodeContent {
            timestamp,
            modified,
            content,
            metadata,
            aux_metadata: IndexMap::default(),
            creator,
        },
    }));

    for child in node.children {
        convert_node(weave, convert_old_identifier, child, Some(id), context)?;
    }

    Ok(())
}

fn parse_notes(notes: Value) -> Vec<String> {
    match notes {
        Value::Array(items) => items
            .into_iter()
            .filter_map(|item| match item {
                Value::String(note) => Some(note),
                _ => None,
            })
            .filter(|note| !note.trim().is_empty())
            .collect(),
        Value::String(note) if !note.trim().is_empty() => vec![note],
        _ => Vec::new(),
    }
}

fn parse_timestamp(value: Option<&str>, time_zone: &TimeZone) -> Option<Zoned> {
    DateTime::strptime("%Y-%m-%d-%H.%M.%S", value?)
        .ok()?
        .to_zoned(time_zone.clone())
        .ok()
}

fn decode_token(token: &str) -> Vec<u8> {
    let Some(mut rest) = token.strip_prefix("bytes:") else {
        return token.as_bytes().to_vec();
    };

    let mut output = Vec::with_capacity(rest.len());

    while !rest.is_empty() {
        if let Some(hex) = rest.strip_prefix("\\x")
            && let Some(digits) = hex.get(..2)
            && digits.bytes().all(|byte| byte.is_ascii_hexdigit())
            && let Ok(byte) = u8::from_str_radix(digits, 16)
        {
            output.push(byte);
            rest = &hex[2..];
        } else {
            let char = rest.chars().next().unwrap();
            let mut buffer = [0; 4];
            output.extend_from_slice(char.encode_utf8(&mut buffer).as_bytes());
            rest = &rest[char.len_utf8()..];
        }
    }

    output
}

type TokenRecord = (
    Vec<u8>,
    Option<f64>,
    Vec<CounterfactualToken>,
    OriginalToken,
);

fn build_tokens(
    text: &str,
    tokens: Vec<TokenRecord>,
    partial: bool,
) -> Option<Vec<InnerNodeToken>> {
    let joined: Vec<u8> = tokens
        .iter()
        .flat_map(|(bytes, _, _, _)| bytes.iter().copied())
        .collect();

    if joined.is_empty() {
        return None;
    }

    let bytes = text.as_bytes();

    let start = if partial {
        bytes
            .windows(joined.len())
            .position(|window| window == joined.as_slice())?
    } else if bytes == joined.as_slice() {
        0
    } else {
        return None;
    };

    let end = start + joined.len();

    let mut output = Vec::with_capacity(tokens.len() + 2);

    if start > 0 {
        output.push(unknown_token(bytes[..start].to_vec()));
    }

    for (token_bytes, logprob, counterfactual, original) in tokens {
        let mut token = InnerNodeToken {
            bytes: token_bytes,
            logprob: logprob.map(|p| p as f32).unwrap_or(f32::NAN),
            id: None,
            entropy: None,
            counterfactual,
            original,
        };
        token.sort_counterfactual();

        output.push(token);
    }

    if end < bytes.len() {
        output.push(unknown_token(bytes[end..].to_vec()));
    }

    Some(output)
}

fn unknown_token(bytes: Vec<u8>) -> InnerNodeToken {
    InnerNodeToken {
        bytes,
        logprob: f32::NAN,
        id: None,
        entropy: None,
        counterfactual: Vec::new(),
        original: OriginalToken::Unmodified,
    }
}

type LegacyRecord<'a> = (&'a str, Option<f64>, Vec<CounterfactualToken>);

fn legacy_records(
    generation: &PyloomLegacyGeneration,
    skip_prompt: bool,
) -> Option<Vec<LegacyRecord<'_>>> {
    let logprobs = generation.logprobs.as_ref()?;

    let prompt_end = if logprobs.text_offset.iter().any(|offset| *offset < 0) {
        0
    } else {
        generation
            .prompt
            .as_ref()
            .map(|prompt| prompt.chars().count() as i64)
            .unwrap_or_default()
    };

    Some(
        logprobs
            .tokens
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                !skip_prompt
                    || logprobs
                        .text_offset
                        .get(*i)
                        .is_none_or(|offset| *offset >= prompt_end)
            })
            .map(|(i, token)| {
                (
                    token.as_str(),
                    logprobs.token_logprobs.get(i).copied().flatten(),
                    parse_counterfactuals(
                        logprobs
                            .top_logprobs
                            .as_ref()
                            .and_then(|top_logprobs| top_logprobs.get(i)),
                    ),
                )
            })
            .collect(),
    )
}

fn build_legacy_tokens(
    text: &str,
    generation: &PyloomLegacyGeneration,
) -> Option<Vec<InnerNodeToken>> {
    let records = |skip_prompt: bool| -> Option<Vec<TokenRecord>> {
        Some(
            legacy_records(generation, skip_prompt)?
                .into_iter()
                .map(|(token, logprob, counterfactual)| {
                    (
                        decode_token(token),
                        logprob,
                        counterfactual,
                        OriginalToken::Unmodified,
                    )
                })
                .collect(),
        )
    };

    build_tokens(text, records(true)?, false)
        .or_else(|| build_tokens(text, records(false)?, false))
        .or_else(|| build_tokens(text, records(false)?, true))
        .or_else(|| build_tokens(text, records(true)?, true))
}

fn build_diff_tokens(
    text: &str,
    diffs: &Value,
    generation: Option<&PyloomLegacyGeneration>,
) -> Option<Vec<InnerNodeToken>> {
    let entries = diffs.as_array()?;

    let (original_tokens, original_positions) =
        parse_tokenization(entries.first()?.get("diff")?.get("old")?)?;
    let (current_tokens, _) = parse_tokenization(entries.last()?.get("diff")?.get("new")?)?;

    if original_tokens.is_empty() || current_tokens.is_empty() {
        return None;
    }

    let prompt_end = if original_positions.iter().any(|offset| *offset < 0) {
        Some(0)
    } else {
        generation
            .and_then(|generation| generation.prompt.as_ref())
            .map(|prompt| prompt.chars().count() as i64)
    };

    let mut kept: Vec<usize> = (0..original_tokens.len())
        .filter(|i| {
            prompt_end.is_none_or(|prompt_end| {
                original_positions
                    .get(*i)
                    .is_none_or(|offset| *offset >= prompt_end)
            })
        })
        .collect();

    if kept.is_empty() {
        kept = (0..original_tokens.len()).collect();
    }

    let mut records: Vec<(Vec<u8>, Option<f64>, Vec<CounterfactualToken>)> = kept
        .iter()
        .map(|&i| (decode_token(&original_tokens[i]), None, Vec::new()))
        .collect();

    if let Some(generation) = generation {
        for skip_prompt in [true, false] {
            let Some(legacy) = legacy_records(generation, skip_prompt) else {
                break;
            };

            if legacy.len() == records.len()
                && legacy
                    .iter()
                    .zip(&kept)
                    .all(|((token, _, _), &i)| *token == original_tokens[i])
            {
                for (record, (_, logprob, counterfactual)) in records.iter_mut().zip(legacy) {
                    record.1 = logprob;
                    record.2 = counterfactual;
                }

                break;
            }
        }
    }

    let current: Vec<Vec<u8>> = current_tokens
        .iter()
        .map(|token| decode_token(token))
        .collect();

    let ops = align(
        &records
            .iter()
            .map(|(bytes, _, _)| bytes.as_slice())
            .collect::<Vec<_>>(),
        &current.iter().map(Vec::as_slice).collect::<Vec<_>>(),
    )?;

    let mut items: Vec<TokenRecord> = Vec::with_capacity(current.len());
    let mut removed: Vec<u8> = Vec::new();
    let mut inserts: Vec<usize> = Vec::new();

    let flush = |items: &mut Vec<TokenRecord>, removed: &mut Vec<u8>, inserts: &mut Vec<usize>| {
        for (k, j) in inserts.drain(..).enumerate() {
            let original = if k == 0 {
                std::mem::take(removed)
            } else {
                Vec::new()
            };

            items.push((
                current[j].clone(),
                None,
                Vec::new(),
                OriginalToken::Known(original),
            ));
        }
    };

    for op in ops {
        match op {
            AlignOp::Delete(i) => removed.extend_from_slice(&records[i].0),
            AlignOp::Insert(j) => inserts.push(j),
            AlignOp::Match(i, j) => {
                flush(&mut items, &mut removed, &mut inserts);

                let (_, logprob, counterfactual) = records[i].clone();
                let mut record = (
                    current[j].clone(),
                    logprob,
                    counterfactual,
                    OriginalToken::Unmodified,
                );

                if !removed.is_empty() {
                    match items.last_mut() {
                        Some(last) => extend_original(last, &removed),
                        None => {
                            let mut original = std::mem::take(&mut removed);
                            original.extend_from_slice(&record.0);
                            record.3 = OriginalToken::Known(original);
                        }
                    }

                    removed.clear();
                }

                items.push(record);
            }
        }
    }

    flush(&mut items, &mut removed, &mut inserts);

    if !removed.is_empty() {
        extend_original(items.last_mut()?, &removed);
    }

    build_tokens(text, items.clone(), false).or_else(|| build_tokens(text, items, true))
}

fn extend_original(record: &mut TokenRecord, removed: &[u8]) {
    match &mut record.3 {
        OriginalToken::Known(original) => original.extend_from_slice(removed),
        _ => {
            let mut original = record.0.clone();
            original.extend_from_slice(removed);
            record.3 = OriginalToken::Known(original);
        }
    }
}

fn parse_tokenization(value: &Value) -> Option<(Vec<String>, Vec<i64>)> {
    let parts = value.as_array()?;

    let tokens = parts
        .first()?
        .as_array()?
        .iter()
        .map(|token| token.as_str().map(str::to_string))
        .collect::<Option<Vec<_>>>()?;

    let positions = parts
        .get(1)
        .and_then(Value::as_array)
        .map(|positions| {
            positions
                .iter()
                .filter_map(|position| position.as_f64().map(|position| position as i64))
                .collect()
        })
        .unwrap_or_default();

    Some((tokens, positions))
}

enum AlignOp {
    Match(usize, usize),
    Delete(usize),
    Insert(usize),
}

fn align<T: PartialEq>(old: &[T], new: &[T]) -> Option<Vec<AlignOp>> {
    const MAX_CELLS: usize = 4_000_000;

    let (n, m) = (old.len(), new.len());

    if (n + 1).checked_mul(m + 1)? > MAX_CELLS {
        return None;
    }

    let width = m + 1;
    let mut table = vec![0u32; (n + 1) * width];

    for i in (0..n).rev() {
        for j in (0..m).rev() {
            table[i * width + j] = if old[i] == new[j] {
                table[(i + 1) * width + j + 1] + 1
            } else {
                table[(i + 1) * width + j].max(table[i * width + j + 1])
            };
        }
    }

    let mut ops = Vec::with_capacity(n + m);
    let (mut i, mut j) = (0, 0);

    while i < n && j < m {
        if old[i] == new[j] {
            ops.push(AlignOp::Match(i, j));
            i += 1;
            j += 1;
        } else if table[(i + 1) * width + j] >= table[i * width + j + 1] {
            ops.push(AlignOp::Delete(i));
            i += 1;
        } else {
            ops.push(AlignOp::Insert(j));
            j += 1;
        }
    }

    ops.extend((i..n).map(AlignOp::Delete));
    ops.extend((j..m).map(AlignOp::Insert));

    Some(ops)
}

fn parse_counterfactuals(value: Option<&Value>) -> Vec<CounterfactualToken> {
    let mut output = Vec::new();

    let mut push = |token: &str, logprob: &Value| {
        if let Some(logprob) = logprob.as_f64() {
            output.push(CounterfactualToken {
                bytes: decode_token(token),
                logprob: logprob as f32,
                id: None,
            });
        }
    };

    match value {
        Some(Value::Object(map)) => {
            for (token, logprob) in map {
                push(token, logprob);
            }
        }
        Some(Value::Array(items)) => {
            for item in items {
                if let Value::Object(map) = item {
                    for (token, logprob) in map {
                        push(token, logprob);
                    }
                }
            }
        }
        _ => {}
    }

    output
}

fn parse_finish_reason(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(reason)) => Some(reason.clone()),
        Some(Value::Object(map)) => map
            .get("reason")
            .and_then(|reason| reason.as_str())
            .map(str::to_string),
        _ => None,
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PyloomWeave {
    root: PyloomNode,
    #[serde(default)]
    chapters: IndexMap<String, PyloomChapter>,
    selected_node_id: Option<String>,
    #[serde(default)]
    model_responses: IndexMap<String, PyloomModelResponse>,
    #[serde(default)]
    tags: IndexMap<String, Value>,
    #[serde(default)]
    canonical: Vec<String>,
}

impl PyloomWeave {
    fn from_root(root: PyloomNode) -> Self {
        Self {
            root,
            chapters: IndexMap::default(),
            selected_node_id: None,
            model_responses: IndexMap::default(),
            tags: IndexMap::default(),
            canonical: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PyloomChapter {
    title: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct PyloomNode {
    #[serde(default)]
    id: String,
    chapter_id: Option<String>,
    #[serde(default)]
    text: String,
    text_attributes: Option<PyloomTextAttr>,
    #[serde(default)]
    children: Vec<PyloomNode>,
    meta: Option<PyloomMeta>,
    tags: Option<Vec<String>>,
    bookmark: Option<bool>,
    archived: Option<bool>,
    canonical: Option<bool>,
    masked_head: Option<Box<PyloomNode>>,
    tail_id: Option<String>,
    generation: Option<PyloomGeneration>,
    notes: Option<Value>,
    multimedia: Option<Value>,
    template: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct PyloomTextAttr {
    active_append: Option<String>,
    child_preview: Option<String>,
    nav_preview: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PyloomMeta {
    creation_timestamp: Option<String>,
    source: Option<String>,
    modified: Option<bool>,
    generation: Option<PyloomLegacyGeneration>,
    diffs: Option<Value>,
    origin: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PyloomGeneration {
    id: String,
    index: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PyloomModelResponse {
    model: Option<String>,
    timestamp: Option<String>,
    prompt: Option<PyloomPrompt>,
    #[serde(default)]
    completions: Vec<PyloomCompletion>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PyloomPrompt {
    text: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PyloomCompletion {
    finishReason: Option<Value>,
    #[serde(default)]
    tokens: Vec<PyloomToken>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PyloomToken {
    generatedToken: PyloomGeneratedToken,
    counterfactuals: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PyloomGeneratedToken {
    token: String,
    logprob: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PyloomLegacyGeneration {
    model: Option<String>,
    prompt: Option<String>,
    finish_reason: Option<String>,
    logprobs: Option<PyloomLegacyLogprobs>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PyloomLegacyLogprobs {
    #[serde(default)]
    tokens: Vec<String>,
    #[serde(default)]
    token_logprobs: Vec<Option<f64>>,
    top_logprobs: Option<Vec<Value>>,
    #[serde(default)]
    text_offset: Vec<i64>,
}
