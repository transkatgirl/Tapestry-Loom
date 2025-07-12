# Tapestry Loom (v1-alpha)

An Obsidian plugin that aims to turn your editor into an IDE for working with base model LLMs.

> [!NOTE]
> This plugin is a work in progress, and may contain bugs and missing/broken functionality.

## Included features

- Tree-based completion management
	- Nodes include metadata about the model and inference parameters used, along with a timestamp
	- Nodes include token probabilities when available
	- Prevents accidental deletion of generated nodes & preserves the context used to generate nodes
	- Support for bookmarking, splitting, and merging nodes
	- Completion tree storage within document
- List-based tree view, similar to [loomsidian](https://github.com/cosmicoptima/loom)
- Graph-based tree view, similar to [exoloom](https://exoloom.io)
- List-based node view, similar to [loomsidian](https://github.com/cosmicoptima/loom) and [exoloom](https://exoloom.io)
- Editor overlay view, similar to [loomsidian](https://github.com/cosmicoptima/loom)
	- Token probability display in editor, similar to [loom](https://github.com/socketteer/loom)
- Automatic creation of single-token probability nodes, similar to [logitloom](https://github.com/vgel/logitloom)
- Recursive completion generation, similar to [loom](https://github.com/socketteer/loom) and [logitloom](https://github.com/vgel/logitloom)
- Color coding by model used (requires a model color to be configured)
- Node metadata display on hover (currently only in list and editor), similar to [exoloom](https://exoloom.io)
- Flexible LLM API client
	- Support for completions with multiple different models at a time
	- Support for custom JSON/headers, similar to [loomsidian](https://github.com/cosmicoptima/loom)

<!-- use below feature list for v1-rc checklist

- DAG-based completion management
	- Nodes are immutable to prevent accidentally modifying completion history
		- Nodes can be bookmarked, split, and merged
		- Node contents are deduplicated and prefix-matching is automatically applied, similar to [Loom Engine](https://github.com/arcreflex/loom-engine)
	- Node metadata: Model + inference parameters used, time generated, and token probabilities (if available)
	- Nodes can be inserted at any point of the graph, similar to this [unreleased loom implementation](https://www.youtube.com/watch?v=xDPKR271jas&list=PLFoZLLI8ZnHCaSyopkws_9344avJQ_VEQ&index=19)
	- Diff mode: Changes to the weave are stored as a tree of diffs rather than a DAG, similar to [minihf's loom](https://github.com/JD-P/minihf)
	- Completion graph can be stored within document for easy sharing
- Tree-based graph view, similar to [loom](https://github.com/socketteer/loom) and [loomsidian](https://github.com/cosmicoptima/loom)
- Graph view, similar to [loom](https://github.com/socketteer/loom) and [exoloom](https://exoloom.io)
- Node list view, similar to [loomsidian](https://github.com/cosmicoptima/loom) and [exoloom](https://exoloom.io)
- Editor overlay view, similar to [loomsidian](https://github.com/cosmicoptima/loom)
	- Token probability display in editor, similar to [loom](https://github.com/socketteer/loom)
	- Sibling node list on hover in editor, similar to [loom](https://github.com/socketteer/loom)
	- Prompt token probability display in editor
- Automatic creation of single-token probability nodes, similar to [logitloom](https://github.com/vgel/logitloom)
- Support for embedding-based node sorting (inspired by this [blog post](https://www.lesswrong.com/posts/u2ww8yKp9xAB6qzcr/if-you-re-not-sure-how-to-sort-a-list-or-grid-seriate-it))
- Recursive completion generation, similar to [loom](https://github.com/socketteer/loom) and [logitloom](https://github.com/vgel/logitloom)
- Color coding by model used (requires a model color to be configured)
- Node metadata display on hover, similar to [exoloom](https://exoloom.io)
- Flexible LLM API client
	- Support for completions with multiple different models at a time
	- Support for custom JSON/headers, similar to [loomsidian](https://github.com/cosmicoptima/loom)

Note: In order to create a good interface, deciding what not to include is just as important as deciding what to include. Some approaches to interacting with base models (such as [minihf](https://github.com/JD-P/minihf) and [Loom Engine](https://github.com/arcreflex/loom-engine)) are fundamentally incompatible with the paradigm that Tapestry Loom adopts; Tapestry Loom is opinionated because it *has to be* in order to deliver a cohesive user experience.

-->

<details>
<summary>Screenshots</summary>

![](screenshots/Screenshot%202025-06-03T03-45-48%20-%20Obsidian.png)
![](screenshots/Screenshot%202025-06-03T03-46-12%20-%20Obsidian.png)
![](screenshots/Screenshot%202025-06-03T03-46-33%20-%20Obsidian.png)
![](screenshots/Screenshot%202025-06-03T03-47-26%20-%20Obsidian.png)
![](screenshots/Screenshot%202025-06-03T03-51-02%20-%20Obsidian.png)

</details>

### Development roadmap

Development on Tapestry Loom is currently paused due to factors outside of my control. When I am able to work on it again (or if I find new maintainers), development will resume.

Development may be intermittent, with long periods of inactivity between periods of development work.

#### Tapestry Loom v1-alpha checklist

The current target is to finish `v1-alpha` before the end of 2025.

- [ ] Improve Weave format to improve flexibility and efficiency:
	- [ ] Create `tapestry-weave` Rust library
		- [x] Write code for new Weave data structure & serialization+deserialization
			- [ ] Rewrite timeline code to use ~~LinkedList~~ ~~any_rope~~ flo_rope + Bytes (WIP)
			- [ ] Rewrite document code to use Entry whenever possible
		- [ ] Test & fix code
			- [ ] Implement unit tests for `content` module
			- [ ] Implement unit tests for `document` module
			- [ ] Implement unit tests for `update` module
			- [ ] Implement integration tests for `document` + `content` modules
			- [ ] Implement integration tests for `document` + `update` modules
			- [ ] Implement integration tests for `document` + `format` modules
	- [ ] Create `tapestry-loomkit` Rust WASM bindings
		- [ ] Implement weave data structure & saving+loading
		- [ ] Implement prefix deduplication overlay
		- [ ] Implement data structure for non-persistent Weave data
			- [ ] Improve handling of node activation/deactivation when node has multiple parents
			- [ ] Implement token streaming and display of nodes being generated
	- [ ] Implement new data structure into Tapestry Loom
		- [ ] Implement base data structure
			- [ ] Implement weave format v0 -> v1 conversion
			- [ ] Store weave in dedicated files by default, only store in document frontmatter if the user explicitly requests to do so
			- [ ] Rewrite async code to fix document switching race conditions
			- [ ] Allow graceful handling of editor undo/redo functionality
		- [ ] Add support for FIM completions, similar to this [unreleased loom implementation](https://www.youtube.com/watch?v=xDPKR271jas&list=PLFoZLLI8ZnHCaSyopkws_9344avJQ_VEQ&index=19)
		- [ ] Add support for diff weaves, similar to [minihf's loom](https://github.com/JD-P/minihf)
		- [ ] Add support for copying / moving nodes (and their children) to different parents
		- [ ] Implement multi-token multi-logprob generations as chains of single token nodes, inspired by [loom](https://github.com/socketteer/loom) and [logitloom](https://github.com/vgel/logitloom)
			- [ ] Update Node list to display sibings of node at cursor rather than last active node
			- [ ] Update graph to focus node at cursor rather than last active node
		- [ ] Add support for saving last-used model & parameter choices in Weave
	- [ ] Implement importing text from other documents, similar to [loom](https://github.com/socketteer/loom)
- [ ] Improve handling of tokenization boundaries
- [ ] Weave format stabilization & finalization
	- [ ] Update feature list
	- [ ] Update README to mention Rust libraries within repository
	- [ ] Merge v1 to main branch

#### Tapestry Loom v1-beta checklist

- [ ] Implement embedding model requests
	- [ ] Node ordering by [seriation](https://www.lesswrong.com/posts/u2ww8yKp9xAB6qzcr/if-you-re-not-sure-how-to-sort-a-list-or-grid-seriate-it)
	- [ ] Color coding by embeddings
- [ ] Add support for displaying prompt logprobs if returned by API
- [ ] UI improvements
	- [ ] Code cleanup
		- [ ] Remove reliance on Obsidian's undocumented styling rules
		- [ ] Rewrite UI using svelte?
	- [ ] Add node sorting options
		- [ ] Time added
		- [ ] Alphabetical
		- [ ] Semantic sort
	- [ ] Add color coding by token variety (number of tokens within 0.95 nucleus)
	- [ ] Improve list view to support all of the same functionality as the tree interface
		- [ ] Add hover buttons to list view
	- [ ] Allow quickly moving to node under cursor, similar to [exoloom](https://exoloom.io)
	- [ ] Allow showing sibling nodes on hover, similar to [loom](https://github.com/socketteer/loom)
	- [ ] Improve interactability of graph view to match [loom](https://generative.ink/posts/loom-interface-to-the-multiverse/)
		- [ ] Implement right click menu in graph view
		- [ ] Implement hover handling in graph view
			- [ ] Implement setting to only show node contents on hover, similar to [exoloom](https://exoloom.io)
		- [ ] Implement folding in graph view
	- [ ] Find out if it's possible to add options to text editor right click menu
		- [ ] Add node splitting to right click menu
	- [ ] Improve node finding
		- [ ] Add find dialog to graph view
	- [ ] Add parent nodes to list view
	- [ ] Scroll to newly generated nodes
	- [ ] Implement tree "unhoisting", similar to [loomsidian](https://github.com/cosmicoptima/loom)
	- [ ] Implement node "editing" UI (not actually editing node content, but editing the tree by adding nodes / splitting nodes / merging nodes), similar to [inkstream](https://inkstream.ai)
	- [ ] Implement "select node and generate completions" selection mode, similar to [inkstream](https://inkstream.ai)
	- [ ] Add Generation presets

#### Tapestry Loom v1-rc checklist

Tapestry Loom v1 will be the first stable version, and will be listed on the Obsidian community plugin registry.

- [ ] Document & selection analysis tools
	- [ ] Predictability analysis using logprobs
	- [ ] Statistical analysis of various metrics (model usage, text length, logprobs, number of branches, etc)
	- [ ] Weave metadata
- [ ] Support for [Standard Completions](https://standardcompletions.org) (after the specification is finalized)
	- [ ] If the specification is not ready by v1, support for Standard Completions will be delayed to v2
- [ ] Implement color palette generator for model colors
- [ ] Blind model comparison mode

#### Tapestry Loom Post-v1 plans

- Update `tapestry-weave` Rust library
	- Implement a zero-copy Weave format for memory mapping (to allow loading documents larger than the available system RAM)
- Update `tapestry-loomkit` library
	- Implement a client-server model for collaborative editing
		- Implement optional edit username tracking
		- Implement support for memory-mapped Weaves on the *server* side
- Implement support for the zero-copy Weave format, implement support for memory mapping Weaves on desktop
- Implement context window wrapping
- Implement mobile UI support

#### Tapestry Loom v2 plans

- Implement multi-platform support (Web, Desktop, Obsidian, Vim??, etc..)
	- Implement integration with some sort of local transcription plugin
- Implement collaborative editing
- Implement summarization of branches using chat-style LLMs to improve browsablity
- Alternately, try integrating the tree/graph and the editor more, similar to [loom](https://github.com/socketteer/loom)
	- Reduces strain on the user's working memory / reduces need for switching attention between UIs
- Allow adding roles to nodes when using chat-style LLM endpoints
	- Allow a chat weave session to reference another weave using LLM tool use (based on this [twitter thread](https://x.com/arcreflex_/status/1930671693707591767))
- Tooling for autolooms (looms where node choices are picked by another model)
- Add some sort of plugin API for building on top of Tapestry Loom???
- Integration with local LLM engines:
	- Option 1: Ollama integration
	- Option 2: An optional Tapestry Loom LLM server to handle running models locally using llama.cpp. This will likely only end up getting implemented if implementing [logprobs in Ollama](https://github.com/ollama/ollama/issues/2415) keeps getting delayed.
- Integration with external inference providers
	- Easy setup + built-in payment UI
	- May allow for monetization through provider profit sharing agreements without degrading user experience
		- Tapestry Loom will always stay FOSS
- Work towards making the UI more accessible for less technical users *without degrading the experience for technical users*
	- Inference parameter suggestions and explanations
	- Interactive sampling parameter visualization tool
	- UI hints for new users
	- UI streamlining *without reducing functionality*
- Aggressively remove friction points in the UI
- Built in user manual, based on the [cyborgism wiki](https://cyborgism.wiki) and any significant subsequent discoveries about base model behavior

## Usage

> [!WARNING]
> This plugin relies on Obsidian's internal styling rules, and will likely have a broken interface on earlier or later versions than what it was built for. At the moment, this plugin is targeting Obsidian **1.8.x**.

### Installation

> [!IMPORTANT]
> It is recommended that you create a dedicated Obsidian vault for this plugin.

1. Make sure community plugins are enabled within Obsidian.
2. Clone this git repository into your vault's `.obsidian/plugins/` folder.
3. Open a terminal in the plugin folder.
4. Run `wasm-pack build --release loomkit && npm install && npm run build` to fetch dependencies and build the plugin.
5. Open Obsidian's plugin settings and enable the plugin.

#### Updating

1. Open a terminal in the plugin folder.
2. Pull recent commits to the repository using `git pull`.
3. Run `wasm-pack build --release loomkit && npm install && npm run build` to fetch dependencies and build the plugin.
4. Open Obsidian's plugin settings and disable the plugin, then re-enable it.

### Post-install

After installing and enabling the plugin, you will need to do some post-install tasks to get the most out of it:

- Add your LLM model endpoints in the Tapestry Loom settings.
	- Unlike other LLM clients, the endpoint *must* be specified by the full URL rather than just the API prefix.
	- If you plan on using multiple models at a time, adding color labels to your models is recommended.
		- As a starting point, consider looking at [brand colors for popular models](./model%20colors.md)
- Open Obsidian's hotkey settings, and add hotkeys for frequently used Tapestry Loom commands.
	- It is strongly recommended that you at least add hotkeys for moving between nodes, splitting nodes, and generating completions.
- Find inference parameters you like, and then set them as your new defaults in the Tapestry Loom settings.