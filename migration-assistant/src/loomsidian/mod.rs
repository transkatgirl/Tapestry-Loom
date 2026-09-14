#![allow(non_snake_case)]

use std::{
    hash::BuildHasherDefault,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use stacksafe::stacksafe;
use tapestry_weave::{
    TapestryNode, TapestryWeave,
    content::{Creator, InnerNodeContent, Model, NodeContent},
    hashers::{RandomIdHasher, RandomState},
    jiff::{Timestamp, Zoned},
    nanorand::{Rng, WyRand},
    universal_weave::{
        MetadataWeave, Weave,
        indexmap::{IndexMap, IndexSet},
    },
    wrappers::UniqueIdentifierRemapper,
};
use uuid::Uuid;

use crate::new_weave;

pub fn migrate_all(input: &str, created: Zoned) -> anyhow::Result<Vec<(PathBuf, TapestryWeave)>> {
    if let Ok(data) =
        serde_json::from_str::<Value>(input).and_then(serde_json::from_value::<LoomsidianData>)
    // Makes parsing untagged enums more reliable
    {
        let presets = data.settings.into_preset_map();

        let mut output = Vec::with_capacity(data.state.len());

        for (filename, weave) in data.state {
            let title = filename
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned());

            output.push((
                filename,
                convert_weave(weave, created.clone(), &presets, title)?,
            ));
        }

        Ok(output)
    } else {
        Ok(Vec::default())
    }
}

pub fn migrate(input: &str, created: Zoned) -> anyhow::Result<Option<TapestryWeave>> {
    // Hack which makes parsing untagged enums more reliable
    if let Ok(data) =
        serde_json::from_str::<Value>(input).and_then(serde_json::from_value::<LoomsidianWeave>)
    {
        Ok(Some(convert_weave(
            data,
            created,
            &IndexMap::default(),
            None,
        )?))
    } else {
        Ok(None)
    }
}

type PresetMap = IndexMap<String, Option<LoomsidianModelPreset>>;

fn convert_weave(
    input: LoomsidianWeave,
    created: Zoned,
    presets: &PresetMap,
    title: Option<String>,
) -> anyhow::Result<TapestryWeave> {
    let mut nodes = input.nodes.into_map();

    let mut id_list = IndexSet::with_capacity(nodes.len());

    for (id, _) in &nodes {
        build_node_list(&nodes, id, &mut id_list);
    }

    let mut output = new_weave(nodes.len(), created, "Loomsidian", None);

    output.metadata_mut(|metadata| metadata.title = title);

    let mut mapper: UniqueIdentifierRemapper<
        Uuid,
        u64,
        RandomState,
        BuildHasherDefault<RandomIdHasher>,
    > = UniqueIdentifierRemapper::with_capacity(nodes.len());

    let time_zone = output.metadata().created.time_zone().clone();

    let mut rng = WyRand::new();

    let mut convert_old_identifier = move |id| *mapper.map(id, || rng.generate()).get();

    for id in id_list {
        let node = nodes.swap_remove(&id).unwrap();

        let timestamp = node
            .lastVisited
            .and_then(|unix_time| {
                Timestamp::try_from(SystemTime::UNIX_EPOCH + Duration::from_millis(unix_time)).ok()
            })
            .map(|timestamp| Zoned::new(timestamp, time_zone.clone()))
            .unwrap_or_default();

        let mut metadata = IndexMap::with_capacity_and_hasher(1, RandomState::default());

        if let Some(color) = node.color {
            metadata.insert("color".to_string(), color);
        }

        assert!(
            output.insert(TapestryNode {
                id: convert_old_identifier(id),
                from: IndexSet::from_iter(node.parentId.map(&mut convert_old_identifier).filter(
                    |parent_id| {
                        if output.contains(parent_id) {
                            true
                        } else {
                            eprintln!("Warning: Node {} has missing parents", id);
                            false
                        }
                    }
                )),
                to: IndexSet::default(),
                active: input.current == id,
                bookmarked: node.bookmarked,
                contents: NodeContent {
                    timestamp,
                    modified: false,
                    content: InnerNodeContent::Snippet(unescape_obsidian_markdown(
                        node.text.or(node.value).unwrap_or_default()
                    )),
                    metadata,
                    aux_metadata: IndexMap::default(),
                    creator: node
                        .author
                        .map(|author| {
                            if author != "genesis" && author != "N/A" {
                                Creator::Model(Some(convert_model(author, presets)))
                            } else {
                                Creator::User(None)
                            }
                        })
                        .unwrap_or(Creator::Unknown)
                },
            })
        );
    }

    Ok(output)
}

