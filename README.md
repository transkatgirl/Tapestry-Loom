# Tapestry Loom

An Obsidian plugin that aims to turn your editor into an IDE for working with base model LLMs.

> [!NOTE]
> This plugin is a work in progress. Avoid using it with other plugins and backup your vaults regularly to avoid data loss.

## How to use

> [!WARNING]
> This plugin relies on Obsidian's internal styling rules, and will likely have a broken interface on earlier or later versions than what it was built for. At the moment, this plugin is targeting Obsidian **1.8.10**.

- Clone this repo.
- Make sure your NodeJS is at least v16 (`node --version`).
- `npm i` or `yarn` to install dependencies.
- `npm run dev` to start compilation in watch mode, or `npm run build` to build an optimized copy.


### Manually installing the plugin

- Copy over `main.js`, `styles.css`, and `manifest.json` to your vault `VaultFolder/.obsidian/plugins/tapestry-loom/`.
