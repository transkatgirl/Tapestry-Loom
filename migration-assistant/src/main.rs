#![allow(non_snake_case)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local};
use clap::Parser;
use walkdir::WalkDir;

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

pub fn migrate_markdown_weave(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
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

pub fn migrate_json_weave(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    assert_ne!(input_path, output_path);

    // TODO

    println!("Skipping {}", input_path.display());

    Ok(())
}
