#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::{
    collections::{HashMap, HashSet},
    hash::BuildHasherDefault,
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use stacksafe::stacksafe;
use tapestry_weave::{
    TapestryNode, TapestryWeave,
    content::{
        CounterfactualToken, Creator, InnerNodeContent, InnerNodeToken, Model, NodeContent,
        OriginalToken,
    },
    jiff::{Zoned, civil::DateTime, tz::TimeZone},
    nanorand::{Rng, WyRand},
    universal_weave::{
        DiscreteContentResult, MetadataWeave, Weave,
        indexmap::{IndexMap, IndexSet},
    },
    util::{RandomIdHasher, RandomState, UniqueIdentifierRemapper},
};

use crate::new_weave;

pub fn migrate(input: &str, created: Zoned) -> anyhow::Result<Option<TapestryWeave>> {
    let Ok(value) = parse_json(input) else {
        return Ok(None);
    };

    let data = if value.is_array() {
        from_value::<Vec<PyloomNode>>(value).map(|children| {
            PyloomWeave::from_root(PyloomNode {
                children,
                ..PyloomNode::default()
            })
        })
    } else if value.get("root").is_some() {
        from_value::<PyloomWeave>(value)
    } else if value.get("text").is_some() {
        from_value::<PyloomNode>(value).map(PyloomWeave::from_root)
    } else {
        return Ok(None);
    };

    let Ok(mut data) = data else {
        return Ok(None);
    };

    let root = std::mem::take(&mut data.root);
    data.root = unzip_masks(root, &mut data.selected_node_id, &mut HashMap::new());

    let node_count = assign_missing_identifiers(&mut data.root, &mut HashSet::new(), &mut 0);

    let mut split_children = HashSet::new();
    collect_split_children(&data.root, &mut split_children);

    let selected = data
        .selected_node_id
        .or_else(|| data.root.children.first().map(|child| child.id.clone()))
        .unwrap_or_default();

    let chapters: IndexMap<String, String> = data
        .chapters
        .into_iter()
        .map(|(id, chapter)| (id, chapter.title))
        .collect();

    let mut tags = default_tags();

    for (name, definition) in &data.tags {
        let definition = TagDefinition::from_value(definition, tags.get(name));
        tags.insert(name.clone(), definition);
    }

    let canonical: HashSet<String> = data.canonical.into_iter().collect();

    let mut ancestry_tags = HashMap::new();
    collect_ancestry_tags(&data.root, &tags, &canonical, &mut ancestry_tags);

    let mut output = new_weave(node_count, created, "PyLoom", None);

    let context = Context {
        chapters,
        responses: data.model_responses,
        tags,
        canonical,
        split_children,
        ancestry_tags,
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
        Inherited::default(),
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
    tags: IndexMap<String, TagDefinition>,
    canonical: HashSet<String>,
    split_children: HashSet<String>,
    ancestry_tags: HashMap<String, Vec<String>>,
    time_zone: TimeZone,
}

impl Context {
    fn is_visible(&self, tags: &[String]) -> bool {
        let mut show_only_defined = false;
        let mut shown = false;

        for (name, definition) in &self.tags {
            let has_tag = tags.iter().any(|tag| tag == name);

            if definition.hide && has_tag {
                return false;
            }

            if definition.show_only {
                show_only_defined = true;
                shown |= has_tag;
            }
        }

        !show_only_defined || shown
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TagScope {
    Node,
    Subtree,
    Ancestry,
}

#[derive(Clone, Copy)]
struct TagDefinition {
    scope: TagScope,
    hide: bool,
    show_only: bool,
}

impl TagDefinition {
    const fn new(scope: TagScope, hide: bool) -> Self {
        Self {
            scope,
            hide,
            show_only: false,
        }
    }

    fn from_value(value: &Value, fallback: Option<&Self>) -> Self {
        let fallback = fallback
            .copied()
            .unwrap_or(Self::new(TagScope::Node, false));

        Self {
            scope: match value.get("scope").and_then(Value::as_str) {
                Some("subtree") => TagScope::Subtree,
                Some("ancestry") => TagScope::Ancestry,
                Some(_) => TagScope::Node,
                None => fallback.scope,
            },
            hide: value
                .get("hide")
                .and_then(Value::as_bool)
                .unwrap_or(fallback.hide),
            show_only: value
                .get("show_only")
                .and_then(Value::as_bool)
                .unwrap_or(fallback.show_only),
        }
    }
}

fn default_tags() -> IndexMap<String, TagDefinition> {
    IndexMap::from_iter([
        (
            "bookmark".to_string(),
            TagDefinition::new(TagScope::Node, false),
        ),
        (
            "canonical".to_string(),
            TagDefinition::new(TagScope::Ancestry, false),
        ),
        (
            "archived".to_string(),
            TagDefinition::new(TagScope::Node, true),
        ),
        ("note".to_string(), TagDefinition::new(TagScope::Node, true)),
        (
            "pinned".to_string(),
            TagDefinition::new(TagScope::Node, false),
        ),
    ])
}

fn own_tags(node: &PyloomNode, canonical: &HashSet<String>) -> Vec<String> {
    let mut tags = node.tags.clone().unwrap_or_default();

    let canonical = node.canonical.unwrap_or_default() || canonical.contains(&node.id);

    for (flag, tag) in [
        (node.bookmark.unwrap_or_default(), "bookmark"),
        (node.archived.unwrap_or_default(), "archived"),
        (canonical, "canonical"),
    ] {
        if flag && !tags.iter().any(|t| t == tag) {
            tags.push(tag.to_string());
        }
    }

    tags
}

#[stacksafe]
fn collect_ancestry_tags(
    node: &PyloomNode,
    definitions: &IndexMap<String, TagDefinition>,
    canonical: &HashSet<String>,
    output: &mut HashMap<String, Vec<String>>,
) -> Vec<String> {
    let mut tags: Vec<String> = own_tags(node, canonical)
        .into_iter()
        .filter(|tag| {
            definitions
                .get(tag)
                .is_some_and(|definition| definition.scope == TagScope::Ancestry)
        })
        .collect();

    for child in &node.children {
        for tag in collect_ancestry_tags(child, definitions, canonical, output) {
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        }
    }

    if !tags.is_empty() {
        output.insert(node.id.clone(), tags.clone());
    }

    tags
}

fn parse_json(input: &str) -> serde_json::Result<Value> {
    let mut deserializer = serde_json::Deserializer::from_str(input);
    deserializer.disable_recursion_limit();

    let value = Value::deserialize(serde_stacker::Deserializer::new(&mut deserializer))?;
    deserializer.end()?;

    Ok(value)
}

fn from_value<T: DeserializeOwned>(value: Value) -> serde_json::Result<T> {
    T::deserialize(serde_stacker::Deserializer::new(value))
}

#[stacksafe]
fn collect_split_children(node: &PyloomNode, output: &mut HashSet<String>) {
    if let Some(child) = node
        .meta
        .as_ref()
        .and_then(|meta| meta.origin.as_deref())
        .and_then(split_child_id)
    {
        output.insert(child.to_string());
    }

    for child in &node.children {
        collect_split_children(child, output);
    }
}

fn split_child_id(origin: &str) -> Option<&str> {
    origin.strip_prefix("split (from child ")?.strip_suffix(')')
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
fn unzip_masks(
    mut node: PyloomNode,
    selected: &mut Option<String>,
    resolved: &mut HashMap<String, String>,
) -> PyloomNode {
    node.children = node
        .children
        .into_iter()
        .map(|child| unzip_masks(child, selected, resolved))
        .collect();

    let Some(head) = node.masked_head.take() else {
        return node;
    };

    let mut head = unzip_masks(*head, selected, resolved);

    let tail_id = node.tail_id.as_deref().map(|tail_id| {
        if contains_node(&head, tail_id) {
            tail_id.to_string()
        } else {
            resolved
                .get(tail_id)
                .cloned()
                .unwrap_or_else(|| tail_id.to_string())
        }
    });

    let tail = tail_id
        .as_deref()
        .and_then(|tail_id| find_node_mut(&mut head, tail_id));

    let merged = match tail {
        Some(tail) => {
            tail.children.append(&mut node.children);
            merge_mask_attributes(&mut node, tail);

            if selected.as_deref() == Some(node.id.as_str()) {
                *selected = Some(tail.id.clone());
            }

            Some(tail.id.clone())
        }
        None => None,
    };

    match merged {
        Some(tail_id) => {
            if !node.id.is_empty() {
                resolved.insert(std::mem::take(&mut node.id), tail_id);
            }

            if head.chapter_id.is_none() {
                head.chapter_id = node.chapter_id.take();
            }

            head
        }
        None => {
            eprintln!(
                "Warning: Missing tail {:?} for node {:?}; keeping the masked subtree as a child of the mask node. The mask node's text repeats the text of the masked chain, so this branch will contain duplicated text",
                node.tail_id, node.id
            );

            node.children.push(head);

            node
        }
    }
}

fn contains_node(root: &PyloomNode, id: &str) -> bool {
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        if node.id == id {
            return true;
        }

        stack.extend(node.children.iter());
    }

    false
}

fn find_node_mut<'a>(root: &'a mut PyloomNode, id: &str) -> Option<&'a mut PyloomNode> {
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        if node.id == id {
            return Some(node);
        }

        stack.extend(node.children.iter_mut());
    }

    None
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

#[derive(Default)]
struct Inherited {
    tags: Vec<String>,
    split: Option<SplitChain>,
}

struct SplitChain {
    remaining: Vec<(String, InnerNodeContent)>,
    timestamp: Option<Zoned>,
    model_label: Option<String>,
    finish_reason: Option<String>,
}

struct Generated {
    tokens: Option<Vec<InnerNodeToken>>,
    model_label: Option<String>,
    finish_reason: Option<String>,
}

#[stacksafe]
fn convert_node(
    weave: &mut TapestryWeave,
    convert_old_identifier: &mut impl FnMut(String) -> u64,
    mut node: PyloomNode,
    parent: Option<u64>,
    context: &Context,
    inherited: Inherited,
) -> anyhow::Result<()> {
    let Inherited {
        tags: inherited_tags,
        split,
    } = inherited;

    let source = node.meta.as_ref().and_then(|meta| meta.source.clone());
    let origin = node.meta.as_ref().and_then(|meta| meta.origin.clone());

    let modified = origin
        .as_deref()
        .is_some_and(|origin| origin.starts_with("split"))
        || context.split_children.contains(&node.id);

    let edited = is_edited(&node);

    let mut chain = split.or_else(|| start_split_chain(&node, context));

    let piece = chain
        .as_mut()
        .filter(|chain| chain.remaining.last().is_some_and(|(id, _)| *id == node.id))
        .and_then(|chain| chain.remaining.pop())
        .map(|(_, content)| content);

    if piece.is_none() {
        chain = None;
    }

    let generated = piece.is_some()
        || node.generation.is_some()
        || node
            .meta
            .as_ref()
            .is_some_and(|meta| meta.generation.is_some());

    let timestamp = node_timestamp(&node, context)
        .or_else(|| chain.as_ref().and_then(|chain| chain.timestamp.clone()))
        .unwrap_or_default();

    let (content, model_label, finish_reason) = match (piece, &chain) {
        (Some(content), Some(chain)) => (
            content,
            chain.model_label.clone(),
            chain.finish_reason.clone(),
        ),
        _ => {
            let generated = build_generated(&node, &node.text, edited, context);

            let content = match generated.tokens {
                Some(tokens) => InnerNodeContent::Tokens(tokens),
                None => InnerNodeContent::Snippet(std::mem::take(&mut node.text).into_bytes()),
            };

            (content, generated.model_label, generated.finish_reason)
        }
    };

    let id = convert_old_identifier(node.id.clone());

    let chapter = node
        .chapter_id
        .as_ref()
        .and_then(|chapter| context.chapters.get(chapter));

    let mut tags = own_tags(&node, &context.canonical);

    for tag in inherited_tags
        .iter()
        .chain(context.ancestry_tags.get(&node.id).into_iter().flatten())
    {
        if !tags.contains(tag) {
            tags.push(tag.clone());
        }
    }

    let bookmarked = chapter.is_some() || tags.iter().any(|tag| tag == "bookmark");

    let hidden = parent.is_some() && !context.is_visible(&tags);

    let subtree_tags: Vec<String> = tags
        .iter()
        .filter(|tag| {
            context
                .tags
                .get(*tag)
                .is_some_and(|definition| definition.scope == TagScope::Subtree)
        })
        .cloned()
        .collect();

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

    if let Some(origin) = origin {
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

    let model = model_label.map(|label| Model {
        label,
        color: None,
        identifier: None,
        seed: None,
        system_fingerprint: None,
        finish_reason,
        metadata: IndexMap::default(),
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
            creator,
        },
    }));

    for child in node.children {
        let continues_chain = chain.as_ref().is_some_and(|chain| {
            chain
                .remaining
                .last()
                .is_some_and(|(id, _)| *id == child.id)
        });

        convert_node(
            weave,
            convert_old_identifier,
            child,
            Some(id),
            context,
            Inherited {
                tags: subtree_tags.clone(),
                split: if continues_chain { chain.take() } else { None },
            },
        )?;
    }

    Ok(())
}

