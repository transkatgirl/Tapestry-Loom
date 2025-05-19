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
	getNodeContent,
	loadDocument,
	overrideEditorContent,
	saveDocument,
	updateDocument,
	WeaveDocument,
	WeaveDocumentNode,
} from "document";
import { ulid } from "ulid";

export const VIEW_COMMANDS: Array<Command> = [];

export const VIEW_TYPE = "tapestry-loom-view";

export class TapestryLoomView extends ItemView {
	plugin: TapestryLoom;
	listeners: EventRef[] = [];
	editor?: Editor;
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
		if (!workspace.activeEditor?.editor) {
			return;
		}

		this.editor = workspace.activeEditor?.editor;

		this.document = loadDocument(this.editor);
		this.renderDocument();
	}
	async update() {
		const { workspace } = this.app;
		this.editor = workspace.activeEditor?.editor;

		if (!this.editor) {
			this.document = undefined;
			return;
		}

		if (this.document) {
			const updated = updateDocument(this.editor, this.document);
			if (updated) {
				this.renderDocument(true);
			}
		} else {
			this.document = loadDocument(this.editor);
			this.renderDocument();
		}
	}
	renderDocument(incremental?: boolean) {
		if (this.document) {
			const container = this.contentEl;
			container.empty();

			const list = container.createEl("ul");

			for (const node of this.document.getRootNodes()) {
				this.renderNode(list, node);
			}

			console.log(this.document);
		} else {
			const container = this.contentEl;
			container.empty();
		}
	}
	private renderNode(root: HTMLElement, node: WeaveDocumentNode) {
		if (!this.document) {
			return;
		}

		const item = root.createEl("li", {
			text: getNodeContent(node),
			attr: { id: node.identifier },
		});
		const addButton = item.createEl("button", {
			text: "Add node",
			type: "button",
		});
		addButton.addEventListener("click", () => {
			console.log(this.document);

			if (!this.document || !this.editor) {
				return;
			}

			const identifier = ulid();
			this.document.addNode({
				identifier: identifier,
				content: "",
				parentNode: node.identifier,
			});
			this.document.currentNode = identifier;
			saveDocument(this.editor, this.document);
		});
		const deleteButton = item.createEl("button", {
			text: "Delete node",
			type: "button",
		});
		deleteButton.addEventListener("click", () => {
			console.log(this.document);

			if (!this.document || !this.editor) {
				return;
			}

			this.document.removeNode(node.identifier);
			overrideEditorContent(this.editor, this.document);
		});

		for (const childNode of this.document.getNodeChildren(node)) {
			const list = item.createEl("ul");

			this.renderNode(list, childNode);
			item.appendChild(list);
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
			/*workspace.on("editor-drop", (_evt, editor) => {
				if (this.document) {
					saveDocument(editor, this.document);
				}

				this.editor = undefined;
			}),*/
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
