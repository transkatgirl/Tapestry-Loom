#![allow(non_snake_case)]

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use base64::prelude::*;
use boa_engine::{Context, JsString, JsValue, Source, js_string, property::Attribute};
use clap::Parser;
use frontmatter::{Yaml, parse_and_find_content};
use miniz_oxide::inflate::decompress_to_vec_zlib;
use serde::{Deserialize, Serialize};
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{
        dependent::DependentNode,
        indexmap::{IndexMap, IndexSet},
    },
    v0::{InnerNodeContent, Model, NodeContent, TapestryWeave},
};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Folder to scan for weaves created by Tapestry Loom v0
    #[arg(short, long)]
    input: PathBuf,

    /// Folder to output migrated weaves
    #[arg(short, long)]
    output: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    fs::create_dir_all(&args.output)?;

    for entry in WalkDir::new(&args.input) {
        let entry = entry?;
        if entry.file_type().is_file()
            && let Some(extension) = entry.path().extension()
            && extension.to_ascii_lowercase().to_str() == Some("md")
        {
            let mut output = if let Ok(stripped_path) = entry.path().strip_prefix(&args.input) {
                args.output.clone().join(stripped_path)
            } else {
                args.output.clone().join(entry.file_name())
            };
            output.set_extension("tapestry");

            if let Some(parent) = output.parent() {
                fs::create_dir_all(&parent)?;
            }

            migrate_weave(entry.path(), &output)?;
        }
    }

    Ok(())
}

fn migrate_weave(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    assert_ne!(input_path, output_path);

    let input = fs::read_to_string(input_path)?;

    if let Ok((Some(Yaml::Hash(mut frontmatter)), _)) = parse_and_find_content(&input) {
        let weave = if let Some(Yaml::String(compressed_weave)) =
            frontmatter.remove(&Yaml::String("TapestryLoomWeaveCompressed".to_string()))
        {
            Some(String::from_utf8(
                decompress_to_vec_zlib(&BASE64_STANDARD.decode(compressed_weave)?)
                    .map_err(|e| anyhow::Error::msg(format!("{}", e)))?,
            )?)
        } else if let Some(Yaml::String(decompressed_weave)) =
            frontmatter.remove(&Yaml::String("TapestryLoomWeave".to_string()))
        {
            Some(decompressed_weave)
        } else {
            None
        };

        if let Some(weave) = weave {
            println!("{} -> {}", input_path.display(), output_path.display());

            fs::write(output_path, convert_weave(weave)?)?;
        }
    }

    Ok(())
}

fn convert_weave(input: String) -> anyhow::Result<Vec<u8>> {
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

    let input: LegacyWeave = serde_json::from_str(&output)?;

    let output = TapestryWeave::with_capacity(
        16384,
        IndexMap::from([(
            "converted_from".to_string(),
            "LegacyTapestryLoom".to_string(),
        )]),
    );

    // TODO

    //println!("{:#?}", input);

    Ok(vec![])
}

#[derive(Serialize, Deserialize, Debug)]
struct LegacyWeave {
    identifier: Ulid,
    models: HashMap<Ulid, LegacyModelLabel>,
    modelNodes: HashMap<Ulid, HashSet<Ulid>>,
    nodes: HashMap<Ulid, LegacyDocumentNode>,
    rootNodes: HashSet<Ulid>,
    nodeChildren: HashMap<Ulid, HashSet<Ulid>>,
    currentNode: Option<Ulid>,
    bookmarks: HashSet<Ulid>,
}

#[derive(Serialize, Deserialize, Debug)]
struct LegacyModelLabel {
    label: String,
    color: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct LegacyDocumentNode {
    identifier: Ulid,
    content: LegacyNodeContent,
    model: Option<Ulid>,
    parentNode: Option<Ulid>,
    parameters: Option<IndexMap<String, String>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum LegacyNodeContent {
    Snippet(String),
    Tokens(Vec<(f64, String)>),
}
