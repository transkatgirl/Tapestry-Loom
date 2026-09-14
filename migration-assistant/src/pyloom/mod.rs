#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::hash::BuildHasherDefault;

use chrono::{Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use stacksafe::stacksafe;
use tapestry_weave::{
    TapestryNode, TapestryWeave,
    content::{Creator, InnerNodeContent, NodeContent},
    hashers::{RandomIdHasher, RandomState},
    jiff::{Zoned, fmt::rfc2822::DateTimeParser},
    nanorand::{Rng, WyRand},
    universal_weave::{
        Weave,
        indexmap::{IndexMap, IndexSet},
    },
    wrappers::UniqueIdentifierRemapper,
};

use crate::new_weave;

const PARSER: DateTimeParser = DateTimeParser::new();

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

    assign_missing_identifiers(&mut data.root, &mut 0);

    let selected = data
        .selected_node_id
        .or_else(|| data.root.children.first().map(|child| child.id.clone()))
        .unwrap_or_default();

    let chapters: IndexMap<String, String> = data
        .chapters
        .into_iter()
        .map(|(id, chapter)| (id, chapter.title))
        .collect();

    let node_count_guess = (input.len() as f64 / 34.0).ceil() as usize;

    let mut output = new_weave(node_count_guess, created, "PyLoom", None);

    let mut mapper: UniqueIdentifierRemapper<
        String,
        u64,
        RandomState,
        BuildHasherDefault<RandomIdHasher>,
    > = UniqueIdentifierRemapper::with_capacity(node_count_guess);

    let mut rng = WyRand::new();

    let mut convert_old_identifier = move |id| *mapper.map(id, || rng.generate()).get();

    convert_node(
        &mut output,
        &mut convert_old_identifier,
        data.root,
        None,
        &chapters,
    )?;

    if !selected.is_empty() {
        let selected = convert_old_identifier(selected);
        if output.contains(&selected) {
            output.set_active_tree_semantics(&selected, true);
        }
    }

    Ok(Some(output))
}

#[stacksafe]
fn assign_missing_identifiers(node: &mut PyloomNode, counter: &mut usize) {
    if node.id.is_empty() {
        node.id = format!("\0generated:{counter}");
        *counter += 1;
    }

    for child in &mut node.children {
        assign_missing_identifiers(child, counter);
    }
}

#[stacksafe]
fn convert_node(
    weave: &mut TapestryWeave,
    convert_old_identifier: &mut impl FnMut(String) -> u64,
    node: PyloomNode,
    parent: Option<u64>,
    chapters: &IndexMap<String, String>,
) -> anyhow::Result<()> {
    let timestamp = node
        .meta
        .as_ref()
        .and_then(|meta| meta.creation_timestamp.clone())
        .and_then(|timestamp| NaiveDateTime::parse_from_str(&timestamp, "%Y-%m-%d-%H.%M.%S").ok())
        .and_then(|timestamp| timestamp.and_local_timezone(Local).earliest())
        .and_then(|timestamp| PARSER.parse_zoned(timestamp.to_rfc2822()).ok())
        .unwrap_or_default();

    let id = convert_old_identifier(node.id.clone());

    let chapter = node.chapter_id.and_then(|chapter| chapters.get(&chapter));

    let tags = node.tags.unwrap_or_default();

    let mut metadata = IndexMap::with_capacity_and_hasher(3, RandomState::default());

    let _suffix = if let Some(attributes) = node.text_attributes {
        if let Some(preview) = attributes.child_preview {
            metadata.insert("child_preview".to_string(), preview.clone());
        }

        if let Some(preview) = attributes.nav_preview {
            metadata.insert("nav_preview".to_string(), preview.clone());
        }

        if let Some(append) = attributes.active_append {
            metadata.insert("active_append".to_string(), append.clone());
            append
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    if let Some(chapter) = chapter {
        metadata.insert("chapter".to_string(), chapter.clone());
    }

    if !tags.is_empty() {
        metadata.insert("tags".to_string(), serde_json::to_string(&tags)?);
    }

    let text = node.text;
    //text.push_str(&suffix);

    assert!(
        weave.insert(TapestryNode {
            id,
            from: IndexSet::from_iter(parent),
            to: IndexSet::default(),
            active: false,
            bookmarked: chapter.is_some() || tags.iter().any(|tag| tag == "bookmark"),
            contents: NodeContent {
                timestamp,
                modified: node
                    .meta
                    .as_ref()
                    .map(|meta| meta.source.as_deref() == Some("mixed")
                        || meta.modified.unwrap_or_default())
                    .unwrap_or_default(),
                content: InnerNodeContent::Snippet(text.into_bytes()),
                metadata,
                aux_metadata: IndexMap::default(),
                creator: node
                    .meta
                    .map(|meta| match meta.source.as_deref() {
                        Some("AI") => Creator::Model(None),
                        Some("mixed") => Creator::User(None),
                        Some("prompt") => Creator::User(None),
                        _ => Creator::Unknown,
                    })
                    .unwrap_or(Creator::Unknown)
            },
        })
    );

    for child in node.children {
        convert_node(weave, convert_old_identifier, child, Some(id), chapters)?;
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PyloomWeave {
    root: PyloomNode,
    #[serde(default)]
    chapters: IndexMap<String, PyloomChapter>,
    selected_node_id: Option<String>,
}

impl PyloomWeave {
    fn from_root(root: PyloomNode) -> Self {
        Self {
            root,
            chapters: IndexMap::default(),
            selected_node_id: None,
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
}
