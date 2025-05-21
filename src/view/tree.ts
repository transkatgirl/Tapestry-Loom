import TapestryLoom, {
	DOCUMENT_DROP_EVENT,
	DOCUMENT_LOAD_EVENT,
	DOCUMENT_TRIGGER_UPDATE_EVENT,
	DOCUMENT_UPDATE_EVENT,
	SETTINGS_UPDATE_EVENT,
} from "main";
import { ItemView, WorkspaceLeaf, setIcon } from "obsidian";
import { getNodeContent, WeaveDocumentNode } from "document";
import { ULID, ulid } from "ulid";

// TODO: Use HoverPopover

export const TREE_VIEW_TYPE = "tapestry-loom-view";

export class TapestryLoomTreeView extends ItemView {
	plugin: TapestryLoom;
	constructor(leaf: WorkspaceLeaf, plugin: TapestryLoom) {
		super(leaf);
		this.plugin = plugin;
	}
	getViewType() {
		return TREE_VIEW_TYPE;
	}
	getDisplayText() {
		return "Tapestry Loom Tree";
	}
	getIcon(): string {
		return "list-tree";
	}
	private renderTree(container: HTMLElement, _incremental?: boolean) {
		container.empty();

		const document = this.plugin.document;
		if (!document) {
			renderMenuNotice(container, "No document found.");
			return;
		}

		const activeNodes = document.getActiveNodes();
		const rootNodes = document.getRootNodes();

		if (
			document.currentNode &&
			activeNodes.length > 3 &&
			document.getNodeChildrenCount(document.currentNode) > 0
		) {
			this.renderNode(container, activeNodes.slice(-3)[0]);
		} else if (document.currentNode && activeNodes.length > 4) {
			this.renderNode(container, activeNodes.slice(-4)[0]);
		} else if (rootNodes.length > 0) {
			for (const node of rootNodes) {
				this.renderNode(container, node);
			}
		} else {
			renderMenuNotice(container, "No nodes found.");
		}
	}
	private renderNode(root: HTMLElement, node: WeaveDocumentNode, depth = 0) {
		const document = this.plugin.document;
		if (!document) {
			return;
		}

		const content = getNodeContent(node);
		const children = document.getNodeChildren(node);
		let flair;
		if (
			node.content.length == 1 &&
			typeof node.content == "object" &&
			Array.isArray(node.content)
		) {
			flair = (node.content[0][0] * 100).toPrecision(3) + "%";
		}

		let modelLabel;
		if (node.model) {
			modelLabel = document.models.get(node.model);
		}

		const tree = renderTree(
			root,
			content,
			document.currentNode == node.identifier,
			children.length > 0,
			flair
		);
		if (modelLabel) {
			tree.label.title = modelLabel.label;
			if (modelLabel.color) {
				tree.label.style.color = modelLabel.color;
			}
		}

		tree.labelContainer.addEventListener("click", () => {
			this.switchToNode(node.identifier);
		});

		const buttons = renderNodeButtons(
			tree,
			typeof node.parentNode == "string" &&
				document.isNodeMergeable(node.parentNode, node.identifier),
			true,
			document.bookmarks.has(node.identifier)
		);

		if (buttons.mergeButton) {
			buttons.mergeButton.addEventListener("click", (event) => {
				event.stopPropagation();
				if (node.parentNode) {
					this.mergeNode(node.parentNode, node.identifier);
				}
			});
		}
		if (buttons.bookmarkToggleButton) {
			buttons.bookmarkToggleButton.addEventListener("click", (event) => {
				event.stopPropagation();
				this.toggleBookmarkNode(node.identifier);
			});
		}

		buttons.generateButton.addEventListener("click", (event) => {
			event.stopPropagation();
			throw new Error("unimplemented"); // TODO
		});
		buttons.addButton.addEventListener("click", (event) => {
			event.stopPropagation();
			this.addNode(node.identifier);
		});
		buttons.deleteButton.addEventListener("click", (event) => {
			event.stopPropagation();
			this.deleteNode(node.identifier);
		});

		if (children.length > 0 && depth > 6) {
			renderDepthNotice(tree);
			tree.childrenContainer.addEventListener("click", () => {
				this.switchToNode(node.identifier);
			});
		} else {
			for (const childNode of document.getNodeChildren(node)) {
				this.renderNode(tree.childrenContainer, childNode, depth + 1);
			}
		}
	}
	private renderModels(container: HTMLElement) {
		container.empty();
		renderMenuNotice(container, "Placeholder text.");
	}
	private addNode(parentNode?: ULID) {
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
		this.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
	}
	private switchToNode(identifier: ULID) {
		if (!this.plugin.document) {
			return;
		}

		this.plugin.document.currentNode = identifier;
		this.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
	}
	private toggleBookmarkNode(identifier: ULID) {
		if (!this.plugin.document) {
			return;
		}

		if (this.plugin.document.bookmarks.has(identifier)) {
			this.plugin.document.bookmarks.delete(identifier);
		} else {
			this.plugin.document.bookmarks.add(identifier);
		}

		this.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
	}
	private mergeNode(primaryIdentifier: ULID, secondaryIdentifier: ULID) {
		if (!this.plugin.document) {
			return;
		}

		this.plugin.document.mergeNode(primaryIdentifier, secondaryIdentifier);
		this.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
	}
	private deleteNode(identifier: ULID) {
		if (!this.plugin.document) {
			return;
		}

		this.plugin.document.removeNode(identifier);
		this.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
	}
	async onOpen() {
		const container = this.containerEl.children[1] as HTMLElement;
		container.empty();

		const { workspace } = this.app;

		const modelContainer = renderCollapsibleMenu(
			container,
			"Inference parameters"
		);
		const treeContainer = renderCollapsibleMenu(container, "Nearby nodes", [
			"tapestry_tree",
		]);

		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_LOAD_EVENT,
				() => {
					this.renderTree(treeContainer, false);
				}
			)
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_UPDATE_EVENT,
				() => {
					this.renderTree(treeContainer, true);
				}
			)
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_DROP_EVENT,
				() => {
					this.renderTree(treeContainer, false);
				}
			)
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				SETTINGS_UPDATE_EVENT,
				() => {
					this.renderModels(modelContainer);
				}
			)
		);

		this.renderModels(modelContainer);
		this.renderTree(treeContainer, false);
	}
	async onClose() {}
}

