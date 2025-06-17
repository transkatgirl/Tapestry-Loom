import { debounce, Editor, MarkdownView, Notice, Plugin } from "obsidian";
import serialize from "serialize-javascript";
import {
	DEFAULT_DOCUMENT_SETTINGS,
	DEFAULT_SESSION_SETTINGS,
	TapestryLoomSettings,
	TapestryLoomSettingTab,
} from "settings";
import { TapestryLoomMainTreeView, TREE_LIST_VIEW_TYPE } from "view/tree";
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
} from "weave/format-v0";
import { deserialize } from "weave/common";
import { buildCommands } from "view/commands";
import { LIST_VIEW_TYPE, TapestryLoomListView } from "view/list";
import AwaitLock from "await-lock";

// @ts-expect-error
import elk from "cytoscape-elk";

export const DOCUMENT_LOAD_EVENT = "tapestry-document:load";
export const DOCUMENT_TRIGGER_UPDATE_EVENT = "tapestry-document:override";
export const DOCUMENT_TRIGGER_UPDATE_DEBOUNCE_EVENT =
	"tapestry-document:override-debounce";
export const DOCUMENT_UPDATE_EVENT = "tapestry-document:update";
export const DOCUMENT_DROP_EVENT = "tapestry-document:drop";
export const SETTINGS_UPDATE_EVENT = "tapestry-settings:update";

export default class TapestryLoom extends Plugin {
	settings: TapestryLoomSettings;
	editorPlugin: EditorPlugin;
	editor?: Editor = this.app.workspace.activeEditor?.editor;
	document?: WeaveDocument;
	lock = new AwaitLock();
	sessionSettings = structuredClone(DEFAULT_SESSION_SETTINGS);
	statusBar: HTMLElement = this.addStatusBarItem().createEl("span", {});
	async onload() {
		const { workspace } = this.app;

		await this.loadSettings();

		if (this.settings.defaultSession) {
			this.sessionSettings = structuredClone(
				this.settings.defaultSession
			);
			if (!this.sessionSettings.depth) {
				this.sessionSettings.depth = DEFAULT_SESSION_SETTINGS.depth;
			}
			if (
				this.settings.document &&
				this.settings.document.renderOverlayColors == undefined
			) {
				this.settings.document.renderOverlayColors = true;
				await this.saveSettings();
			}
		}

		cytoscape.use(elk);

		this.editorPlugin = buildEditorPlugin(this.settings, this.document);
		this.registerEditorExtension([this.editorPlugin]);

		if (this.editor) {
			try {
				this.document = await loadDocument(this.editor, true);
				workspace.trigger(DOCUMENT_LOAD_EVENT);
				updateEditorPluginState(
					this.editorPlugin,
					this.editor,
					this.settings,
					this.document
				);
			} catch (error) {
				new Notice(error);
			}
		}

		let debounceTime = DEFAULT_DOCUMENT_SETTINGS.debounce;
		if (this.settings.document) {
			debounceTime = this.settings.document.debounce;
		}

		this.registerEvent(
			workspace.on("active-leaf-change", async (leaf) => {
				if (leaf && leaf.view instanceof MarkdownView) {
					await this.lock.acquireAsync();
					try {
						this.editor = leaf.view.editor;
						const newDocument = await loadDocument(
							this.editor,
							true
						);

						if (
							this.document &&
							this.document.identifier ==
								newDocument.identifier &&
							this.document.getActiveContent() ==
								newDocument.getActiveContent()
						) {
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
							this.document = newDocument;
							workspace.trigger(DOCUMENT_LOAD_EVENT);
							updateEditorPluginState(
								this.editorPlugin,
								this.editor,
								this.settings,
								this.document
							);
						}
					} catch (error) {
						new Notice(error);
					} finally {
						this.lock.release();
					}
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
							await this.lock.acquireAsync();
							try {
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
							} catch (error) {
								new Notice(error);
							} finally {
								this.lock.release();
							}
						} else {
							await this.lock.acquireAsync();
							try {
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
							} catch (error) {
								new Notice(error);
							} finally {
								this.lock.release();
							}
						}
					},
					debounceTime,
					true
				)
			)
		);
		this.registerEvent(
			workspace.on("editor-drop", async (_evt, editor) => {
				await this.lock.acquireAsync();
				try {
					this.editor = undefined;
					this.document = undefined;
					workspace.trigger(DOCUMENT_DROP_EVENT);
					updateEditorPluginState(
						this.editorPlugin,
						editor,
						this.settings
					);
				} catch (error) {
					new Notice(error);
				} finally {
					this.lock.release();
				}
			})
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_TRIGGER_UPDATE_EVENT,
				async () => {
					if (this.editor && this.document) {
						await this.lock.acquireAsync();
						try {
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
						} catch (error) {
							new Notice(error);
						} finally {
							this.lock.release();
						}
					}
				}
			)
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_TRIGGER_UPDATE_DEBOUNCE_EVENT,
				debounce(
					() => {
						workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
					},
					debounceTime,
					false
				)
			)
		);

		this.registerView(
			TREE_LIST_VIEW_TYPE,
			(leaf) => new TapestryLoomMainTreeView(leaf, this)
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
			name: "Show tree view",
			callback: async () => {
				await this.showView(TREE_LIST_VIEW_TYPE);
			},
		});
		this.addCommand({
			id: "show-tapestry-loom-graph-view",
			name: "Show graph view",
			callback: async () => {
				await this.showView(GRAPH_VIEW_TYPE, true);
			},
		});
		this.addCommand({
			id: "show-tapestry-loom-sibling-list-view",
			name: "Show list view",
			callback: async () => {
				await this.showView(LIST_VIEW_TYPE);
			},
		});
		this.addCommand({
			id: "debug-tapestry-loom-decompress-document",
			name: "Debug: Save weave in plaintext representation",
			callback: async () => {
				if (this.editor && this.document) {
					await this.lock.acquireAsync();
					try {
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
					} catch (error) {
						new Notice(error);
					} finally {
						this.lock.release();
					}
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
		await this.lock.acquireAsync();
		try {
			const data = await this.loadData();

			if (data && "settings" in data) {
				this.settings = deserialize(data.settings);
			} else {
				this.settings = {};
			}
		} catch (error) {
			new Notice(error);
		} finally {
			this.lock.release();
		}
	}
	async saveSettings() {
		await this.lock.acquireAsync();
		try {
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
		} catch (error) {
			new Notice(error);
		} finally {
			this.lock.release();
		}
	}
}