fn is_edited(node: &PyloomNode) -> bool {
    node.meta.as_ref().is_some_and(|meta| {
        meta.source.as_deref() == Some("mixed") || meta.modified.unwrap_or_default()
    })
}

fn node_timestamp(node: &PyloomNode, context: &Context) -> Option<Zoned> {
    parse_timestamp(
        node.meta
            .as_ref()
            .and_then(|meta| meta.creation_timestamp.as_deref()),
        &context.time_zone,
    )
    .or_else(|| {
        let response = node
            .generation
            .as_ref()
            .and_then(|generation| context.responses.get(&generation.id));

        parse_timestamp(
            response.and_then(|response| response.timestamp.as_deref()),
            &context.time_zone,
        )
    })
}

fn build_generated(node: &PyloomNode, text: &str, edited: bool, context: &Context) -> Generated {
    let generated = node.generation.is_some()
        || node
            .meta
            .as_ref()
            .is_some_and(|meta| meta.generation.is_some());

    let mut model_label = None;
    let mut finish_reason = None;
    let mut tokens = None;

    if let Some(generation) = &node.generation {
        if let Some(response) = context.responses.get(&generation.id) {
            model_label = response.model.clone();

            if let Some(completion) = response.completions.get(generation.index) {
                finish_reason = parse_finish_reason(completion.finishReason.as_ref());
                tokens = build_tokens(
                    text,
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
                    edited,
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
        finish_reason = legacy.finish_reason.clone();
        tokens = build_legacy_tokens(text, legacy, edited);
    }

    if tokens.is_none()
        && generated
        && let Some(meta) = &node.meta
        && let Some(diffs) = &meta.diffs
    {
        tokens = build_diff_tokens(text, diffs, meta.generation.as_ref());
    }

    Generated {
        tokens,
        model_label,
        finish_reason,
    }
}

fn start_split_chain(node: &PyloomNode, context: &Context) -> Option<SplitChain> {
    let mut chain = vec![node];
    let mut current = node;

    while let Some(child_id) = current
        .meta
        .as_ref()
        .and_then(|meta| meta.origin.as_deref())
        .and_then(split_child_id)
    {
        let Some(child) = current.children.iter().find(|child| child.id == child_id) else {
            break;
        };

        chain.push(child);
        current = child;
    }

    if chain.len() < 2 {
        return None;
    }

    let text: String = chain.iter().map(|node| node.text.as_str()).collect();
    let edited = chain.iter().any(|node| is_edited(node));

    let generated = build_generated(current, &text, edited, context);
    let content = InnerNodeContent::Tokens(generated.tokens?);

    if content.len() != text.len() {
        return None;
    }

    let lengths: Vec<usize> = chain.iter().map(|node| node.text.len()).collect();

    let remaining = chain
        .iter()
        .map(|node| node.id.clone())
        .zip(split_content(content, &lengths))
        .rev()
        .collect();

    Some(SplitChain {
        remaining,
        timestamp: node_timestamp(current, context),
        model_label: generated.model_label,
        finish_reason: generated.finish_reason,
    })
}

fn split_content(mut content: InnerNodeContent, lengths: &[usize]) -> Vec<InnerNodeContent> {
    let mut output = Vec::with_capacity(lengths.len());

    for &length in lengths.iter().take(lengths.len().saturating_sub(1)) {
        if length == 0 {
            output.push(InnerNodeContent::Snippet(Vec::new()));
            continue;
        }

        content = match content.split(length) {
            DiscreteContentResult::Two(left, right) => {
                output.push(left);
                right
            }
            DiscreteContentResult::One(whole) => {
                output.push(whole);
                InnerNodeContent::Snippet(Vec::new())
            }
        };
    }

    output.push(content);

    output
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
    edited: bool,
) -> Option<Vec<InnerNodeToken>> {
    let joined: Vec<u8> = tokens
        .iter()
        .flat_map(|(bytes, _, _, _)| bytes.iter().copied())
        .collect();

    let bytes = text.as_bytes();

    if joined.is_empty() || bytes.is_empty() {
        return None;
    }

    let origin: isize = if bytes == joined.as_slice() {
        0
    } else if !partial {
        return None;
    } else if let Some(position) = find(bytes, &joined) {
        -(position as isize)
    } else {
        find_unique(&joined, bytes)? as isize
    };

    let joined_len = joined.len() as isize;
    let end = origin + bytes.len() as isize;

    let mut output = Vec::with_capacity(tokens.len() + 2);

    if origin < 0 {
        output.push(unknown_token(
            bytes[..origin.unsigned_abs()].to_vec(),
            edited,
        ));
    }

    let mut position: isize = 0;

    for (token_bytes, logprob, counterfactual, original) in tokens {
        let start = position;
        let stop = start + token_bytes.len() as isize;
        position = stop;

        let window_start = start.max(origin);
        let window_stop = stop.min(end);

        if window_start > window_stop || (window_start == window_stop && !token_bytes.is_empty()) {
            continue;
        }

        let (token_bytes, original) = if window_start == start && window_stop == stop {
            (token_bytes, original)
        } else {
            let offset = (window_start - start) as usize;
            let slice = token_bytes[offset..(window_stop - start) as usize].to_vec();

            let original = if original.is_modified() {
                original
            } else {
                OriginalToken::Known {
                    bytes: token_bytes,
                    id: None,
                    offset,
                }
            };

            (slice, original)
        };

        let mut token = InnerNodeToken {
            bytes: token_bytes,
            logprob: logprob.map(|p| p as f32),
            id: None,
            entropy: None,
            counterfactual,
            original,
        };
        token.sort_counterfactual();

        output.push(token);
    }

    if end > joined_len {
        output.push(unknown_token(
            bytes[(joined_len - origin) as usize..].to_vec(),
            edited,
        ));
    }

    Some(output)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn find_unique(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let mut matches = haystack
        .windows(needle.len())
        .enumerate()
        .filter(|(_, window)| *window == needle)
        .map(|(position, _)| position);

    let first = matches.next()?;

    matches.next().is_none().then_some(first)
}

fn unknown_token(bytes: Vec<u8>, edited: bool) -> InnerNodeToken {
    InnerNodeToken {
        bytes,
        logprob: None,
        id: None,
        entropy: None,
        counterfactual: Vec::new(),
        original: if edited {
            OriginalToken::Unknown
        } else {
            OriginalToken::Unmodified
        },
    }
}

type LegacyRecord = (Vec<u8>, Option<f64>, Vec<CounterfactualToken>);

fn prompt_token_count(tokens: &[Vec<u8>], offsets: &[i64], prompt: Option<&str>) -> usize {
    if offsets.iter().any(|offset| *offset < 0) {
        return offsets
            .iter()
            .take(tokens.len())
            .take_while(|offset| **offset < 0)
            .count();
    }

    let Some(prompt) = prompt.filter(|prompt| !prompt.is_empty()) else {
        return 0;
    };

    let first_offset_in_prompt = offsets
        .first()
        .is_none_or(|offset| *offset < prompt.chars().count() as i64);
    let joined_len: usize = tokens.iter().map(Vec::len).sum();

    if !first_offset_in_prompt
        || joined_len < prompt.len()
        || !tokens
            .iter()
            .flatten()
            .zip(prompt.as_bytes())
            .all(|(token, prompt)| token == prompt)
    {
        return 0;
    }

    let mut consumed = 0;
    let mut count = 0;

    for token in tokens {
        if consumed >= prompt.len() {
            break;
        }

        consumed += token.len();
        count += 1;
    }

    count
}

fn legacy_records(
    generation: &PyloomLegacyGeneration,
    skip_prompt: bool,
) -> Option<Vec<LegacyRecord>> {
    let logprobs = generation.logprobs.as_ref()?;

    let tokens: Vec<Vec<u8>> = logprobs
        .tokens
        .iter()
        .map(|token| decode_token(token))
        .collect();

    let skip = if skip_prompt {
        prompt_token_count(&tokens, &logprobs.text_offset, generation.prompt.as_deref())
    } else {
        0
    };

    Some(
        tokens
            .into_iter()
            .enumerate()
            .skip(skip)
            .map(|(i, token)| {
                (
                    token,
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
    edited: bool,
) -> Option<Vec<InnerNodeToken>> {
    let records = |skip_prompt: bool| -> Option<Vec<TokenRecord>> {
        Some(
            legacy_records(generation, skip_prompt)?
                .into_iter()
                .map(|(token, logprob, counterfactual)| {
                    (token, logprob, counterfactual, OriginalToken::Unmodified)
                })
                .collect(),
        )
    };

    build_tokens(text, records(true)?, false, edited)
        .or_else(|| build_tokens(text, records(false)?, false, edited))
        .or_else(|| build_tokens(text, records(true)?, true, edited))
        .or_else(|| build_tokens(text, records(false)?, true, edited))
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

    let original: Vec<Vec<u8>> = original_tokens
        .iter()
        .map(|token| decode_token(token))
        .collect();

    let mut skip = prompt_token_count(
        &original,
        &original_positions,
        generation.and_then(|generation| generation.prompt.as_deref()),
    );

    if skip >= original.len() {
        skip = 0;
    }

    let mut records: Vec<LegacyRecord> = original
        .into_iter()
        .skip(skip)
        .map(|token| (token, None, Vec::new()))
        .collect();

    if let Some(generation) = generation {
        for skip_prompt in [true, false] {
            let Some(legacy) = legacy_records(generation, skip_prompt) else {
                break;
            };

            if legacy.len() == records.len()
                && legacy
                    .iter()
                    .zip(&records)
                    .all(|(legacy, record)| legacy.0 == record.0)
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

    let items: Vec<TokenRecord> = ops
        .into_iter()
        .filter_map(|op| match op {
            AlignOp::Match(i, j) => Some((
                current[j].clone(),
                records[i].1,
                std::mem::take(&mut records[i].2),
                OriginalToken::Unmodified,
            )),
            AlignOp::Insert(j) => {
                Some((current[j].clone(), None, Vec::new(), OriginalToken::Unknown))
            }
            AlignOp::Delete => None,
        })
        .collect();

    build_tokens(text, items.clone(), false, true).or_else(|| build_tokens(text, items, true, true))
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
    Delete,
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
            ops.push(AlignOp::Delete);
            i += 1;
        } else {
            ops.push(AlignOp::Insert(j));
            j += 1;
        }
    }

    ops.extend((i..n).map(|_| AlignOp::Delete));
    ops.extend((j..m).map(AlignOp::Insert));

    Some(ops)
}

fn parse_counterfactuals(value: Option<&Value>) -> Vec<CounterfactualToken> {
    let mut output = Vec::new();

    let mut push = |token: &str, logprob: &Value| {
        if let Some(logprob) = logprob.as_f64() {
            output.push(CounterfactualToken {
                bytes: decode_token(token),
                logprob: Some(logprob as f32),
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
