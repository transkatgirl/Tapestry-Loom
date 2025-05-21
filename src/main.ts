import { debounce, Editor, MarkdownView, Plugin } from "obsidian";
import serialize from "serialize-javascript";
import { deserialize } from "common";
import { TapestryLoomSettings, TapestryLoomSettingTab } from "settings";
import {
	TapestryLoomView,
	VIEW_TYPE,
	VIEW_COMMANDS,
	EDITOR_PLUGIN,
} from "view";
import cytoscape from "cytoscape";
import dagre from "cytoscape-dagre";
import {
	loadDocument,
	overrideEditorContent,
	updateDocument,
	WeaveDocument,
} from "document";

export default class TapestryLoom extends Plugin {
	settings: TapestryLoomSettings;
	editor?: Editor = this.app.workspace.activeEditor?.editor;
	document?: WeaveDocument;
	async onload() {
		await this.loadSettings();

		cytoscape.use(dagre);

		if (this.editor) {
			this.document = loadDocument(this.editor);
			this.app.workspace.trigger("tapestry-document:load");
		}

		this.registerEvent(
			this.app.workspace.on("active-leaf-change", (leaf) => {
				if (leaf && leaf.view instanceof MarkdownView) {
					this.editor = leaf.view.editor;
					this.document = loadDocument(this.editor);
					this.app.workspace.trigger("tapestry-document:load");
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
									"tapestry-document:update"
								);
							}
						} else {
							this.document = loadDocument(this.editor);
							this.app.workspace.trigger(
								"tapestry-document:load"
							);
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
				this.app.workspace.trigger("tapestry-document:drop");
			})
		);
		this.registerEvent(
			this.app.workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				"tapestry-document:override",
				() => {
					if (this.editor && this.document) {
						overrideEditorContent(this.editor, this.document);
						this.app.workspace.trigger("tapestry-document:update");
					}
				}
			)
		);
		this.registerView(
			VIEW_TYPE,
			(leaf) => new TapestryLoomView(leaf, this)
		);

		this.showView();

		this.registerEditorExtension([EDITOR_PLUGIN]);

		this.addCommand({
			id: "show-tapestry-loom-view",
			name: "Show Tapestry Loom",
			callback: async () => {
				await this.showView();
			},
		});

		VIEW_COMMANDS.forEach((command) => this.addCommand(command));

		this.addSettingTab(new TapestryLoomSettingTab(this.app, this));
	}
	onunload() {}
	async showView() {
		const { workspace } = this.app;

		const leaves = workspace.getLeavesOfType(VIEW_TYPE);

		if (leaves.length > 0) {
			workspace.revealLeaf(leaves[0]);
		} else {
			const leaf = workspace.getRightLeaf(false);
			await leaf?.setViewState({ type: VIEW_TYPE, active: true });
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
	}
}
