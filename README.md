# Tapestry Loom

An IDE for working with base model LLMs, inspired by the designs of [loom](https://github.com/socketteer/loom), [loomsidian](https://github.com/cosmicoptima/loom), [exoloom](https://exoloom.io), [logitloom](https://github.com/vgel/logitloom), and [wool](https://github.com/lyramakesmusic/wool).

> [!WARNING]
> This is beta software. Most of it works, but there are plenty of undiscovered bugs and things will randomly break from time to time. Make backups.

## Known issues

- Some documents may cause the text editor to render token boundaries incorrectly
	- This seems to be due to a bug in egui regarding textedit underline rendering

## Getting started

### Binary releases

Compiled binaries can be found on the [releases page](https://github.com/transkatgirl/Tapestry-Loom/releases).

#### MacOS-specific instructions

Before using the app, you will need to mark the file as executable by running the following CLI commands in the folder containing the downloaded binary:

```bash
xattr -d com.apple.quarantine tapestry-loom-macos-aarch64
chmod +x tapestry-loom-macos-aarch64
```

#### Linux-specific instructions

Before using the app, you will need to mark the file as executable by running the following CLI command in the folder containing the downloaded binary:

```bash
chmod +x $FILENAME
```

Where `$FILENAME` matches the name of the binary you downloaded.

### Compiling from source

Requires the [Rust Programming Language](https://rust-lang.org/tools/install/) and a working C compiler to be installed.

```bash
git clone --recurse-submodules https://github.com/transkatgirl/Tapestry-Loom.git
cd Tapestry-Loom
cargo build --release
```

The compiled binary can be found in the ./target/release/ folder.

#### Updating

Run the following commands in the repository folder:

```bash
git pull
git submodule update --init --recursive
cargo build --release
```

## Usage

TODO: Make tutorial video

### Migrating weaves from the old Tapestry Loom Obsidian plugin

After compiling Tapestry Loom from source, run the following commands in the repository folder:

```bash
cd obsidian-migration
cargo run --release -- --input $OLD_TAPESTRY_OBSIDIAN_VAULT --output ~/"Documents/Tapestry Loom/Migrated Weaves"
```

Where `$OLD_TAPESTRY_OBSIDIAN_VAULT` is set to the location of the vault used by the Obsidian plugin.

## Plans

- [x] Migrate to a desktop app rather than an Obsidian plugin
	- [x] Implement conversion from the old Weave format to the new one
	- [x] Implement all functionality supported by the original Obsidian plugin
- [x] Full UI redesign
	- [x] Resizable, dragable, scrollable, and collapsible settings
	- [x] Three user-switchable UIs:
		- [x] Interactive canvas + textbox, similar to [wool](https://github.com/lyramakesmusic/wool)
			- [x] Implement dragable and scrollable canvas
			- [x] Implement draggable canvas nodes
				- [x] Implement canvas node left click
				- [x] Implement canvas node right click
			- [x] Implement canvas actions
			- [x] Implement resizable & dragable textbox
			- [x] Automatically adjust canvas position & highlighting based on textbox cursor location
			- [x] Automatically scroll textbox based on canvas cursor
			- [x] Scroll newly generated nodes into view
		- [x] Compact tree + compact treelist + textbox, similar to [loomsidian](https://github.com/cosmicoptima/loom) & old Tapestry Loom
			- [x] Implement resizable, dragable & scrollable tree
			- [x] Implement tree nodes
				- [x] Implement tree node content display on hover
				- [x] Implement tree node left click
				- [x] Implement tree node right click
			- [x] Implement resizable & scrollable treelist
			- [x] Implement resizable and scrollable textbox
			- [x] Automatically adjust tree position & highlighting based on textbox cursor location
			- [x] Automatically scroll textbox based on tree cursor location
			- [x] Scroll newly generated nodes into view
		- [x] Compact tree + node child list + textbox, similar to [exoloom](https://exoloom.io)
			- [x] Implement resizable, dragable & scrollable tree
			- [x] Implement tree nodes
				- [x] Implement tree node content display on hover
				- [x] Implement tree node left click
				- [x] Implement tree node right click
			- [x] Implement resizable and scrollable node child list
			- [x] Implement resizable and scrollable textbox
			- [x] Automatically adjust tree position & highlighting based on textbox cursor location
			- [x] Automatically scroll textbox based on tree cursor location
			- [x] Scroll newly generated nodes into view
	- [x] Weave metadata tab
	- [x] Better UI error handling
- [x] Keyboard shortcut implementation
	- [x] Automatically adapt keyboards shortcuts based on OS (such as Mac vs Windows/Linux)
	- [x] Repeat keypresses when a keyboard be cut is held down
- [ ] Store model output token IDs in nodes and reuse them when applicable
- [ ] Allows saving and switching between multiple inference presets
- [ ] Better documentation & onboarding
	- [ ] Tutorial video
	- [x] Binary releases

### Plans for next major version

<!--
- [ ] Server-client, multi-user WebUI
	- [ ] Support collaborating on Weaves
	- [ ] User authentication
	- [ ] User permissions
	- [ ] User rate limiting
- [ ] Event-based server-client communication to reduce bandwidth usage
- [ ] Automatic color palette generation in settings
- [ ] HTTPS support
- [ ] Compression support
	- [ ] Brotli compression for static assets
	- [ ] LZ4 compression for websocket data
-->

- [ ] Support for DAG-based Weaves, similar to this [unreleased loom implementation](https://www.youtube.com/watch?v=xDPKR271jas&list=PLFoZLLI8ZnHCaSyopkws_9344avJQ_VEQ&index=19)
	- [ ] FIM completions
		- [ ] Selected text is used to determine FIM location
	- [ ] Node copying & moving
	- [ ] Perform heavy testing of data structures and/or formal verification to prevent bugs that could result in data loss
	- [ ] Implement node "editing" UI (not actually editing node content, but editing the tree by adding nodes / splitting nodes / merging nodes), similar to [inkstream](https://inkstream.ai)
- [ ] Support for migrating weaves from other Loom implementations
	- [ ] [loomsidian](https://github.com/cosmicoptima/loom)
	- [ ] [exoloom](https://exoloom.io)
	- [ ] [loom](https://github.com/socketteer/loom)
	- [ ] [wool](https://github.com/lyramakesmusic/wool)
- [ ] Embedding model support
	- [ ] Node ordering by [seriation](https://www.lesswrong.com/posts/u2ww8yKp9xAB6qzcr/if-you-re-not-sure-how-to-sort-a-list-or-grid-seriate-it)
- [ ] Further UI improvements
	- [ ] Better file manager
	- [ ] Support keyboard shortcuts for all aspects of the UI, not just the weave editor
		- [ ] Aim to support navigating the entirety of the UI without a mouse
	- [ ] Node finding
	- [ ] Customizable node sorting
	- [ ] Node bulk selection
	- [ ] Node custom ordering via drag and drop
	- [ ] Show node siblings on hover in textbox
	- [ ] Keyboard shortcut presets
		- [ ] Built-in presets
			- [ ] [loomsidian](https://github.com/cosmicoptima/loom)-like
			- [ ] [exoloom](https://exoloom.io)-like
			- [ ] Tapestry Loom
		- [ ] Saving & loading custom presets
			- [ ] Importing & exporting custom presets
	- [ ] Support touchscreen-only devices
- [ ] Weave statistical analysis tools
- [ ] Optimize for performance whenever possible
	- [ ] Aim to have acceptable performance on weaves with ~1 million nodes, ~200k active and ~10MB of active text on low-end hardware (such as a Raspberry Pi)
		- [ ] Implement a special "link" node to allow splitting giant weaves into multiple documents
- [ ] Collaborative weave editing over LAN
- [ ] Adaptive looming using token entropy or [confidence](https://arxiv.org/pdf/2508.15260)
- [ ] Token streaming and display of nodes being generated
- [ ] Prefix-based duplication
- [ ] Undo/redo functionality
- [ ] Blind comparison modes
	- [ ] (Hide) Models & token probabilities / boundaries
	- [ ] (Hide) Generated node text (only showing metadata & probabilities)
- [ ] Allow adjusting proportion of completions from each model
	- [ ] Allow dynamically adjusting proportions based on usage
		- [ ] Flatten proportion bias when increasing number of completions, do the inverse when reducing completion count
- [ ] Allow adjusting model parameters for each model
	- [ ] Add node sorting options
		- [ ] Time added
		- [ ] Alphabetical
		- [ ] Semantic sort
	- [ ] Add color coding by token entropy or [confidence](https://arxiv.org/pdf/2508.15260)
	- [ ] Add more color coding customization
- [ ] Support alternate input devices
	- [ ] Talon Voice
	- [ ] Controllers / Gamepads
	- [ ] USB DDR Pads
- [ ] Document & selection analysis tools
	- [ ] Predictability analysis using logprobs
	- [ ] Statistical analysis of various metrics (model usage, text length, logprobs, number of branches, etc)
	- [ ] Weave metadata
- [ ] Implement context window wrapping
- [ ] Support for [Standard Completions](https://standardcompletions.org) (after the specification is finalized)
- [ ] Tooling for autolooms (looms where node choices are picked by another model or an algorithm)
- [ ] Add some sort of plugin API for building on top of Tapestry Loom???
- [ ] Implement an optional inference server using llama.cpp

See also: the [original rewrite plans](https://github.com/transkatgirl/Tapestry-Loom/blob/c8ccca0079ae186fcc7a70b955b2d2b123082d63/README.md)

Note: Tapestry Loom will be *entirely* focused on base and/or embedding models for the foreseeable future.

There are already good chat looms (such as [miniloom](https://github.com/JD-P/miniloom)) and base model looms which heavily integrate assistant functionality (such as [helm](https://github.com/Shoalstone/helm)); Tapestry Loom will **not** be one of them.