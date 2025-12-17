#![allow(non_snake_case)]
#![allow(clippy::upper_case_acronyms)]

use std::{
    collections::{HashMap, HashSet},
    time::SystemTime,
};

use chrono::{DateTime, Local, Utc};
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

use crate::new_weave;

pub fn migrate(input: &str, created: DateTime<Local>) -> anyhow::Result<Option<Vec<u8>>> {
    if let Ok(mut data) = serde_json::from_str::<ExoloomWeave>(input) {
        assert!(data.loomType == "Exoloom" && data.schemaVersion == 1);

        let created = if let Some(created) = data.tree.createdAt {
            created.with_timezone(&Local)
        } else {
            created
        };

        let mut output = new_weave(data.tree.nodes.len(), created, "Exoloom");

        if let Some(version) = data.version {
            output
                .weave
                .metadata
                .insert("converted_from_version".to_string(), version);
        }

        let bookmarks: HashSet<u64> = data
            .lens
            .bookmarks
            .into_iter()
            .map(|bookmark| bookmark.nodeId)
            .collect();

        let pruned: HashSet<u64> = data
            .lens
            .prunedNodes
            .into_iter()
            .map(|pruned| pruned.nodeId)
            .collect();

        assert!(
            data.tree
                .nodes
                .get(&data.tree.rootNodeId)
                .unwrap()
                .parentId
                .is_none()
        );

        let mut id_map = IndexMap::with_capacity(data.tree.nodes.len());

        build_node_list(
            &data.tree,
            data.tree.rootNodeId,
            &mut id_map,
            SystemTime::from(created),
        );

        for (id, new_id) in id_map.iter().map(|(a, b)| (*a, *b)) {
            let node = data.tree.nodes.remove(&id).unwrap();
            let bookmarked = bookmarks.contains(&id);
            let pruned = pruned.contains(&id);

            let parent = node.parentId.map(|parent| id_map.get(&parent).unwrap());

            assert!(output.weave.add_node(DependentNode {
                id: new_id.0,
                from: parent.map(|id| id.0),
                to: IndexSet::default(),
                active: false,
                bookmarked,
                contents: NodeContent {
                    content: InnerNodeContent::Snippet(node.content.into_bytes()),
                    metadata: if pruned {
                        IndexMap::from_iter([("pruned".to_string(), "true".to_string())])
                    } else {
                        IndexMap::default()
                    },
                    model: if node.authorType == ExoloomAuthorType::LLM {
                        Some(Model {
                            label: node
                                .authorName
                                .unwrap_or_else(|| String::from("Unknown Model")),
                            metadata: IndexMap::default(),
                        })
                    } else {
                        None
                    },
                },
            }));
        }

        if let Some(title) = data.tree.title {
            output.weave.metadata.insert("title".to_string(), title);
        }

        if let Some(description) = data.tree.description {
            output
                .weave
                .metadata
                .insert("notes".to_string(), description);
        }

        Ok(Some(output.to_versioned_bytes()?))
    } else {
        Ok(None)
    }
}

fn build_node_list(
    weave: &ExoloomTree,
    node: u64,
    nodes: &mut IndexMap<u64, Ulid>,
    created: SystemTime,
) {
    if nodes.contains_key(&node) {
        return;
    }

    let id = node;
    if let Some(node) = weave.nodes.get(&node) {
        let new_id = if let Some(created_at) = node.createdAt {
            Ulid::from_datetime(SystemTime::from(created_at))
        } else {
            Ulid::from_datetime(created)
        };

        nodes.insert(id, new_id);

        for child in node.childrenIds.iter().copied() {
            build_node_list(weave, child, nodes, created);
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExoloomWeave {
    loomType: String,
    version: Option<String>,
    schemaVersion: usize,
    tree: ExoloomTree,
    lens: ExoloomLens,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExoloomTree {
    title: Option<String>,
    description: Option<String>,
    createdAt: Option<DateTime<Utc>>,
    nodes: HashMap<u64, ExoloomNode>,
    rootNodeId: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExoloomNode {
    content: String,
    authorType: ExoloomAuthorType,
    authorName: Option<String>,
    parentId: Option<u64>,
    childrenIds: Vec<u64>,
    createdAt: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ExoloomAuthorType {
    USER,
    LLM,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExoloomLens {
    bookmarks: Vec<ExoloomBookmark>,
    prunedNodes: Vec<ExoloomPrunedNode>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExoloomBookmark {
    nodeId: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExoloomPrunedNode {
    nodeId: u64,
}
