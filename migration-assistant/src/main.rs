#![allow(non_snake_case)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local};
use clap::Parser;
use tapestry_weave::{universal_weave::indexmap::IndexMap, v0::TapestryWeave};
use walkdir::WalkDir;

mod exoloom;
mod loomsidian;
mod obsidian_tapestry;

#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Folder to scan for weaves to convert
    #[arg(short, long)]
    input: PathBuf,

    /// Folder to output migrated weaves into
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

                migrate_markdown_weave(entry.path(), &output)?;
            } else if extension == "json" {
                if let Some(parent) = output.parent() {
                    fs::create_dir_all(parent)?;
                }

                migrate_json_weave(entry.path(), &output)?;
            }
        }
    }

    Ok(())
}

fn migrate_markdown_weave(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    assert_ne!(input_path, output_path);

    let input = fs::read_to_string(input_path)?;
    let created: DateTime<Local> = DateTime::from(fs::metadata(input_path)?.created()?);

    if let Some(weave_data) = obsidian_tapestry::migrate(&input, created)? {
        println!("{} -> {}", input_path.display(), output_path.display());

        fs::write(output_path, weave_data)?;

        return Ok(());
    }

    println!("Skipping {}", input_path.display());

    Ok(())
}

fn migrate_json_weave(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    assert_ne!(input_path, output_path);

    let input = fs::read_to_string(input_path)?;
    let created: DateTime<Local> = DateTime::from(fs::metadata(input_path)?.created()?);

    {
        let output_weaves = loomsidian::migrate_all(&input, created)?;

        let has_outputs = !output_weaves.is_empty();

        for (filename, weave_data) in output_weaves {
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

            fs::write(output_path, weave_data)?;
        }

        if has_outputs {
            return Ok(());
        }
    }

    if let Some(weave_data) = loomsidian::migrate(&input, created)? {
        println!("{} -> {}", input_path.display(), output_path.display());

        fs::write(output_path, weave_data)?;

        return Ok(());
    }

    if let Some(weave_data) = exoloom::migrate(&input, created)? {
        println!("{} -> {}", input_path.display(), output_path.display());

        fs::write(output_path, weave_data)?;

        return Ok(());
    }

    // TODO

    println!("Skipping {}", input_path.display());

    Ok(())
}

fn new_weave(
    capacity: usize,
    created: DateTime<Local>,
    converted_from: &'static str,
) -> TapestryWeave {
    TapestryWeave::with_capacity(
        capacity,
        IndexMap::from([
            ("converted_from".to_string(), converted_from.to_string()),
            ("created".to_string(), created.to_rfc3339()),
            ("converted".to_string(), Local::now().to_rfc3339()),
        ]),
    )
}
