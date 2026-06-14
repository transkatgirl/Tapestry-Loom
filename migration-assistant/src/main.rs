#![allow(non_snake_case)]

use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local};
use clap::Parser;
use jiff::Zoned;
use tapestry_weave::{
    VersionedInnerWeave, VersionedWeave,
    v1::{
        self,
        metadata::{ConvertedFrom, MetadataMap, WeaveMetadata},
    },
};
use walkdir::WalkDir;

mod exoloom;
mod loomsidian;
mod obsidian_tapestry;
mod pyloom;

#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Folder to scan for weaves to convert
    #[arg(short, long)]
    input: PathBuf,

    /// Folder to output migrated weaves into
    #[arg(short, long)]
    output: PathBuf,

    /// Don't upgrade weaves into newer versions of the Tapestry Loom format
    ///
    /// Weaves will always be deserialized and serialized regardless of this setting; This only determines whether or not format migration is performed during serialization
    #[arg(long)]
    no_upgrade: bool,

    /// Serialize weaves into JSON format
    ///
    /// JSON serialized weaves cannot be natively read by Tapestry Loom, but they are easier to modify and can be converted back into binary weaves using migration-assistant
    #[arg(long)]
    output_debug_json: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    fs::create_dir_all(&args.output)?;

    for entry in WalkDir::new(&args.input) {
        let entry = entry?;
        if entry.file_type().is_file()
            && let Some(extension) = entry.path().extension()
            && let Some(extension) = extension.to_ascii_lowercase().to_str()
        {
            let mut output = if let Ok(stripped_path) = entry.path().strip_prefix(&args.input) {
                args.output.clone().join(stripped_path)
            } else {
                args.output.clone().join(entry.file_name())
            };
            output.set_extension("tapestry");

            if extension == "md" {
                if let Some(parent) = output.parent() {
                    fs::create_dir_all(parent)?;
                }

                migrate_markdown_weave(entry.path(), &output, !args.no_upgrade)?;
            } else if extension == "json" {
                if let Some(parent) = output.parent() {
                    fs::create_dir_all(parent)?;
                }

                migrate_json_weave(entry.path(), &output, !args.no_upgrade)?;
            } else if extension == "tapestry" {
                if let Some(parent) = output.parent() {
                    fs::create_dir_all(parent)?;
                }

                assert_ne!(entry.path(), output);

                if let Some(weave) = VersionedWeave::from_bytes(&fs::read(entry.path())?) {
                    let weave = weave?;

                    println!("{} -> {}", entry.path().display(), output.display());

                    write_weave_to_file(&output, weave, !args.no_upgrade)?;
                } else {
                    println!("Skipping {}", entry.path().display());
                }
            }
        }
    }

    Ok(())
}

fn new_weave(
    capacity: usize,
    created: Zoned,
    source: &'static str,
    source_version: Option<&str>,
) -> v1::dependent::TapestryWeave {
    v1::dependent::TapestryWeave::with_capacity_and_metadata(
        capacity,
        WeaveMetadata {
            title: None,
            description: None,
            created,
            converted_from: vec![ConvertedFrom {
                source: source.to_string(),
                source_version: source_version.map(|v| v.to_string()),
                converter: env!("CARGO_PKG_NAME").to_string(),
                converter_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                timestamp: Zoned::now(),
            }],
            metadata: MetadataMap::default(),
        },
    )
}

fn write_weave_to_file(
    path: &Path,
    mut weave: VersionedWeave,
    upgrade: bool,
) -> anyhow::Result<()> {
    if upgrade {
        weave = weave.into_latest().to_versioned_weave();
    }

    let file = File::create(path)?;
    let mut buffer = BufWriter::new(file);
    weave.write_bytes(&mut buffer)?;
    buffer.flush()?;

    Ok(())
}

fn migrate_markdown_weave(
    input_path: &Path,
    output_path: &Path,
    upgrade: bool,
) -> anyhow::Result<()> {
    assert_ne!(input_path, output_path);

    let input = fs::read_to_string(input_path)?;
    let created: DateTime<Local> = DateTime::from(fs::metadata(input_path)?.created()?);

    if let Some(weave) = obsidian_tapestry::migrate(&input, created)? {
        println!("{} -> {}", input_path.display(), output_path.display());

        return write_weave_to_file(&output_path, weave, upgrade);
    }

    println!("Skipping {}", input_path.display());

    Ok(())
}

fn migrate_json_weave(input_path: &Path, output_path: &Path, upgrade: bool) -> anyhow::Result<()> {
    assert_ne!(input_path, output_path);

    let input = fs::read_to_string(input_path)?;
    let created: DateTime<Local> = DateTime::from(fs::metadata(input_path)?.created()?);

    if let Ok(weave) = serde_json::from_str::<VersionedInnerWeave>(&input) {
        write_weave_to_file(&output_path, weave.into_weave(), upgrade)?;
    }

    {
        let output_weaves = loomsidian::migrate_all(&input, created)?;

        let has_outputs = !output_weaves.is_empty();

        for (filename, weave) in output_weaves {
            let output_path = output_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_default()
                .join(filename)
                .with_extension("tapestry");

            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }

            println!("{} -> {}", input_path.display(), output_path.display());

            write_weave_to_file(&output_path, weave, upgrade)?;
        }

        if has_outputs {
            return Ok(());
        }
    }

    if let Some(weave) = loomsidian::migrate(&input, created)? {
        println!("{} -> {}", input_path.display(), output_path.display());

        return write_weave_to_file(&output_path, weave, upgrade);
    }

    if let Some(weave) = exoloom::migrate(&input, created)? {
        println!("{} -> {}", input_path.display(), output_path.display());

        return write_weave_to_file(&output_path, weave, upgrade);
    }

    if let Some(weave) = pyloom::migrate(&input, created)? {
        println!("{} -> {}", input_path.display(), output_path.display());

        return write_weave_to_file(&output_path, weave, upgrade);
    }

    if let Some(weave) = pyloom::migrate_simple(&input, created)? {
        println!("{} -> {}", input_path.display(), output_path.display());

        return write_weave_to_file(&output_path, weave, upgrade);
    }

    println!("Skipping {}", input_path.display());

    Ok(())
}
