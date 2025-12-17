# Tapestry Loom Migration Assistant

A tool for converting weaves from other Loom implementations into the Tapestry Loom weave format.

Supported input formats:
- [Original Tapestry Loom](https://github.com/transkatgirl/Tapestry-Loom/tree/47561a4386ca9c0f09afb293d4e21eb4d7fe0c54)
- [loomsidian](https://github.com/cosmicoptima/loom)*

\* = Supported on a best-effort basis

## Getting Started

### Binary releases

See [tapestry-loom's documentation](../README.md) in order to download a binary release.

### Compiling from source

Once you are in the migration-assistant folder, you can build a binary with the following command:

```bash
cargo build --release
cp target/release/tapestry-loom-migration-assistant tapestry-migration-assistant
```

## Usage

The migration assistant requires the following CLI arguments:

- \-\-input = Folder to scan for weaves to convert
- \-\-output = Folder to output migrated weaves into

The folder structure from the input folder will be replicated within the output folder.

Here is an example of the tool being run on the input folder `~/Documents/Obsidian`

```bash
./tapestry-migration-assistant --input ~/"Documents/Obsidian" --output ~/"Documents/Tapestry Loom/Migrated Weaves"
```
