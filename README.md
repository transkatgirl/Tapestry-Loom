# Tapestry Loom

An Obsidian plugin that aims to turn your editor into an IDE for working with base model LLMs.

> [!NOTE]
> This plugin is a work in progress, and may contain bugs and missing/broken functionality.

## Usage

> [!WARNING]
> This plugin relies on Obsidian's internal styling rules, and will likely have a broken interface on earlier or later versions than what it was built for. At the moment, this plugin is targeting Obsidian **1.8.10**.

### Building

- Clone this repo.
- Make sure your NodeJS is at least v16 (`node --version`).
- `npm i` or `yarn` to install dependencies.
- `npm run dev` to start compilation in watch mode, or `npm run build` to build a release copy.

### Installation

> [!IMPORTANT]
> It is recommended (but not required) that you create a dedicated Obsidian vault for this plugin.

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