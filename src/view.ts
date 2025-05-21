import {
	getFrontMatterInfo,
	debounce,
	parseYaml,
	Command,
	Editor,
	ItemView,
	EventRef,
	WorkspaceLeaf,
	setIcon,
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
import { ULID, ulid } from "ulid";
import cytoscape, { Core } from "cytoscape";
import dagre from "cytoscape-dagre";

// TODO: Use HoverPopover

// TODO: Eliminate (mis)use of Obsidian's internal CSS classes

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
		const editor = workspace.activeEditor?.editor;
		if (!editor) {
			return;
		}

		this.editor = editor;

		this.document = loadDocument(this.editor);
		this.renderDocument();
	}
	async update(editor: Editor) {
		this.editor = editor;

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

			console.log(container);

			//this.renderGraph(container);
			this.renderTree(container);

			console.log(this.document);
		} else {
			const container = this.contentEl;
			container.empty();
		}
	}
	private renderGraph(root: HTMLElement) {
		// TODO: Implement same functionality as renderTree()
		if (!this.document) {
			return;
		}

		const container = root.createEl("div", {
			cls: ["tapestry_graph"],
		});

		const elements: Array<cytoscape.ElementDefinition> = [];

		for (const node of this.document.getRootNodes()) {
			this.buildGraphNode(elements, node);
		}

		const cy = cytoscape({
			container: container,
			elements: elements,
			layout: { name: "dagre" },
		});
	}
	private buildGraphNode(
		elements: Array<cytoscape.ElementDefinition>,
		node: WeaveDocumentNode
	) {
		if (!this.document) {
			return;
		}

		const content = getNodeContent(node);
		const children = this.document.getNodeChildren(node);
		let modelLabel;
		if (node.model) {
			modelLabel = this.document.models.get(node.model);
		}

		elements.push({
			group: "nodes",
			data: {
				id: node.identifier,
			},
			grabbable: false,
		});
		if (node.parentNode) {
			elements.push({
				group: "edges",
				data: {
					source: node.parentNode,
					target: node.identifier,
				},
			});
		}

		for (const childNode of this.document.getNodeChildren(node)) {
			this.buildGraphNode(elements, childNode);
		}
	}
	private renderTree(root: HTMLElement) {
		if (!this.document) {
			return;
		}

		const list = root.createEl("div", {
			cls: ["tapestry_tree"],
		});

		for (const node of this.document.getRootNodes()) {
			this.renderNode(list, node);
		}
	}
	private renderNode(root: HTMLElement, node: WeaveDocumentNode) {
		if (!this.document) {
			return;
		}

		const content = getNodeContent(node);
		const children = this.document.getNodeChildren(node);
		let modelLabel;
		if (node.model) {
			modelLabel = this.document.models.get(node.model);
		}

		const item = root.createEl("div", {
			cls: ["tree-item"],
		});
		const labelContainer = item.createEl("div", {
			cls: ["tree-item-self", "is-clickable"],
			attr: { dragable: false },
		});
		if (this.document.currentNode == node.identifier) {
			labelContainer.style.backgroundColor =
				"var(--nav-item-background-selected)";
			labelContainer.style.color = "var(--nav-item-color-selected)";
		}
		const childrenContainer = item.createEl("div", {
			cls: ["tree-item-children"],
			attr: { dragable: false },
		});

		if (children.length > 0) {
			labelContainer.classList.add("mod-collapsible");
			const iconContainer = labelContainer.createEl("div", {
				text: getNodeContent(node),
				cls: ["tree-item-icon", "collapse-icon"],
			});
			iconContainer.innerHTML =
				'<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="svg-icon right-triangle"><path d="M3 8L12 17L21 8"></path></svg>';
			iconContainer.addEventListener("click", (event) => {
				event.stopPropagation();

				if (iconContainer.classList.contains("is-collapsed")) {
					item.classList.remove("is-collapsed");
					iconContainer.classList.remove("is-collapsed");
					childrenContainer.style.display = "inherit";
				} else {
					item.classList.add("is-collapsed");
					iconContainer.classList.add("is-collapsed");
					childrenContainer.style.display = "none";
				}
			});
		} else {
			labelContainer.style.marginLeft =
				"var(--nav-item-children-padding-start)";
			labelContainer.style.paddingLeft =
				"var(--nav-item-children-padding-start)";
		}

		const label = labelContainer.createEl("div", {
			cls: ["tree-item-inner"],
		});
		if (content.length > 0) {
			label.textContent = content;
		} else {
			label.innerHTML = "<em>Empty node</em>";
			label.style.color = "var(--text-faint)";
		}

		if (modelLabel) {
			label.title = modelLabel.label;
			if (modelLabel.color) {
				label.style.color = modelLabel.color;
			}
		}

		if (
			node.content.length == 1 &&
			typeof node.content == "object" &&
			Array.isArray(node.content)
		) {
			const probContainer = labelContainer.createEl("div", {
				cls: ["tree-item-flair-outer"],
			});
			probContainer.createEl("div", {
				text: (node.content[0][0] * 100).toPrecision(3) + "%",
				cls: ["tree-item-flair"],
			});
		}

		label.style.flexGrow = "1";
		labelContainer.addEventListener("click", () => {
			this.switchToNode(node.identifier);
		});

		const buttonContainer = labelContainer.createEl("div", {
			cls: ["tapestry_tree-buttons"],
		});

		if (
			node.parentNode &&
			this.document.isNodeMergeable(node.parentNode, node.identifier)
		) {
			const mergeButton = buttonContainer.createEl("div", {
				title: "Merge node with parent",
				cls: ["clickable-icon"],
			});
			setIcon(mergeButton, "merge");
			mergeButton.addEventListener("click", (event) => {
				event.stopPropagation();
				if (node.parentNode) {
					this.mergeNode(node.parentNode, node.identifier);
				}
			});
		}

		const generateButton = buttonContainer.createEl("div", {
			title: "Generate node",
			cls: ["clickable-icon"],
		});
		setIcon(generateButton, "bot-message-square"); // alternate generate icon: "bot"
		generateButton.addEventListener("click", (event) => {
			event.stopPropagation();
			throw new Error("unimplemented"); // TODO
		});

		const addButton = buttonContainer.createEl("div", {
			title: "Add node",
			cls: ["clickable-icon"],
		});
		setIcon(addButton, "message-square-plus");
		addButton.addEventListener("click", (event) => {
			event.stopPropagation();
			this.addNode(node.identifier);
		});

		const deleteButton = buttonContainer.createEl("div", {
			title: "Delete node",
			cls: ["clickable-icon"],
		});
		setIcon(deleteButton, "eraser");
		deleteButton.addEventListener("click", (event) => {
			event.stopPropagation();
			this.deleteNode(node.identifier);
		});

		for (const childNode of this.document.getNodeChildren(node)) {
			this.renderNode(childrenContainer, childNode);
		}
	}
	addNode(parentNode?: ULID) {
		if (!this.document || !this.editor) {
			return;
		}

		const identifier = ulid();
		this.document.addNode({
			identifier: identifier,
			content: "",
			parentNode: parentNode,
		});
		this.document.currentNode = identifier;
		overrideEditorContent(this.editor, this.document);
		this.renderDocument();
	}
	switchToNode(identifier: ULID) {
		if (!this.document || !this.editor) {
			return;
		}

		this.document.currentNode = identifier;
		overrideEditorContent(this.editor, this.document);
		this.renderDocument();
	}
	mergeNode(primaryIdentifier: ULID, secondaryIdentifier: ULID) {
		if (!this.document || !this.editor) {
			return;
		}

		this.document.mergeNode(primaryIdentifier, secondaryIdentifier);
		overrideEditorContent(this.editor, this.document);
		this.renderDocument();
	}
	deleteNode(identifier: ULID) {
		if (!this.document || !this.editor) {
			return;
		}

		this.document.removeNode(identifier);
		overrideEditorContent(this.editor, this.document);
		this.renderDocument();
	}
	async onOpen() {
		const container = this.containerEl.children[1];
		container.empty();

		const { workspace } = this.app;

		this.listeners = [
			workspace.on("active-leaf-change", () => this.load()),
			workspace.on(
				"editor-change",
				debounce((editor) => this.update(editor), 500, true) // TODO: Add setting for timeout
			),
			workspace.on("editor-drop", (_evt, editor) => {
				/*if (this.document) {
					saveDocument(editor, this.document);
				}*/

				this.document = undefined;
				this.editor = undefined;
				container.empty();
			}),
		];

		this.load();
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
