#![allow(non_snake_case)]

use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::{HashMap, HashSet},
    hash::BuildHasherDefault,
    num::NonZeroU128,
};

use base64::prelude::*;
use boa_engine::{Context, JsString, Source, js_string, property::Attribute};
use frontmatter::{Yaml, parse_and_find_content};
use miniz_oxide::inflate::decompress_to_vec_zlib;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use stacksafe::stacksafe;
use tapestry_weave::{
    TapestryNode, TapestryWeave,
    content::{
        Creator, InnerNodeContent, InnerNodeToken, Model, NodeContent, OriginalToken,
        UNKNOWN_MODEL_LABEL,
    },
    hashers::{RandomIdHasher, UlidHasher},
    jiff::{Timestamp, Zoned},
    metadata::MetadataMap,
    nanorand::{Rng, WyRand},
    universal_weave::{
        MetadataWeave, Weave,
        indexmap::{IndexMap, IndexSet},
    },
    wrappers::UniqueIdentifierRemapper,
};
use ulid::Ulid;

use crate::new_weave;

pub fn migrate(input: &str, created: Zoned) -> anyhow::Result<Option<TapestryWeave>> {
    if let Ok((Some(Yaml::Hash(mut frontmatter)), _)) =
        parse_and_find_content(&normalize_line_endings(input))
    {
        let weave = if let Some(Yaml::String(compressed_weave)) =
            frontmatter.remove(&Yaml::String("TapestryLoomWeaveCompressed".to_string()))
        {
            Some(String::from_utf8(decompress_to_vec_zlib(
                &BASE64_STANDARD.decode(compressed_weave)?,
            )?)?)
        } else if let Some(Yaml::String(decompressed_weave)) =
            frontmatter.remove(&Yaml::String("TapestryLoomWeave".to_string()))
        {
            Some(decompressed_weave)
        } else {
            None
        };

        if let Some(weave) = weave {
            return Ok(Some(convert_weave(weave, created)?));
        }
    }

    Ok(None)
}

fn normalize_line_endings(input: &str) -> Cow<'_, str> {
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);

    if input.contains('\r') {
        Cow::Owned(input.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(input)
    }
}

