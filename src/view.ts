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
import { loadDocument, refreshDocument, WeaveDocument } from "document";

export const VIEW_COMMANDS: Array<Command> = [];

export const VIEW_TYPE = "tapestry-loom-view";

export class TapestryLoomView extends ItemView {
	plugin: TapestryLoom;
	listeners: EventRef[] = [];

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
		console.log(refreshDocument());
	}

	async update() {}

	async onOpen() {
		const container = this.containerEl.children[1];
		container.empty();
		container.createEl("h4", { text: "Title" });

		const { workspace } = this.app;

		this.listeners = [
			workspace.on("active-leaf-change", () => this.load()),
			workspace.on(
				"editor-change",
				debounce(() => this.update(), 180, true)
			),
		];
	}

	async onClose() {
		const { workspace } = this.app;
		this.listeners.forEach((listener) => workspace.offref(listener));
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
