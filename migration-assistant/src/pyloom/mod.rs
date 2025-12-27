#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::{
    collections::{HashMap, hash_map::Entry},
    time::SystemTime,
};

use chrono::{DateTime, Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use tapestry_weave::{
    VersionedWeave,
    rustc_hash::FxBuildHasher,
    ulid::Ulid,
    universal_weave::{
        Weave,
        dependent::DependentNode,
        indexmap::{IndexMap, IndexSet},
    },
    v0::{InnerNodeContent, Model, NodeContent, TapestryWeave},
};

use crate::new_weave_v0;

pub fn migrate(input: &str, created: DateTime<Local>) -> anyhow::Result<Option<VersionedWeave>> {
    if let Ok(data) = serde_json::from_str::<PyloomWeave>(input) {
        let chapters: IndexMap<String, String> = data
            .chapters
            .into_iter()
            .map(|(id, chapter)| (id, chapter.title))
            .collect();

        let mut id_map = HashMap::with_capacity(16384);
        let mut output = new_weave_v0(16384, created, "PyLoom");

        convert_node(
            &mut output,
            &mut id_map,
            SystemTime::from(created),
            data.root,
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
    id_map: &mut HashMap<String, Ulid>,
    created: SystemTime,
    node: PyloomNode,
    selected: &String,
    chapters: &IndexMap<String, String>,
) -> anyhow::Result<()> {
    let time = node
        .meta
        .as_ref()
        .and_then(|meta| meta.creation_timestamp.clone())
        .and_then(|timestamp| NaiveDateTime::parse_from_str(&timestamp, "%Y-%m-%d-%H.%M.%S").ok())
        .and_then(|timestamp| timestamp.and_local_timezone(Local).earliest())
        .map(SystemTime::from)
        .unwrap_or(created);
    let new_id = map_id(id_map, node.id.clone(), time);

    let parent = node.parent_id.and_then(|parent| {
        if let Some(parent) = id_map.get(&parent) {
            Some(parent)
        } else {
            eprintln!("Warning: Node {} has missing parents", &node.id);
            None
        }
    });

    let chapter = node.chapter_id.and_then(|chapter| chapters.get(&chapter));

    let mut metadata = IndexMap::with_capacity_and_hasher(3, FxBuildHasher);

    if let Some(meta) = &node.meta
        && let Some(modified) = meta.modified
    {
        metadata.insert("modified".to_string(), modified.to_string());
    }

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
        weave.weave.add_node(DependentNode {
            id: new_id.0,
            from: parent.map(|id| id.0),
            to: IndexSet::default(),
            active: &node.id == selected,
            bookmarked: chapter.is_some(),
            contents: NodeContent {
                content: InnerNodeContent::Snippet(text.into_bytes()),
                metadata,
                model: if node
                    .meta
                    .as_ref()
                    .map(|meta| meta.source.as_deref() == Some("AI"))
                    .unwrap_or_default()
                {
                    Some(Model {
                        label: "Unknown Model".to_string(),
                        metadata: IndexMap::default(),
                    })
                } else {
                    None
                }
            },
        })
    );

    for child in node.children {
        convert_node(weave, id_map, created, child, selected, chapters)?;
    }

    Ok(())
}

fn map_id(id_map: &mut HashMap<String, Ulid>, id: String, time: SystemTime) -> Ulid {
    match id_map.entry(id) {
        Entry::Occupied(occupied) => *occupied.get(),
        Entry::Vacant(vacant) => *vacant.insert_entry(Ulid::from_datetime(time)).get(),
    }
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

pub fn migrate_simple(
    input: &str,
    created: DateTime<Local>,
) -> anyhow::Result<Option<VersionedWeave>> {
    if let Ok(data) = serde_json::from_str::<PyloomSimpleNode>(input) {
        let mut output = new_weave_v0(16384, created, "PyLoomSimple");

        convert_export_node(&mut output, data, SystemTime::from(created), None);

        Ok(Some(output.to_versioned_weave()))
    } else {
        Ok(None)
    }
}

fn convert_export_node(
    weave: &mut TapestryWeave,
    node: PyloomSimpleNode,
    created: SystemTime,
    parent: Option<Ulid>,
) {
    let id = Ulid::from_datetime(created);

    assert!(weave.weave.add_node(DependentNode {
        id: id.0,
        from: parent.map(|id| id.0),
        to: IndexSet::default(),
        active: false,
        bookmarked: false,
        contents: NodeContent {
            content: InnerNodeContent::Snippet(node.text.into_bytes()),
            metadata: IndexMap::default(),
            model: None,
        },
    }));

    for child in node.children {
        convert_export_node(weave, child, created, Some(id));
    }
}
