# Tapestry Loom Migration Assistant

A tool for converting weaves from other Loom implementations into the Tapestry Loom weave format.

Fully supported input formats:
- [tapestry-weave](../tapestry-weave)
	- Binary (**Tapestry Loom format**)
		- Useful for upgrading outdated weaves
	- JSON
		- Useful for editing weaves using 3rd party tools
- [Legacy Tapestry Loom Obsidian plugin](https://github.com/transkatgirl/Tapestry-Loom/tree/legacy)

Partially supported input formats:
- [loom](https://github.com/socketteer/loom), tested with commit 91ca920551120ad4508540e8da057c0b94067afc
- [loomsidian](https://github.com/cosmicoptima/loom), tested with commit afbb3519f10d668d4688c68370d7b9305c9f80dc
- [exoloom](https://exoloom.io), last tested on December 17, 2025
	- Note: Exoloom's export format does not contain information on which nodes are active
- Please file any bugs that you find and/or submit PRs to improve format support

Supported output formats:
- [tapestry-weave](../tapestry-weave)
	- Binary (**Tapestry Loom format**, default)
	- JSON
		- Useful for editing weaves using 3rd party tools; *Cannot be read inside Tapestry Loom*

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

> [!WARNING]
> Conversion is often lossy and may contain bugs. **Do not delete your original input files after conversion.**

The migration assistant requires the following CLI arguments:

- \-\-input = Folder to scan for weaves to convert
- \-\-output = Folder to output migrated weaves into

The folder structure from the input folder will be replicated within the output folder.

Here is an example of the tool being run on the input folder `~/Documents/Obsidian`

```bash
./tapestry-migration-assistant --input ~/"Documents/Obsidian" --output ~/"Documents/Tapestry Loom/Migrated Weaves"
```
