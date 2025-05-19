import {
	getFrontMatterInfo,
	debounce,
	parseYaml,
	Command,
	Editor,
	ItemView,
	EventRef,
	WorkspaceLeaf,
} from "obsidian";
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
import {
	loadDocument,
	saveDocument,
	updateDocument,
	WeaveDocument,
	WeaveDocumentNode,
} from "document";

export const VIEW_COMMANDS: Array<Command> = [];

export const VIEW_TYPE = "tapestry-loom-view";

export class TapestryLoomView extends ItemView {
	plugin: TapestryLoom;
	listeners: EventRef[] = [];
	document?: WeaveDocument;

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
	async load() {
		const { workspace } = this.app;
		const editor = workspace.activeEditor?.editor;

		if (!editor) {
			this.document = undefined;
			return;
		}

		this.document = loadDocument(editor);
		this.renderDocument();
	}
	async update() {
		const { workspace } = this.app;
		const editor = workspace.activeEditor?.editor;

		if (!editor) {
			this.document = undefined;
			return;
		}

		if (this.document) {
			const updated = updateDocument(editor, this.document);
			if (updated) {
				this.renderDocument(true);
			}
		} else {
			this.document = loadDocument(editor);
			this.renderDocument();
		}
	}
	async renderDocument(incremental?: boolean) {
		if (this.document) {
			const container = this.contentEl;
			container.empty();
			container.createEl("p", { text: this.document.getActiveContent() });

			console.log(this.document);
		} else {
			const container = this.contentEl;
			container.empty();
		}
	}
	async onOpen() {
		const container = this.containerEl.children[1];
		container.empty();
		//container.createEl("h4", { text: "Title" });

		const { workspace } = this.app;

		this.listeners = [
			workspace.on("active-leaf-change", () => this.load()),
			workspace.on(
				"editor-change",
				debounce(() => this.update(), 1500, true) // TODO: Add setting for timeout
			),
		];
	}
	async onClose() {
		const { workspace } = this.app;
		this.listeners.forEach((listener) => workspace.offref(listener));

		const editor = workspace.activeEditor?.editor;
		if (editor && this.document) {
			saveDocument(editor, this.document);
		}
		this.document = undefined;
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

export const EDITOR_PLUGIN = ViewPlugin.fromClass(TapestryLoomPlugin);