fn convert_weave(input: String, created: Zoned) -> anyhow::Result<TapestryWeave> {
    let mut context = Context::default();

    context
        .register_global_property(
            js_string!("input_data"),
            JsString::from(input),
            Attribute::READONLY,
        )
        .unwrap();

    let output = context
        .eval(Source::from_bytes(include_bytes!("convert.js")))
        .map_err(|e| anyhow::Error::msg(format!("{}", e)))?
        .as_string()
        .ok_or(anyhow::Error::msg(
            "Incorrect return type from conversion script",
        ))?
        .to_std_string()?;

    let mut input: LegacyWeave = serde_json::from_value(serde_json::from_str::<Value>(&output)?)?; // Makes parsing untagged enums more reliable

    input.sort();

    let mut input_nodes = Vec::with_capacity(input.nodes.len());

    for node in &input.rootNodes {
        input.build_node_list(*node, &mut input_nodes);
    }

    let mut output = new_weave(input.nodes.len(), created, "LegacyTapestryLoom", None);

    let mut mapper: UniqueIdentifierRemapper<
        u128,
        u64,
        BuildHasherDefault<UlidHasher>,
        BuildHasherDefault<RandomIdHasher>,
    > = UniqueIdentifierRemapper::with_capacity(input.nodes.len());

    let time_zone = output.metadata().created.time_zone().clone();

    let mut rng = WyRand::new();

    let mut convert_old_identifier = move |id| {
        *mapper
            .map_with_initial(id, (id >> 64) as u64, || rng.generate())
            .get()
    };

    for node in input_nodes {
        if let Some(node) = input.nodes.get(&node).cloned() {
            let timestamp = Timestamp::try_from(Ulid(node.identifier.0).datetime())
                .map(|timestamp| Zoned::new(timestamp, time_zone.clone()))
                .unwrap_or(Zoned::default());

            assert!(
                output.insert(TapestryNode {
                    id: convert_old_identifier(node.identifier.0),
                    from: IndexSet::from_iter(
                        node.parentNode
                            .map(|id| convert_old_identifier(id.0))
                            .filter(|id| if output.contains(id) {
                                true
                            } else {
                                eprintln!("Warning: Node {} has missing parents", node.identifier);
                                false
                            })
                    ),
                    to: IndexSet::default(),
                    active: false,
                    bookmarked: input.bookmarks.contains(&node.identifier),
                    contents: NodeContent {
                        timestamp,
                        modified: false,
                        content: match node.content {
                            LegacyNodeContent::Snippet(snippet) =>
                                InnerNodeContent::Snippet(snippet.into_bytes()),
                            LegacyNodeContent::Tokens(tokens) => InnerNodeContent::Tokens(
                                tokens
                                    .into_iter()
                                    .map(|(probability, token)| {
                                        InnerNodeToken {
                                            bytes: token.into_bytes(),
                                            logprob: Some(probability.ln() as f32),
                                            id: None,
                                            entropy: None,
                                            counterfactual: vec![],
                                            original: OriginalToken::Unmodified,
                                        }
                                    })
                                    .collect()
                            ),
                        },
                        metadata: node.parameters.unwrap_or_default(),
                        aux_metadata: IndexMap::default(),
                        creator: node
                            .model
                            .map(|id| {
                                let model =
                                    input.models.get(&id).cloned().unwrap_or(LegacyModelLabel {
                                        label: UNKNOWN_MODEL_LABEL.to_string(),
                                        color: None,
                                    });

                                Creator::Model(Some(Model {
                                    label: model.label,
                                    color: model.color,
                                    identifier: NonZeroU128::try_from(id.0).ok(),
                                    metadata: IndexMap::default(),
                                    seed: None,
                                    system_fingerprint: None,
                                    finish_reason: None,
                                }))
                            })
                            .unwrap_or(Creator::User(None))
                    }
                })
            );
        }
    }

    if let Some(current) = input.currentNode {
        let current = convert_old_identifier(current.0);
        if output.contains(&current) {
            output.set_active_tree_semantics(&current, true);
        }
    }

    Ok(output)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LegacyWeave {
    identifier: Ulid,
    #[serde(default)]
    models: HashMap<Ulid, LegacyModelLabel>,
    #[serde(default)]
    modelNodes: HashMap<Ulid, HashSet<Ulid>>,
    #[serde(default)]
    nodes: HashMap<Ulid, LegacyDocumentNode>,
    #[serde(default)]
    rootNodes: Vec<Ulid>,
    #[serde(default)]
    nodeChildren: HashMap<Ulid, Vec<Ulid>>,
    currentNode: Option<Ulid>,
    #[serde(default)]
    bookmarks: HashSet<Ulid>,
}

impl LegacyWeave {
    fn sort(&mut self) {
        let mut roots = self
            .rootNodes
            .iter()
            .filter_map(|id| self.nodes.get(id))
            .collect();
        sort_node_list(&mut roots);
        self.rootNodes = roots.into_iter().map(|node| node.identifier).collect();

        for children in self.nodeChildren.values_mut() {
            let mut children_nodes = children
                .iter()
                .filter_map(|id| self.nodes.get(id))
                .collect();
            sort_node_list(&mut children_nodes);
            *children = children_nodes
                .into_iter()
                .map(|node| node.identifier)
                .collect();
        }
    }
    #[stacksafe]
    fn build_node_list(&self, id: Ulid, nodes: &mut Vec<Ulid>) {
        nodes.push(id);

        if let Some(children) = self.nodeChildren.get(&id) {
            for child in children {
                self.build_node_list(*child, nodes);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LegacyModelLabel {
    label: String,
    color: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LegacyDocumentNode {
    identifier: Ulid,
    content: LegacyNodeContent,
    model: Option<Ulid>,
    parentNode: Option<Ulid>,
    #[serde(alias = "metadata")]
    parameters: Option<MetadataMap>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
enum LegacyNodeContent {
    Snippet(String),
    Tokens(Vec<(f64, String)>),
}

fn sort_node_list(nodes: &mut Vec<&LegacyDocumentNode>) {
    nodes.sort_by(|a, b| {
        let a_tokens = if let LegacyNodeContent::Tokens(tokens) = &a.content {
            Some(tokens)
        } else {
            None
        };
        let b_tokens = if let LegacyNodeContent::Tokens(tokens) = &b.content {
            Some(tokens)
        } else {
            None
        };

        let x = a_tokens.map(|t| t.len()).unwrap_or_default() == 1;
        let y = b_tokens.map(|t| t.len()).unwrap_or_default() == 1;

        if x && y {
            a.model.cmp(&b.model).then(
                b_tokens.unwrap()[0]
                    .0
                    .partial_cmp(&a_tokens.unwrap()[0].0)
                    .unwrap_or(Ordering::Equal),
            )
        } else {
            a.model
                .cmp(&b.model)
                .then(x.cmp(&y))
                .then(a.identifier.cmp(&b.identifier))
        }
    });
}