fn unescape_obsidian_markdown(text: String) -> Vec<u8> {
    if !text.contains('\\') {
        return text.into_bytes();
    }

    let bytes = text.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'\\'
            && let Some(next) = bytes.get(i + 1)
            && matches!(next, b'<' | b'[')
        {
            output.push(*next);
            i += 2;
        } else {
            output.push(bytes[i]);
            i += 1;
        }
    }

    output
}

fn convert_model(author: String, presets: &PresetMap) -> Model {
    let mut metadata = IndexMap::with_capacity_and_hasher(5, RandomState::default());

    if let Some(Some(preset)) = presets.get(&author) {
        let fields = [
            ("provider", &preset.provider),
            ("model", &preset.model),
            ("url", &preset.url),
            ("organization", &preset.organization),
            ("quantization", &preset.quantization),
        ];

        for (key, value) in fields {
            if let Some(value) = value
                && !value.is_empty()
            {
                metadata.insert(key.to_string(), value.clone());
            }
        }
    }

    Model {
        label: author,
        color: None,
        metadata,
        identifier: None,
        seed: None,
        system_fingerprint: None,
        finish_reason: None,
    }
}

#[stacksafe]
fn build_node_list(
    weave: &IndexMap<Uuid, LoomsidianNode>,
    node: &Uuid,
    nodes: &mut IndexSet<Uuid>,
) {
    if nodes.contains(node) {
        return;
    }

    let id = *node;
    if let Some(node) = weave.get(node) {
        if let Some(parent) = node.parentId {
            build_node_list(weave, &parent, nodes);
        }

        nodes.insert(id);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LoomsidianData {
    #[serde(default)]
    settings: LoomsidianSettings,

    state: IndexMap<PathBuf, LoomsidianWeave>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct LoomsidianSettings {
    #[serde(default)]
    modelPresets: Vec<LoomsidianModelPreset>,
}

impl LoomsidianSettings {
    fn into_preset_map(self) -> PresetMap {
        let mut presets: PresetMap = IndexMap::with_capacity(self.modelPresets.len());

        for preset in self.modelPresets {
            let name = preset.name.clone();

            match presets.get(&name) {
                None => {
                    presets.insert(name, Some(preset));
                }
                Some(Some(existing)) if *existing != preset => {
                    eprintln!(
                        "Warning: Multiple model presets are named {:?}; nodes authored by it will not be attributed to a specific preset",
                        name
                    );
                    presets.insert(name, None);
                }
                Some(_) => {}
            }
        }

        presets
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
struct LoomsidianModelPreset {
    name: String,
    provider: Option<String>,
    model: Option<String>,
    url: Option<String>,
    organization: Option<String>,
    quantization: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LoomsidianWeave {
    current: Uuid,
    nodes: LoomsidianNodes,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
enum LoomsidianNodes {
    Map(IndexMap<Uuid, LoomsidianNode>),
    List(Vec<LoomsidianListNode>),
}

impl LoomsidianNodes {
    fn into_map(self) -> IndexMap<Uuid, LoomsidianNode> {
        match self {
            Self::Map(map) => map,
            Self::List(list) => list
                .into_iter()
                .map(|node| {
                    (
                        node.id,
                        LoomsidianNode {
                            text: node.text,
                            value: node.value,
                            author: node.author,
                            parentId: node.parentId,
                            bookmarked: node.bookmarked,
                            color: node.color,
                            lastVisited: node.lastVisited,
                        },
                    )
                })
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LoomsidianListNode {
    id: Uuid,
    text: Option<String>,
    value: Option<String>,
    author: Option<String>,
    parentId: Option<Uuid>,

    #[serde(default)]
    bookmarked: bool,

    color: Option<String>,

    lastVisited: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LoomsidianNode {
    text: Option<String>,
    value: Option<String>,
    author: Option<String>,
    parentId: Option<Uuid>,

    #[serde(default)]
    bookmarked: bool,

    color: Option<String>,

    lastVisited: Option<u64>,
}
