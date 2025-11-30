# Tapestry Loom (v1)

This branch contains is where work on Tapestry Loom's Version 1 rewrite is taking place.

## Plans

The plans for the rewrite are the following:
- [ ] Migrate to a desktop app rather than an Obsidian plugin
	- [ ] Implement conversion from the old Weave format to the new one
	- [ ] Implement all functionality supported by the original Obsidian plugin
- [ ] Full UI redesign
	- [x] Resizable, dragable, scrollable, and collapsible settings
	- [ ] Selection = generation mode, similar to [inkstream](https://inkstream.ai)
	- [ ] Three user-switchable UIs:
		- [ ] Interactive canvas + textbox, similar to [wool](https://github.com/lyramakesmusic/wool)
			- [ ] Implement dragable and scrollable canvas
			- [ ] Implement draggable canvas nodes
				- [ ] Implement canvas node left click
				- [ ] Implement canvas node right click
			- [ ] Implement canvas actions
			- [x] Implement resizable & dragable textbox
				- [ ] Show node siblings on hover
			- [ ] Automatically adjust canvas position & highlighting based on textbox cursor location
			- [ ] Automatically scroll textbox based on canvas cursor
			- [ ] Scroll newly generated nodes into view
		- [ ] Compact tree + compact treelist + textbox, similar to [loomsidian](https://github.com/cosmicoptima/loom) & old Tapestry Loom
			- [ ] Implement resizable, dragable & scrollable tree
			- [ ] Implement tree nodes
				- [ ] Implement tree node content display on hover
				- [ ] Implement tree node left click
				- [ ] Implement tree node right click
			- [x] Implement resizable & scrollable treelist
			- [x] Implement resizable and scrollable textbox
				- [ ] Show node siblings on hover
			- [ ] Automatically adjust tree position & highlighting based on textbox cursor location
			- [ ] Automatically scroll textbox based on tree cursor location
			- [ ] Scroll newly generated nodes into view
		- [ ] Compact tree + node child list + textbox, similar to [exoloom](https://exoloom.io)
			- [ ] Implement resizable, dragable & scrollable tree
			- [ ] Implement tree nodes
				- [ ] Implement tree node content display on hover
				- [ ] Implement tree node left click
				- [ ] Implement tree node right click
			- [x] Implement resizable and scrollable node child list
			- [x] Implement resizable and scrollable textbox
				- [ ] Show node siblings on hover
			- [ ] Automatically adjust tree position & highlighting based on textbox cursor location
			- [ ] Automatically scroll textbox based on tree cursor location
			- [ ] Scroll newly generated nodes into view
	- [ ] Node finding
	- [ ] Better error handling
		- [ ] Make UI operation failures visible
		- [ ] Make configuration errors visible
		- [ ] Trim trailing white space from nodes before generation
	- [ ] Experiments to try (may or may not end up being implemented):
		- [ ] Applying colors to background instead of text
		- [ ] Implement multi-token multi-logprob generations as chains of single token nodes, inspired by [loom](https://github.com/socketteer/loom) and [logitloom](https://github.com/vgel/logitloom)
- [ ] Keyboard shortcut implementation
	- [ ] Automatically adapt keyboards shortcuts based on OS (such as Mac vs Windows/Linux)
	- [ ] Repeat keypresses when a keyboard be cut is held down
	- [ ] Support shortcuts for all aspects of the UI, not just the weave editor
	- [ ] Multiple presets
		- [ ] [loomsidian](https://github.com/cosmicoptima/loom)-like
		- [ ] [exoloom](https://exoloom.io)-like
		- [ ] Tapestry Loom
	- [ ] Allows saving loading custom presets
	- [ ] Allow importing and exporting custom presets
- [ ] UI improvements
- [ ] Better documentation & onboarding

In addition, below are the tentative plans for Tapestry Loom v2:

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
- [ ] Mobile support
- [ ] Support for DAG-based Weaves, similar to this [unreleased loom implementation](https://www.youtube.com/watch?v=xDPKR271jas&list=PLFoZLLI8ZnHCaSyopkws_9344avJQ_VEQ&index=19)
	- [ ] FIM completions
	- [ ] Node copying & moving
	- [ ] Perform heavy testing of data structures and/or formal verification to prevent bugs that could result in data loss
	- [ ] Implement node "editing" UI (not actually editing node content, but editing the tree by adding nodes / splitting nodes / merging nodes), similar to [inkstream](https://inkstream.ai)
- [ ] Embedding model support
	- [ ] Node ordering by [seriation](https://www.lesswrong.com/posts/u2ww8yKp9xAB6qzcr/if-you-re-not-sure-how-to-sort-a-list-or-grid-seriate-it)
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

See also: the [original v1 plans](https://github.com/transkatgirl/Tapestry-Loom/blob/c8ccca0079ae186fcc7a70b955b2d2b123082d63/README.md)

## Usage

Note: This project is a work in progress. Missing features and major bugs are a certainty.

Make sure to clone the repository with submodules!
