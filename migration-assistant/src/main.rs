#![allow(non_snake_case)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;
use walkdir::WalkDir;

mod obsidian_tapestry;

#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Folder to scan for convertible weaves
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
            && let Some(extension) = extension.to_ascii_lowercase().to_str()
        {
            if extension == "md" {
                let mut output = if let Ok(stripped_path) = entry.path().strip_prefix(&args.input) {
                    args.output.clone().join(stripped_path)
                } else {
                    args.output.clone().join(entry.file_name())
                };
                output.set_extension("tapestry");

                if let Some(parent) = output.parent() {
                    fs::create_dir_all(parent)?;
                }

                migrate_markdown_weave(entry.path(), &output)?;
            } else if extension == "json" {
                // Soon
            }
        }
    }

    Ok(())
}

pub fn migrate_markdown_weave(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    assert_ne!(input_path, output_path);

    let input = fs::read_to_string(input_path)?;

    if let Some(weave_data) = obsidian_tapestry::migrate(&input)? {
        println!("{} -> {}", input_path.display(), output_path.display());

        fs::write(output_path, weave_data)?;

        return Ok(());
    }

    println!("Skipping {}", input_path.display());

    Ok(())
}
