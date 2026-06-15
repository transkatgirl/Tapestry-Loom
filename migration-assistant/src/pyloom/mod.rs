#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::hash::BuildHasherDefault;

use chrono::{Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use tapestry_weave::{
    VersionedWeave,
    hashers::{RandomIdHasher, RandomState},
    jiff::{Zoned, fmt::rfc2822::DateTimeParser},
    nanorand::Rng,
    universal_weave::{
        dependent::DependentNode,
        indexmap::{IndexMap, IndexSet},
    },
    v1::{
        content::{Creator, InnerNodeContent, NodeContent},
        dependent::TapestryWeave,
    },
    wrappers::UniqueIdentifierRemapper,
};

use crate::new_weave;

const PARSER: DateTimeParser = DateTimeParser::new();

pub fn migrate(input: &str, created: Zoned) -> anyhow::Result<Option<VersionedWeave>> {
    if let Ok(data) = serde_json::from_str::<PyloomWeave>(input) {
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

        let mut rng = output.rng.clone();

        let mut convert_old_identifier = move |id| *mapper.map(id, || rng.generate()).get();

        convert_node(
            &mut output,
            &mut convert_old_identifier,
            data.root,
            None,
            &data.selected_node_id,
            &chapters,
        )?;

        Ok(Some(output.to_versioned_weave()))
    } else {
        Ok(None)
    }
}

fn convert_node(
    weave: &mut TapestryWeave,
    convert_old_identifier: &mut impl FnMut(String) -> u64,
    node: PyloomNode,
    parent: Option<u64>,
    selected: &String,
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

    if !node.tags.is_empty() {
        metadata.insert("tags".to_string(), serde_json::to_string(&node.tags)?);
    }

    let text = node.text;
    //text.push_str(&suffix);

    assert!(
        weave.add_node_direct(DependentNode {
            id,
            from: parent,
            to: IndexSet::default(),
            active: &node.id == selected,
            bookmarked: chapter.is_some(),
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
        convert_node(
            weave,
            convert_old_identifier,
            child,
            Some(id),
            selected,
            chapters,
        )?;
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PyloomWeave {
    root: PyloomNode,
    chapters: IndexMap<String, PyloomChapter>,
    selected_node_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PyloomChapter {
    title: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PyloomNode {
    id: String,
    parent_id: Option<String>,
    chapter_id: Option<String>,
    text: String,
    text_attributes: Option<PyloomTextAttr>,
    children: Vec<PyloomNode>,
    meta: Option<PyloomMeta>,

    #[serde(default)]
    tags: Vec<String>,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PyloomSimpleNode {
    text: String,
    children: Vec<PyloomSimpleNode>,
}

pub fn migrate_simple(input: &str, created: Zoned) -> anyhow::Result<Option<VersionedWeave>> {
    if let Ok(data) = serde_json::from_str::<PyloomSimpleNode>(input) {
        let node_count_guess = (input.len() as f64 / 26.0).ceil() as usize;

        let mut output = new_weave(node_count_guess, created, "PyLoomSimple", None);

        convert_export_node(&mut output, data, None)?;

        Ok(Some(output.to_versioned_weave()))
    } else {
        Ok(None)
    }
}

fn convert_export_node(
    weave: &mut TapestryWeave,
    node: PyloomSimpleNode,
    parent: Option<u64>,
) -> anyhow::Result<()> {
    let id = weave.generate_id();

    assert!(weave.add_node_direct(DependentNode {
        id,
        from: parent,
        to: IndexSet::default(),
        active: false,
        bookmarked: false,
        contents: NodeContent {
            timestamp: Zoned::default(),
            modified: false,
            content: InnerNodeContent::Snippet(node.text.into_bytes()),
            metadata: IndexMap::default(),
            creator: Creator::Unknown,
        },
    }));

    for child in node.children {
        convert_export_node(weave, child, Some(id))?;
    }

    Ok(())
}