function renderCollapsibleMenu(
	root: HTMLElement,
	title: string,
	classes: Array<string> = []
) {
	const item = root.createEl("div", {
		cls: ["tree-item"],
	});
	const labelContainer = item.createEl("div", {
		cls: ["tree-item-self", "is-clickable"],
		attr: { dragable: false },
	});
	labelContainer.createEl("div", {
		text: title,
		cls: ["tree-item-inner", "tapestry_tree-heading"],
	});
	const container = root.createEl("div", {
		cls: ["tapestry_tree-heading-container"].concat(classes),
	});
	labelContainer.addEventListener("click", (event) => {
		event.stopPropagation();

		if (labelContainer.classList.contains("is-collapsed")) {
			item.classList.remove("is-collapsed");
			labelContainer.classList.remove("is-collapsed");
			container.style.display = "inherit";
		} else {
			item.classList.add("is-collapsed");
			labelContainer.classList.add("is-collapsed");
			container.style.display = "none";
		}
	});

	return container;
}

function renderMenuNotice(root: HTMLElement, text: string) {
	root.createEl("div", {
		text: text,
		cls: ["search-empty-state"],
	});
}

interface TreeElements {
	label: HTMLElement;
	labelContainer: HTMLElement;
	flairContainer: HTMLElement;
	childrenContainer: HTMLElement;
}

