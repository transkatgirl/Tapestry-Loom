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
	- Completion graph can be stored within document for easy sharing
	- Proofreading Mode: Freezes the completion graph and stores further modifications as a diff
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

- [ ] Improve weave format to improve flexibility and efficiency:
	- [ ] Create Rust library implementing new weave data structure & serialization+deserialization
		- [ ] Implement unit testing for the Rust library
	- [ ] Implement new data structure into Tapestry Loom via WASM
		- [ ] Add support for binary nodes to improve handling of invalid unicode
		- [ ] Add support for FIM completions, similar to this [unreleased loom implementation](https://www.youtube.com/watch?v=xDPKR271jas&list=PLFoZLLI8ZnHCaSyopkws_9344avJQ_VEQ&index=19)
		- [ ] Add support for content diff nodes for user modifications, similar to [minihf's loom](https://github.com/JD-P/minihf)
		- [ ] Add support for displaying alternate node options on hover, similar to [loom](https://github.com/socketteer/loom)
			- [ ] Update Node list to display sibings of node at cursor rather than last active node
			- [ ] Update graph to focus node at cursor rather than last active node
	- [ ] Store weave in plugin database by default, only store in document frontmatter if the user explicitly requests to do so
		- [ ] Allow graceful handling of editor undo/redo functionality
	- [ ] Implement weave format v0 -> v1 conversion
- [ ] Weave format stabilization & finalization

#### Tapestry Loom v1-beta checklist

- [ ] Support for displaying prompt logprobs if returned by API
- [ ] Rewrite async code to fix document switching race conditions
- [ ] Embedding model support:
	- [ ] Node ordering by [seriation](https://www.lesswrong.com/posts/u2ww8yKp9xAB6qzcr/if-you-re-not-sure-how-to-sort-a-list-or-grid-seriate-it)
- [ ] Merging of v1 to main branch

#### Tapestry Loom v1-rc checklist

Tapestry Loom v1 will be the first stable version, and will be listed on the Obsidian community plugin registry.

- [ ] UI improvements
	- [ ] Code cleanup
		- [ ] Remove reliance on Obsidian's undocumented styling rules
		- [ ] Rewrite UI using svelte?
	- [ ] Add hover buttons to list view
	- [ ] Allow quickly moving to node under cursor, similar to [exoloom](https://exoloom.io)
	- [ ] Allow showing sibling nodes on hover, similar to [loom](https://github.com/socketteer/loom)
	- [ ] Improve interactability of graph view to match [loom](https://generative.ink/posts/loom-interface-to-the-multiverse/)
		- [ ] Implement right click menu in graph view
		- [ ] Implement hover handling in graph view
			- [ ] Implement setting to only show node contents on hover, similar to [exoloom](https://exoloom.io)
		- [ ] Implement folding in graph view
	- [ ] Add Generation presets
- [ ] Document & selection analysis tools
	- [ ] Predictability analysis using logprobs
	- [ ] Statistical analysis of various metrics (model usage, text length, logprobs, number of branches, etc)
	- [ ] Weave metadata
- [ ] Support for [Standard Completions](https://standardcompletions.org) (after the specification is finalized)
	- [ ] If the specification is not ready by v1, support for Standard Completions will be delayed to v2
- [ ] Blind model comparison mode

#### Tapestry Loom v2 plans

- Allow adding roles to nodes when using chat-style LLM endpoints
	- Allow a chat weave session to reference another weave using LLM tool use (based on this [twitter thread](https://x.com/arcreflex_/status/1930671693707591767))
- Integration with local LLM engines:
	- Option 1: Ollama integration
	- Option 2: An optional Tapestry Loom LLM server to handle running models locally using llama.cpp. This will likely only end up getting implemented if implementing [logprobs in Ollama](https://github.com/ollama/ollama/issues/2415) keeps getting delayed.
- Integration with external inference providers?
	- Easy setup + built-in payment UI
	- May allow for monetization through provider profit sharing agreements without degrading user experience
		- Tapestry Loom will always stay FOSS
- Work towards making the UI more accessible for less technical users *without degrading the experience for technical users*
	- Inference parameter suggestions and explanations
	- Interactive sampling parameter visualization tool
	- UI hints for new users
	- UI streamlining *without reducing functionality*
- Built in user manual, based on the [cyborgism wiki](https://cyborgism.wiki) and any significant subsequent discoveries about base model behavior

## Usage

> [!WARNING]
> This plugin relies on Obsidian's internal styling rules, and will likely have a broken interface on earlier or later versions than what it was built for. At the moment, this plugin is targeting Obsidian **1.8.x**, and was last tested on Obsidian **1.8.10**.

### Installation

> [!IMPORTANT]
> It is recommended that you create a dedicated Obsidian vault for this plugin.

1. Make sure community plugins are enabled within Obsidian.
2. Clone this git repository into your vault's `.obsidian/plugins/` folder.
3. Open a terminal in the plugin folder.
4. Run `wasm-pack build --release wasm && npm install && npm run build` to fetch dependencies and build the plugin.
5. Open Obsidian's plugin settings and enable the plugin.

#### Updating

1. Open a terminal in the plugin folder.
2. Pull recent commits to the repository using `git pull`.
3. Run `wasm-pack build --release wasm && npm install && npm run build` to fetch dependencies and build the plugin.
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