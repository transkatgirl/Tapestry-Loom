#![allow(non_snake_case)]

use std::{
    path::PathBuf,
    time::{Duration, SystemTime},
};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{
        Weave,
        dependent::DependentNode,
        indexmap::{IndexMap, IndexSet},
    },
    v0::{InnerNodeContent, Model, NodeContent},
};
use uuid::Uuid;

use crate::new_weave;

pub fn migrate(input: &str, created: DateTime<Local>) -> anyhow::Result<Vec<(PathBuf, Vec<u8>)>> {
    if let Ok(data) = serde_json::from_str::<LoomsidianData>(input) {
        let mut output = Vec::with_capacity(data.state.len());

        for (filename, weave) in data.state {
            output.push((filename, convert_weave(weave, created)?));
        }

        Ok(output)
    } else {
        Ok(Vec::default())
    }
}

fn convert_weave(mut input: LoomsidianWeave, created: DateTime<Local>) -> anyhow::Result<Vec<u8>> {
    let mut id_map = IndexMap::with_capacity(input.nodes.len());

    for (id, _) in &input.nodes {
        build_node_list(&input, id, &mut id_map, SystemTime::from(created));
    }

    let mut output = new_weave(input.nodes.len(), created, "Loomsidian");

    for (id, new_id) in id_map.iter().map(|(a, b)| (*a, *b)) {
        let node = input.nodes.swap_remove(&id).unwrap();

        let parent = node.parentId.and_then(|parent| {
            if let Some(parent) = id_map.get(&parent) {
                Some(parent)
            } else {
                eprintln!("Warning: Node {} has missing parents", id);
                None
            }
        });

        assert!(output.weave.add_node(DependentNode {
            id: new_id.0,
            from: parent.map(|id| id.0),
            to: IndexSet::default(),
            active: input.current == id,
            bookmarked: node.bookmarked,
            contents: NodeContent {
                content: InnerNodeContent::Snippet(node.text.into_bytes()),
                metadata: IndexMap::default(),
                model: node.author.and_then(|author| {
                    if author != "genesis" && author != "N/A" {
                        Some(Model {
                            label: author,
                            metadata: IndexMap::default(),
                        })
                    } else {
                        None
                    }
                }),
            },
        }));
    }

    Ok(output.to_versioned_bytes()?)
}

fn build_node_list(
    weave: &LoomsidianWeave,
    node: &Uuid,
    nodes: &mut IndexMap<Uuid, Ulid>,
    created: SystemTime,
) {
    if nodes.contains_key(node) {
        return;
    }

    let id = *node;
    if let Some(node) = weave.nodes.get(node) {
        let new_id = if let Some(last_visited) = node.lastVisited {
            Ulid::from_datetime(SystemTime::UNIX_EPOCH + Duration::from_secs(last_visited))
        } else {
            Ulid::from_datetime(created)
        };

        if let Some(parent) = node.parentId {
            build_node_list(weave, &parent, nodes, created);
        }

        nodes.insert(id, new_id);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LoomsidianData {
    state: IndexMap<PathBuf, LoomsidianWeave>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LoomsidianWeave {
    current: Uuid,
    nodes: IndexMap<Uuid, LoomsidianNode>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LoomsidianNode {
    text: String,
    author: Option<String>,
    parentId: Option<Uuid>,

    #[serde(default)]
    bookmarked: bool,

    lastVisited: Option<u64>,
}
