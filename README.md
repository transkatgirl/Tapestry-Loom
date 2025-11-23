# Tapestry Loom (v1)

This branch contains is where work on Tapestry Loom's Version 1 rewrite is taking place.

## Plans

The plans for the rewrite are the following:
- [ ] Migrate to a collaborative (unauthenticated / LAN-only) WebUI rather than an Obsidian plugin
	- [ ] Implement conversion from the old Weave format to the new one
	- [ ] Implement all functionality supported by the original Obsidian plugin
- [ ] UI improvements
	- [ ] Implement default shortcuts from [loomsidian](https://github.com/cosmicoptima/loom)
	- [ ] Repeat keypresses when a keyboard shortcut is held down
	- [ ] Try applying colors to text background instead of text?
	- [ ] Add error message when when attempting to generate without any models selected
	- [ ] Implement multi-token multi-logprob generations as chains of single token nodes, inspired by [loom](https://github.com/socketteer/loom) and [logitloom](https://github.com/vgel/logitloom)
		- [ ] Update Node list to display sibings of node at cursor rather than last active node
	- [ ] Automatic color palette generation
	- [ ] Improve handling of trailing whitespace
	- [ ] Bring list view to feature parity with tree view
	- [ ] Integrate the tree/graph and the editor more, similar to [loom](https://github.com/socketteer/loom)
		- [ ] Implement tree "unhoisting", similar to [loomsidian](https://github.com/cosmicoptima/loom)
		- [ ] Update graph to focus node at cursor rather than last active node, similar to [exoloom](https://exoloom.io)
		- [ ] Show sibling nodes on hover, similar to [loom](https://github.com/socketteer/loom)
		- [ ] Improve interactability of graph view to match [loom](https://generative.ink/posts/loom-interface-to-the-multiverse/)
			- [ ] Implement right click menu in graph view
			- [ ] Implement hover handling in graph view
				- [ ] Implement setting to only show node contents on hover, similar to [exoloom](https://exoloom.io)
			- [ ] Implement folding in graph view
	- [ ] Improve node searching
	- [ ] Scroll to newly generated nodes
	- [ ] Implement "select node and generate completions" selection mode, similar to [inkstream](https://inkstream.ai)
- [ ] Better documentation & onboarding

In addition, below are the tentative plans for Tapestry Loom v2:
- [ ] Multi-user rather than single-user, multi-session
	- [ ] User authentication
	- [ ] User permissions
	- [ ] User rate limiting
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

To run Tapestry Loom, install npm and rust, and then run the following command:

```bash
sh run.sh
```

### Development

If the repository has been freshly cloned, run the the following commands first:

```bash
cd frontend
npm install
mkdir dist
cd ..
```

To run Tapestry Loom in development mode, run the following commands:

Terminal 1:

```bash
cargo run
```

Terminal 2:
```bash
cd frontend
npm run dev
```