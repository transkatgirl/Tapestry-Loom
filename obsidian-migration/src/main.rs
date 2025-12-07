use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;
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

fn migrate_weave(input: &Path, output: &Path) -> anyhow::Result<()> {
    let contents = fs::read_to_string(input)?;

    // TODO

    println!("{} -> {}", input.display(), output.display());

    Ok(())
}
