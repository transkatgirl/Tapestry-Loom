import { Command, Editor, ItemView, WorkspaceLeaf } from "obsidian";
import TapestryLoom from "main";
import {
	App,
	ItemView,
	Menu,
	Modal,
	Setting,
	WorkspaceLeaf,
	setIcon,
} from "obsidian";
import { Range } from "@codemirror/state";
import {
	Decoration,
	DecorationSet,
	ViewUpdate,
	EditorView,
	ViewPlugin,
	PluginSpec,
	PluginValue,
	WidgetType,
} from "@codemirror/view";

export const commandSet: Array<Command> = [];

export const VIEW_TYPE = "tapestry-loom-view";

export class TapestryLoomView extends ItemView {
	plugin: TapestryLoom;

	constructor(leaf: WorkspaceLeaf, plugin: TapestryLoom) {
		super(leaf);
		this.plugin = plugin;
	}

	getViewType() {
		return VIEW_TYPE;
	}

	getDisplayText() {
		return "Tapestry Loom";
	}

	getIcon(): string {
		return "list-tree";
	}

	async updateDocument(editor: Editor) {}

	async onOpen() {
		const container = this.containerEl.children[1];
		container.empty();
		container.createEl("h4", { text: "Created on " + Date.now() });
	}

	async onClose() {
		// Nothing to clean up.
	}
}

class TapestryLoomPlugin implements PluginValue {
	constructor(view: EditorView) {
		// ...
	}

	update(update: ViewUpdate) {
		// ...
	}

	destroy() {
		// ...
	}
}

export const editorPlugin = ViewPlugin.fromClass(TapestryLoomPlugin);
