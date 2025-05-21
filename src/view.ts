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
import { getNodeContent, WeaveDocumentNode } from "document";
import { ULID, ulid } from "ulid";
import cytoscape from "cytoscape";

// TODO: Use HoverPopover

// TODO: Eliminate (mis)use of Obsidian's internal CSS classes

export const VIEW_COMMANDS: Array<Command> = [];

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
	render(container: HTMLElement, incremental?: boolean) {
		const document = this.plugin.document;
		container.empty();

		if (document) {
			console.log(container);

			//this.renderGraph(container);
			this.renderTree(container);

			console.log(this.plugin.document);
		}
	}
	private renderGraph(root: HTMLElement) {
		// TODO: Implement same functionality as renderTree()
		const document = this.plugin.document;
		if (!document) {
			return;
		}

		const container = root.createEl("div", {
			cls: ["tapestry_graph"],
		});

		const elements: Array<cytoscape.ElementDefinition> = [];

		for (const node of document.getRootNodes()) {
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
		const document = this.plugin.document;
		if (!document) {
			return;
		}

		const content = getNodeContent(node);
		const children = document.getNodeChildren(node);
		let modelLabel;
		if (node.model) {
			modelLabel = document.models.get(node.model);
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

		for (const childNode of document.getNodeChildren(node)) {
			this.buildGraphNode(elements, childNode);
		}
	}
	private renderTree(root: HTMLElement) {
		const document = this.plugin.document;
		if (!document) {
			return;
		}

		const list = root.createEl("div", {
			cls: ["tapestry_tree"],
		});

		for (const node of document.getRootNodes()) {
			this.renderNode(list, node);
		}
	}
	private renderNode(root: HTMLElement, node: WeaveDocumentNode) {
		const document = this.plugin.document;
		if (!document) {
			return;
		}

		const content = getNodeContent(node);
		const children = document.getNodeChildren(node);
		let modelLabel;
		if (node.model) {
			modelLabel = document.models.get(node.model);
		}

		const item = root.createEl("div", {
			cls: ["tree-item"],
		});
		const labelContainer = item.createEl("div", {
			cls: ["tree-item-self", "is-clickable"],
			attr: { dragable: false },
		});
		if (document.currentNode == node.identifier) {
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
			document.isNodeMergeable(node.parentNode, node.identifier)
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

		for (const childNode of document.getNodeChildren(node)) {
			this.renderNode(childrenContainer, childNode);
		}
	}
	addNode(parentNode?: ULID) {
		if (!this.plugin.document) {
			return;
		}

		const identifier = ulid();
		this.plugin.document.addNode({
			identifier: identifier,
			content: "",
			parentNode: parentNode,
		});
		this.plugin.document.currentNode = identifier;
		this.app.workspace.trigger("tapestry-document:override");
	}
	switchToNode(identifier: ULID) {
		if (!this.plugin.document) {
			return;
		}

		this.plugin.document.currentNode = identifier;
		this.app.workspace.trigger("tapestry-document:override");
	}
	mergeNode(primaryIdentifier: ULID, secondaryIdentifier: ULID) {
		if (!this.plugin.document) {
			return;
		}

		this.plugin.document.mergeNode(primaryIdentifier, secondaryIdentifier);
		this.app.workspace.trigger("tapestry-document:override");
	}
	deleteNode(identifier: ULID) {
		if (!this.plugin.document) {
			return;
		}

		this.plugin.document.removeNode(identifier);
		this.app.workspace.trigger("tapestry-document:override");
	}
	async onOpen() {
		const container = this.containerEl.children[1] as HTMLElement;
		container.empty();

		const { workspace } = this.app;

		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				"tapestry-document:load",
				() => {
					this.render(container, false);
				}
			)
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				"tapestry-document:update",
				() => {
					this.render(container, true);
				}
			)
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				"tapestry-document:drop",
				() => {
					this.render(container, false);
				}
			)
		);

		if (this.plugin.document) {
			this.render(container, false);
		}
	}
	async onClose() {}
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