function renderTree(
	root: HTMLElement,
	text: string,
	selected: boolean,
	collapsible: boolean,
	flair?: string
): TreeElements {
	const item = root.createEl("div", {
		cls: ["tree-item"],
	});
	const labelContainer = item.createEl("div", {
		cls: ["tree-item-self", "is-clickable"],
		attr: { dragable: false },
	});
	if (selected) {
		labelContainer.classList.add("is-selected");
	}
	const childrenContainer = item.createEl("div", {
		cls: ["tree-item-children"],
		attr: { dragable: false },
	});

	if (collapsible) {
		labelContainer.classList.add("mod-collapsible");
		const iconContainer = labelContainer.createEl("div", {
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
	}

	const label = labelContainer.createEl("div", {
		cls: ["tree-item-inner"],
	});
	if (text.length > 0) {
		label.textContent = text;
	} else {
		label.innerHTML = "<em>No text</em>";
		label.classList.add("tapestry_tree-notice");
	}

	const flairContainer = labelContainer.createEl("div", {
		cls: ["tree-item-flair-outer"],
	});

	if (flair && flair.length > 0) {
		flairContainer.createEl("div", {
			text: flair,
			cls: ["tree-item-flair"],
		});
	}

	return {
		label: label,
		labelContainer: labelContainer,
		flairContainer: flairContainer,
		childrenContainer: childrenContainer,
	};
}

function renderDepthNotice(tree: TreeElements) {
	const root = tree.childrenContainer;

	const item = root.createEl("div", {
		cls: ["tree-item"],
	});
	const labelContainer = item.createEl("div", {
		cls: ["tree-item-self", "is-clickable"],
		attr: { dragable: false },
	});

	const iconContainer = labelContainer.createEl("div", {
		cls: ["tree-item-icon"],
	});
	setIcon(iconContainer, "arrow-up");

	const label = labelContainer.createEl("div", {
		cls: ["tree-item-inner", "tapestry_tree-notice"],
	});

	label.innerHTML = "Show more";
}

interface NodeButtonElements {
	mergeButton?: HTMLElement;
	generateButton: HTMLElement;
	addButton: HTMLElement;
	bookmarkToggleButton?: HTMLElement;
	deleteButton: HTMLElement;
}

function renderNodeButtons(
	tree: TreeElements,
	mergeable: boolean,
	bookmarkable: boolean,
	bookmarked?: boolean
): NodeButtonElements {
	const buttonContainer = tree.labelContainer.createEl("div", {
		cls: ["tapestry_tree-buttons"],
	});

	let mergeButton;

	if (mergeable) {
		mergeButton = buttonContainer.createEl("div", {
			title: "Merge node with parent",
			cls: ["clickable-icon"],
		});
		setIcon(mergeButton, "merge");
	}

	const generateButton = buttonContainer.createEl("div", {
		title: "Generate node",
		cls: ["clickable-icon"],
	});
	setIcon(generateButton, "bot-message-square"); // alternate generate icon: "bot"

	const addButton = buttonContainer.createEl("div", {
		title: "Add node",
		cls: ["clickable-icon"],
	});
	setIcon(addButton, "message-square-plus");

	let bookmarkButton;
	if (bookmarkable) {
		if (bookmarked) {
			bookmarkButton = buttonContainer.createEl("div", {
				title: "Remove bookmark",
				cls: ["clickable-icon"],
			});
			setIcon(bookmarkButton, "bookmark-minus");
		} else {
			bookmarkButton = buttonContainer.createEl("div", {
				title: "Bookmark node",
				cls: ["clickable-icon"],
			});
			setIcon(bookmarkButton, "bookmark-plus");
		}
	}

	const deleteButton = buttonContainer.createEl("div", {
		title: "Delete node",
		cls: ["clickable-icon"],
	});
	setIcon(deleteButton, "eraser");

	return {
		mergeButton: mergeButton,
		generateButton: generateButton,
		addButton: addButton,
		bookmarkToggleButton: bookmarkButton,
		deleteButton: deleteButton,
	};
}
