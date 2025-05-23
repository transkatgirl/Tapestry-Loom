import { debounce, Editor, MarkdownView, Plugin } from "obsidian";
import serialize from "serialize-javascript";
import { deserialize } from "common";
import {
	DEFAULT_DOCUMENT_SETTINGS,
	DEFAULT_SESSION_SETTINGS,
	TapestryLoomSettings,
	TapestryLoomSettingTab,
} from "settings";
import { TapestryLoomTreeView, TREE_VIEW_TYPE } from "view/tree";
import { TapestryLoomGraphView, GRAPH_VIEW_TYPE } from "view/graph";
import { EDITOR_PLUGIN } from "view/editor";
import cytoscape from "cytoscape";
import {
	loadDocument,
	overrideEditorContent,
	updateDocument,
	WeaveDocument,
} from "document";
import { buildCommands } from "view/commands";

// @ts-expect-error
import elk from "cytoscape-elk";

export const DOCUMENT_LOAD_EVENT = "tapestry-document:load";
export const DOCUMENT_TRIGGER_UPDATE_EVENT = "tapestry-document:override";
export const DOCUMENT_UPDATE_EVENT = "tapestry-document:update";
export const DOCUMENT_DROP_EVENT = "tapestry-document:drop";
export const SETTINGS_UPDATE_EVENT = "tapestry-settings:update";

export default class TapestryLoom extends Plugin {
	settings: TapestryLoomSettings;
	editor?: Editor = this.app.workspace.activeEditor?.editor;
	document?: WeaveDocument;
	sessionSettings = DEFAULT_SESSION_SETTINGS;
	async onload() {
		const { workspace } = this.app;

		await this.loadSettings();

		if (this.settings.defaultSession) {
			this.sessionSettings = structuredClone(
				this.settings.defaultSession
			);
		}

		cytoscape.use(elk);

		if (this.editor) {
			this.document = loadDocument(this.editor);
			workspace.trigger(DOCUMENT_LOAD_EVENT);
		}

		let debounceTime = DEFAULT_DOCUMENT_SETTINGS.debounce;
		if (this.settings.document) {
			debounceTime = this.settings.document.debounce;
		}

		this.registerEvent(
			workspace.on("active-leaf-change", (leaf) => {
				if (leaf && leaf.view instanceof MarkdownView) {
					this.editor = leaf.view.editor;
					const oldIdentifier = this.document?.identifier;
					this.document = loadDocument(this.editor);
					if (this.document.identifier == oldIdentifier) {
						workspace.trigger(DOCUMENT_UPDATE_EVENT);
					} else {
						workspace.trigger(DOCUMENT_LOAD_EVENT);
					}
				}
			})
		);
		this.registerEvent(
			workspace.on(
				"editor-change",
				debounce(
					(editor) => {
						this.editor = editor;

						if (this.document) {
							if (updateDocument(this.editor, this.document)) {
								workspace.trigger(DOCUMENT_UPDATE_EVENT);
							}
						} else {
							this.document = loadDocument(this.editor);
							workspace.trigger(DOCUMENT_LOAD_EVENT);
						}
					},
					debounceTime,
					true
				)
			)
		);
		this.registerEvent(
			workspace.on("editor-drop", (_evt, _editor) => {
				this.editor = undefined;
				this.document = undefined;
				workspace.trigger(DOCUMENT_DROP_EVENT);
			})
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_TRIGGER_UPDATE_EVENT,
				() => {
					if (this.editor && this.document) {
						overrideEditorContent(this.editor, this.document);
						workspace.trigger(DOCUMENT_UPDATE_EVENT);
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
			name: "Show node tree view",
			callback: async () => {
				await this.showView(TREE_VIEW_TYPE);
			},
		});
		this.addCommand({
			id: "show-tapestry-loom-graph-view",
			name: "Show node graph view",
			callback: async () => {
				await this.showView(GRAPH_VIEW_TYPE, true);
			},
		});

		buildCommands(this).forEach((command) => this.addCommand(command));

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
		} else {
			this.settings = {};
		}
	}
	async saveSettings() {
		await this.saveData({ settings: serialize(this.settings) });
		this.app.workspace.trigger(SETTINGS_UPDATE_EVENT);
	}
}
