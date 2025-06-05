# Tapestry Loom

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

<details>
<summary>Screenshots</summary>

![](screenshots/Screenshot%202025-06-03T03-45-48%20-%20Obsidian.png)
![](screenshots/Screenshot%202025-06-03T03-46-12%20-%20Obsidian.png)
![](screenshots/Screenshot%202025-06-03T03-46-33%20-%20Obsidian.png)
![](screenshots/Screenshot%202025-06-03T03-47-26%20-%20Obsidian.png)
![](screenshots/Screenshot%202025-06-03T03-51-02%20-%20Obsidian.png)

</details>

### Planned features

Development on Tapestry Loom is currently paused due to factors outside of my control. When I am able to work on it again (or if I find new maintainers), development will resume.

#### Still in planning phase (may be implemented in any version)

- Generation presets
- Document analysis tools
- Blind model comparison mode
- Interactive sampling parameter visualizations
- Customizable node ordering

#### Tapestry Loom v1

Tapestry Loom v1 will be the first stable version, and will be listed on the Obsidian community plugin registry.

- Support for [Standard Completions](https://standardcompletions.org) (after the specification is finalized)
	- If this is not ready by v1, support for Standard Completions will be delayed to v2
	- Prompt logprobs support
- Improve weave storage:
	- Improve weave format; Implement efficient weave loading and saving
		- Implement binary nodes to improve handling of invalid unicode
	- Store weave in plugin database by default, only store in document frontmatter if the user explicitly requests to do so
	- Allow graceful handling of editor undo/redo functionality
- Improve weave flexibility:
	- Option 1: Store content diffs inside of nodes rather than raw text using [diff-match-patch](https://github.com/google/diff-match-patch), similar to [minihf's loom](https://github.com/JD-P/minihf)
	- Option 2: Store nodes in a DAG to allow for middle-of-text completions, similar to this [unreleased loom implementation](https://www.youtube.com/watch?v=xDPKR271jas&list=PLFoZLLI8ZnHCaSyopkws_9344avJQ_VEQ&index=19)
	- Option 3 (planned): **A hybrid approach**, storing nodes in a DAG while still implementing diff nodes. The user will be able to switch between the two for their own modifications, while FIM completions will be implemented as DAG nodes.
	- Weave data structure & serialization+deserialization will be rewritten as a Rust library loaded via WASM
- Prefix multiverse view: A global weave containing the first few nodes of all documents in the vault, similar to [Loom Engine](https://github.com/arcreflex/loom-engine)

#### Tapestry Loom v2

- Integration with local LLM engines:
	- Option 1: Ollama integration
	- Option 2: An optional Tapestry Loom LLM server to handle running models locally using llama.cpp. This will likely only end up getting implemented if implementing [logprobs in Ollama](https://github.com/ollama/ollama/issues/2415) keeps getting delayed.
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
4. Run `npm install && npm run build` to fetch dependencies and build the plugin.
5. Open Obsidian's plugin settings and enable the plugin.

#### Updating

1. Open a terminal in the plugin folder.
2. Pull recent commits to the repository using `git pull`.
3. Run `npm install && npm run build` to fetch dependencies and build the plugin.
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