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
- Editor overlay view, similar to [loomsidian](https://github.com/cosmicoptima/loom)
	- Token probability display in editor, similar to [loom](https://github.com/socketteer/loom)
- Automatic creation of single-token probability nodes, similar to [logitloom](https://github.com/vgel/logitloom)
- Color coding by model used (requires a model color to be configured)
- Node metadata display on hover (currently only in list and editor), similar to [exoloom](https://exoloom.io)
- Flexible LLM API client
	- Support for completions with multiple different models at a time
	- Support for custom JSON/headers, similar to [loomsidian](https://github.com/cosmicoptima/loom)

You can see example screenshots on [this Twitter post](https://x.com/transkatgirl/status/1927328228802187329).

### Possible future features

- Generation presets
- Implementing a less restrictive tree updating algorithm, potentially based on [diff-match-patch](https://github.com/google/diff-match-patch), similar to [minihf's loom](https://github.com/JD-P/minihf)
	- Switching to a DAG and supporting middle-of-text completions, similar to this [unknown loom implementation](https://www.youtube.com/watch?£v=xDPKR271jas&list=PLFoZLLI8ZnHCaSyopkws_9344avJQ_VEQ&index=19)
- Prompt logprobs support
- Support for additional endpoint types (especially if they allow implementing additional features)
- Document analysis tools
- Interactive sampling parameter visualizations
- Built in user manual

## Usage

> [!WARNING]
> This plugin relies on Obsidian's internal styling rules, and will likely have a broken interface on earlier or later versions than what it was built for. At the moment, this plugin is targeting Obsidian **1.8.x**, and was last tested on Obsidian **1.8.10**.

### Building

1. Clone this repo.
2. Run `npm install` to install dependencies.
3. Run `npm run build` to build a release copy.

### Installation

> [!IMPORTANT]
> It is recommended that you create a dedicated Obsidian vault for this plugin.

1. Make sure community plugins are enabled within Obsidian.
2. Copy the `main.js`, `styles.css`, and `manifest.json` files into a folder named `tapestry-loom` within your vault's `.obsidian/plugins/` folder.
3. Open Obsidian's plugin settings, and enable the plugin.

### Post-install

After installing and enabling the plugin, you will need to do some post-install tasks to get the most out of it:

- Add your LLM model endpoints in the Tapestry Loom settings.
	- Unlike other LLM clients, the endpoint *must* be specified by the full URL rather than just the API prefix.
	- If you plan on using multiple models at a time, adding color labels to your models is recommended.
- Open Obsidian's hotkey settings, and add hotkeys for frequently used Tapestry Loom commands.
	- It is strongly recommended that you at least add hotkeys for moving between nodes, splitting nodes, and generating completions.
- Find inference parameters you like, and then set them as your new defaults in the Tapestry Loom settings.