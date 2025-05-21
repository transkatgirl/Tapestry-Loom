import { debounce, Editor, MarkdownView, Plugin } from "obsidian";
import serialize from "serialize-javascript";
import { deserialize } from "common";
import { TapestryLoomSettings, TapestryLoomSettingTab } from "settings";
import { TapestryLoomTreeView, TREE_VIEW_TYPE } from "view/tree";
import { TapestryLoomGraphView, GRAPH_VIEW_TYPE } from "view/graph";
import { EDITOR_COMMANDS, EDITOR_PLUGIN } from "view/editor";
import cytoscape from "cytoscape";
import dagre from "cytoscape-dagre";
import {
	loadDocument,
	overrideEditorContent,
	updateDocument,
	WeaveDocument,
} from "document";

export const DOCUMENT_LOAD_EVENT = "tapestry-document:load";
export const DOCUMENT_TRIGGER_UPDATE_EVENT = "tapestry-document:override";
export const DOCUMENT_UPDATE_EVENT = "tapestry-document:update";
export const DOCUMENT_DROP_EVENT = "tapestry-document:drop";
export const SETTINGS_UPDATE_EVENT = "tapestry-settings:update";

export default class TapestryLoom extends Plugin {
	settings: TapestryLoomSettings;
	editor?: Editor = this.app.workspace.activeEditor?.editor;
	document?: WeaveDocument;
	async onload() {
		await this.loadSettings();

		cytoscape.use(dagre);

		if (this.editor) {
			this.document = loadDocument(this.editor);
			this.app.workspace.trigger(DOCUMENT_LOAD_EVENT);
		}

		this.registerEvent(
			this.app.workspace.on("active-leaf-change", (leaf) => {
				if (leaf && leaf.view instanceof MarkdownView) {
					this.editor = leaf.view.editor;
					this.document = loadDocument(this.editor);
					this.app.workspace.trigger(DOCUMENT_LOAD_EVENT);
				}
			})
		);
		this.registerEvent(
			this.app.workspace.on(
				"editor-change",
				debounce(
					(editor) => {
						this.editor = editor;

						if (this.document) {
							if (updateDocument(this.editor, this.document)) {
								this.app.workspace.trigger(
									DOCUMENT_UPDATE_EVENT
								);
							}
						} else {
							this.document = loadDocument(this.editor);
							this.app.workspace.trigger(DOCUMENT_LOAD_EVENT);
						}
					},
					500, // TODO: Add setting for debounce time
					true
				)
			)
		);
		this.registerEvent(
			this.app.workspace.on("editor-drop", (_evt, _editor) => {
				this.editor = undefined;
				this.document = undefined;
				this.app.workspace.trigger(DOCUMENT_DROP_EVENT);
			})
		);
		this.registerEvent(
			this.app.workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_TRIGGER_UPDATE_EVENT,
				() => {
					if (this.editor && this.document) {
						overrideEditorContent(this.editor, this.document);
						this.app.workspace.trigger(DOCUMENT_UPDATE_EVENT);
					}
				}
			)
		);

		this.registerView(
			TREE_VIEW_TYPE,
			(leaf) => new TapestryLoomTreeView(leaf, this)
		);
		this.registerView(
			GRAPH_VIEW_TYPE,
			(leaf) => new TapestryLoomGraphView(leaf, this)
		);
		Promise.all([
			this.showView(TREE_VIEW_TYPE),
			this.showView(GRAPH_VIEW_TYPE, true),
		]).then(() => this.showView(TREE_VIEW_TYPE));

		this.registerEditorExtension([EDITOR_PLUGIN]);

		this.addCommand({
			id: "show-tapestry-loom-tree-view",
			name: "Show Tapestry Loom tree view",
			callback: async () => {
				await this.showView(TREE_VIEW_TYPE);
			},
		});
		this.addCommand({
			id: "show-tapestry-loom-graph-view",
			name: "Show Tapestry Loom graph view",
			callback: async () => {
				await this.showView(GRAPH_VIEW_TYPE, true);
			},
		});

		EDITOR_COMMANDS.forEach((command) => this.addCommand(command));

		this.addSettingTab(new TapestryLoomSettingTab(this.app, this));
	}
	onunload() {}
	async showView(viewType: string, left?: boolean) {
		const { workspace } = this.app;

		const leaves = workspace.getLeavesOfType(viewType);

		if (leaves.length > 0) {
			workspace.revealLeaf(leaves[0]);
		} else {
			let leaf;
			if (left) {
				leaf = workspace.getLeftLeaf(true);
			} else {
				leaf = workspace.getRightLeaf(false);
			}
			await leaf?.setViewState({ type: viewType, active: true });
			if (leaf) {
				workspace.revealLeaf(leaf);
			}
		}
	}
	async loadSettings() {
		const data = await this.loadData();

		if (data && "settings" in data) {
			this.settings = deserialize(data.settings);
		}
	}
	async saveSettings() {
		await this.saveData({ settings: serialize(this.settings) });
		this.app.workspace.trigger(SETTINGS_UPDATE_EVENT);
	}
}
