#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::{
    collections::{HashMap, hash_map::Entry},
    time::SystemTime,
};

use chrono::{DateTime, Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{
        Weave,
        dependent::DependentNode,
        indexmap::{IndexMap, IndexSet},
    },
    v0::{InnerNodeContent, Model, NodeContent, TapestryWeave},
};

use crate::new_weave;

pub fn migrate(input: &str, created: DateTime<Local>) -> anyhow::Result<Option<Vec<u8>>> {
    if let Ok(data) = serde_json::from_str::<PyloomWeave>(input) {
        let chapters: IndexMap<String, String> = data
            .chapters
            .into_iter()
            .map(|(id, chapter)| (id, chapter.title))
            .collect();

        let mut id_map = HashMap::with_capacity(65536);
        let mut output = new_weave(65536, created, "Loom");

        convert_node(
            &mut output,
            &mut id_map,
            data.root,
            &data.selected_node_id,
            &chapters,
        )?;

        Ok(Some(output.to_versioned_bytes()?))
    } else {
        Ok(None)
    }
}

fn convert_node(
    weave: &mut TapestryWeave,
    id_map: &mut HashMap<String, Ulid>,
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
        .map(SystemTime::from);
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

    let mut metadata = IndexMap::with_capacity(3);

    if let Some(meta) = &node.meta
        && let Some(modified) = meta.modified
    {
        metadata.insert("modified".to_string(), modified.to_string());
    }

    if let Some(chapter) = chapter {
        metadata.insert("chapter".to_string(), chapter.clone());
    }

    if !node.tags.is_empty() {
        metadata.insert("tags".to_string(), serde_json::to_string(&node.tags)?);
    }

    assert!(
        weave.weave.add_node(DependentNode {
            id: new_id.0,
            from: parent.map(|id| id.0),
            to: IndexSet::default(),
            active: &node.id == selected,
            bookmarked: chapter.is_some(),
            contents: NodeContent {
                content: InnerNodeContent::Snippet(node.text.into_bytes()),
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
        convert_node(weave, id_map, child, selected, chapters)?;
    }

    Ok(())
}

fn map_id(id_map: &mut HashMap<String, Ulid>, id: String, time: Option<SystemTime>) -> Ulid {
    match id_map.entry(id) {
        Entry::Occupied(occupied) => *occupied.get(),
        Entry::Vacant(vacant) => *vacant
            .insert_entry(if let Some(time) = time {
                Ulid::from_datetime(time)
            } else {
                Ulid::new()
            })
            .get(),
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
    children: Vec<PyloomNode>,
    meta: Option<PyloomMeta>,

    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PyloomMeta {
    creation_timestamp: Option<String>,
    source: Option<String>,
    modified: Option<bool>,
}
