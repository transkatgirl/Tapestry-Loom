import { debounce, Editor, MarkdownView, Plugin } from "obsidian";
import serialize from "serialize-javascript";
import { deserialize } from "common";
import {
	DEFAULT_DOCUMENT_SETTINGS,
	DEFAULT_SESSION_SETTINGS,
	TapestryLoomSettings,
	TapestryLoomSettingTab,
} from "settings";
import { TapestryLoomTreeListView, TREE_LIST_VIEW_TYPE } from "view/tree";
import { TapestryLoomGraphView, GRAPH_VIEW_TYPE } from "view/graph";
import {
	buildEditorPlugin,
	EditorPlugin,
	updateEditorPluginState,
} from "view/editor";
import cytoscape from "cytoscape";
import {
	loadDocument,
	overrideEditorContent,
	updateDocument,
	WeaveDocument,
} from "document";
import { buildCommands } from "view/commands";
import { LIST_VIEW_TYPE, TapestryLoomListView } from "view/list";

// @ts-expect-error
import elk from "cytoscape-elk";

export const DOCUMENT_LOAD_EVENT = "tapestry-document:load";
export const DOCUMENT_TRIGGER_UPDATE_EVENT = "tapestry-document:override";
export const DOCUMENT_UPDATE_EVENT = "tapestry-document:update";
export const DOCUMENT_DROP_EVENT = "tapestry-document:drop";
export const SETTINGS_UPDATE_EVENT = "tapestry-settings:update";

export default class TapestryLoom extends Plugin {
	settings: TapestryLoomSettings;
	editorPlugin: EditorPlugin;
	editor?: Editor = this.app.workspace.activeEditor?.editor;
	document?: WeaveDocument;
	sessionSettings = DEFAULT_SESSION_SETTINGS;
	statusBar: HTMLElement = this.addStatusBarItem().createEl("span", {});
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
			this.document = await loadDocument(this.editor, true);
			workspace.trigger(DOCUMENT_LOAD_EVENT);
		}

		let debounceTime = DEFAULT_DOCUMENT_SETTINGS.debounce;
		if (this.settings.document) {
			debounceTime = this.settings.document.debounce;
		}

		this.editorPlugin = buildEditorPlugin(this.settings, this.document);
		this.registerEditorExtension([this.editorPlugin]);

		this.registerEvent(
			workspace.on("active-leaf-change", async (leaf) => {
				if (leaf && leaf.view instanceof MarkdownView) {
					this.editor = leaf.view.editor;
					const oldIdentifier = this.document?.identifier;
					this.document = await loadDocument(this.editor, true);

					if (this.document.identifier == oldIdentifier) {
						workspace.trigger(DOCUMENT_UPDATE_EVENT);
					} else {
						workspace.trigger(DOCUMENT_LOAD_EVENT);
					}
					updateEditorPluginState(
						this.editorPlugin,
						this.editor,
						this.settings,
						this.document
					);
				}
			})
		);
		this.registerEvent(
			workspace.on(
				"editor-change",
				debounce(
					async (editor) => {
						this.editor = editor;

						if (this.document) {
							if (
								await updateDocument(
									this.editor,
									this.document,
									true
								)
							) {
								workspace.trigger(DOCUMENT_UPDATE_EVENT);
								updateEditorPluginState(
									this.editorPlugin,
									this.editor,
									this.settings,
									this.document
								);
							}
						} else {
							this.document = await loadDocument(
								this.editor,
								true
							);
							workspace.trigger(DOCUMENT_LOAD_EVENT);
							updateEditorPluginState(
								this.editorPlugin,
								this.editor,
								this.settings,
								this.document
							);
						}
					},
					debounceTime,
					true
				)
			)
		);
		this.registerEvent(
			workspace.on("editor-drop", (_evt, editor) => {
				this.editor = undefined;
				this.document = undefined;
				workspace.trigger(DOCUMENT_DROP_EVENT);
				updateEditorPluginState(
					this.editorPlugin,
					editor,
					this.settings
				);
			})
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_TRIGGER_UPDATE_EVENT,
				async () => {
					if (this.editor && this.document) {
						await overrideEditorContent(
							this.editor,
							this.document,
							true
						);
						workspace.trigger(DOCUMENT_UPDATE_EVENT);
						updateEditorPluginState(
							this.editorPlugin,
							this.editor,
							this.settings,
							this.document
						);
					}
				}
			)
		);

		this.registerView(
			TREE_LIST_VIEW_TYPE,
			(leaf) => new TapestryLoomTreeListView(leaf, this)
		);
		this.registerView(
			GRAPH_VIEW_TYPE,
			(leaf) => new TapestryLoomGraphView(leaf, this)
		);
		this.registerView(
			LIST_VIEW_TYPE,
			(leaf) => new TapestryLoomListView(leaf, this)
		);
		Promise.all([
			this.showView(LIST_VIEW_TYPE),
			this.showView(TREE_LIST_VIEW_TYPE),
			this.showView(GRAPH_VIEW_TYPE, true),
		]).then(() => this.showView(TREE_LIST_VIEW_TYPE));

		this.addCommand({
			id: "show-tapestry-loom-tree-list-view",
			name: "Show node tree list view",
			callback: async () => {
				await this.showView(TREE_LIST_VIEW_TYPE);
			},
		});
		this.addCommand({
			id: "show-tapestry-loom-graph-view",
			name: "Show node graph view",
			callback: async () => {
				await this.showView(GRAPH_VIEW_TYPE, true);
			},
		});
		this.addCommand({
			id: "show-tapestry-loom-sibling-list-view",
			name: "Show node sibling list view",
			callback: async () => {
				await this.showView(LIST_VIEW_TYPE);
			},
		});
		this.addCommand({
			id: "debug-tapestry-loom-decompress-document",
			name: "Debug: Save weave in plaintext representation",
			callback: async () => {
				if (this.editor && this.document) {
					await overrideEditorContent(
						this.editor,
						this.document,
						false
					);
					workspace.trigger(DOCUMENT_UPDATE_EVENT);
					updateEditorPluginState(
						this.editorPlugin,
						this.editor,
						this.settings,
						this.document
					);
				}
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
		if (this.editor) {
			updateEditorPluginState(
				this.editorPlugin,
				this.editor,
				this.settings,
				this.document
			);
		}
	}
}
