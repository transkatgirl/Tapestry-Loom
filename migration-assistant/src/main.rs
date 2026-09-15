#![allow(non_snake_case)]

use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use clap::Parser;
use tapestry_weave::{
    TapestryWeave,
    jiff::Zoned,
    metadata::{AuxMetadataMap, ConvertedFrom, MetadataMap, WeaveMetadata},
    universal_weave::rkyv::util::AlignedVec,
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
            if args.output_debug_json {
                output.set_extension("json");
            } else {
                output.set_extension("tapestry");
            }

            if extension == "md" {
                if let Some(parent) = output.parent()
                    && !parent.as_os_str().is_empty()
                {
                    fs::create_dir_all(parent)?;
                }

                migrate_markdown_weave(entry.path(), &output, args.output_debug_json)?;
            } else if extension == "json" {
                if let Some(parent) = output.parent()
                    && !parent.as_os_str().is_empty()
                {
                    fs::create_dir_all(parent)?;
                }

                migrate_json_weave(entry.path(), &output, args.output_debug_json)?;
            } else if extension == "tapestry" {
                if let Some(parent) = output.parent()
                    && !parent.as_os_str().is_empty()
                {
                    fs::create_dir_all(parent)?;
                }

                migrate_tapestry_weave(entry.path(), &output, args.output_debug_json)?;
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
) -> TapestryWeave {
    TapestryWeave::with_capacity_and_metadata(
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
            aux_metadata: AuxMetadataMap::default(),
        },
    )
}

fn read_weave_from_file(path: &Path) -> anyhow::Result<Option<TapestryWeave>> {
    let mut file = File::open(path)?;
    let size = file
        .metadata()
        .map(|m| usize::try_from(m.len()).unwrap_or(usize::MAX))
        .ok();
    let mut bytes: AlignedVec<16> = AlignedVec::with_capacity(size.unwrap_or(0));
    bytes.extend_from_reader(&mut file)?;
    drop(file);

    if TapestryWeave::is_header_valid(&bytes) {
        Ok(Some(TapestryWeave::from_bytes(&bytes)?))
    } else {
        Ok(None)
    }
}

fn write_weave_to_file(path: &Path, weave: TapestryWeave, json: bool) -> anyhow::Result<()> {
    let file = File::create(path)?;
    let mut buffer = BufWriter::new(file);
    if json {
        buffer.write_all(&weave.to_json()?.into_bytes())?;
    } else {
        weave.to_bytes_in(&mut buffer)?;
    }

    buffer.flush()?;

    Ok(())
}

fn migrate_tapestry_weave(input_path: &Path, output_path: &Path, json: bool) -> anyhow::Result<()> {
    assert_ne!(input_path, output_path);

    println!("\n> {}", input_path.display());

    if let Some(weave) = read_weave_from_file(input_path)? {
        println!("{} -> {}", input_path.display(), output_path.display());

        write_weave_to_file(output_path, weave, json)?;
    } else {
        println!("Skipping {}", input_path.display());
    }

    Ok(())
}

fn migrate_markdown_weave(input_path: &Path, output_path: &Path, json: bool) -> anyhow::Result<()> {
    assert_ne!(input_path, output_path);

    let input = fs::read_to_string(input_path)?;

    println!("\n> {}", input_path.display());

    let created = Zoned::try_from(fs::metadata(input_path)?.created()?)?;
    if let Some(weave) = obsidian_tapestry::migrate(&input, created)? {
        println!("{} -> {}", input_path.display(), output_path.display());

        return write_weave_to_file(output_path, weave, json);
    }

    println!("Skipping {}", input_path.display());

    Ok(())
}

fn migrate_json_weave(input_path: &Path, output_path: &Path, json: bool) -> anyhow::Result<()> {
    assert_ne!(input_path, output_path);

    let input = fs::read_to_string(input_path)?;
    let created = Zoned::try_from(fs::metadata(input_path)?.created()?)?;
    println!("\n> {}", input_path.display());

    if let Ok(weave) = TapestryWeave::from_json_str(&input) {
        println!("{} -> {}", input_path.display(), output_path.display());

        return write_weave_to_file(output_path, weave, json);
    }

    {
        let output_weaves = loomsidian::migrate_all(&input, created.clone())?;

        let has_outputs = !output_weaves.is_empty();

        for (filename, weave) in output_weaves {
            let mut output_path = output_path
                .parent()
                .map(|p| p.to_owned())
                .unwrap_or_default()
                .join(filename);
            if json {
                output_path.set_extension("json");
            } else {
                output_path.set_extension("tapestry");
            }

            if let Some(parent) = output_path.parent()
                && !parent.as_os_str().is_empty()
            {
                fs::create_dir_all(parent)?;
            }

            println!("{} -> {}", input_path.display(), output_path.display());

            write_weave_to_file(&output_path, weave, json)?;
        }

        if has_outputs {
            return Ok(());
        }
    }

    if let Some(weave) = loomsidian::migrate(&input, created.clone())? {
        println!("{} -> {}", input_path.display(), output_path.display());

        return write_weave_to_file(output_path, weave, json);
    }

    if let Some(weave) = exoloom::migrate(&input, created.clone())? {
        println!("{} -> {}", input_path.display(), output_path.display());

        return write_weave_to_file(output_path, weave, json);
    }

    if let Some(weave) = pyloom::migrate(&input, created)? {
        println!("{} -> {}", input_path.display(), output_path.display());

        return write_weave_to_file(output_path, weave, json);
    }

    println!("Skipping {}", input_path.display());

    Ok(())
}
