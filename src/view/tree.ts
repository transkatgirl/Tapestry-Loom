import TapestryLoom, {
	DOCUMENT_DROP_EVENT,
	DOCUMENT_LOAD_EVENT,
	DOCUMENT_TRIGGER_UPDATE_EVENT,
	DOCUMENT_UPDATE_EVENT,
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
			container.createEl("div", {
				text: "No document found.",
				cls: ["search-empty-state"],
			});
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
			container.createEl("div", {
				text: "No nodes found.",
				cls: ["search-empty-state"],
			});
		}
	}
	private renderNode(root: HTMLElement, node: WeaveDocumentNode, depth = 0) {
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
			labelContainer.classList.add("is-selected");
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
		}

		const label = labelContainer.createEl("div", {
			cls: ["tree-item-inner"],
		});
		if (content.length > 0) {
			label.textContent = content;
		} else {
			label.innerHTML = "<em>No text</em>";
			label.classList.add("tapestry_tree-notice");
		}

		if (modelLabel) {
			label.title = modelLabel.label;
			if (modelLabel.color) {
				label.style.color = modelLabel.color;
			}
		}

		const probContainer = labelContainer.createEl("div", {
			cls: ["tree-item-flair-outer"],
		});

		if (
			node.content.length == 1 &&
			typeof node.content == "object" &&
			Array.isArray(node.content)
		) {
			probContainer.createEl("div", {
				text: (node.content[0][0] * 100).toPrecision(3) + "%",
				cls: ["tree-item-flair"],
			});
		}

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

		/*if (document.bookmarks.has(node.identifier)) {
			const bookmarkButton = buttonContainer.createEl("div", {
				title: "Remove bookmark",
				cls: ["clickable-icon"],
			});
			setIcon(bookmarkButton, "bookmark-minus");
			bookmarkButton.addEventListener("click", (event) => {
				event.stopPropagation();
				this.toggleBookmarkNode(node.identifier);
			});
		} else {
			const bookmarkButton = buttonContainer.createEl("div", {
				title: "Bookmark node",
				cls: ["clickable-icon"],
			});
			setIcon(bookmarkButton, "bookmark-plus");
			bookmarkButton.addEventListener("click", (event) => {
				event.stopPropagation();
				this.toggleBookmarkNode(node.identifier);
			});
		}*/

		const deleteButton = buttonContainer.createEl("div", {
			title: "Delete node",
			cls: ["clickable-icon"],
		});
		setIcon(deleteButton, "eraser");
		deleteButton.addEventListener("click", (event) => {
			event.stopPropagation();
			this.deleteNode(node.identifier);
		});

		if (children.length > 0 && depth > 6) {
			renderDepthNotice(childrenContainer);
			childrenContainer.addEventListener("click", () => {
				this.switchToNode(node.identifier);
			});
		} else {
			for (const childNode of document.getNodeChildren(node)) {
				this.renderNode(childrenContainer, childNode, depth + 1);
			}
		}
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

		const treeItem = container.createEl("div", {
			cls: ["tree-item"],
		});
		const labelContainer = treeItem.createEl("div", {
			cls: ["tree-item-self", "is-clickable"],
			attr: { dragable: false },
		});
		labelContainer.createEl("div", {
			text: "Nearby nodes",
			cls: ["tree-item-inner", "tapestry_tree-heading"],
		});
		const treeContainer = container.createEl("div", {
			cls: ["tapestry_tree-heading-container", "tapestry_tree"],
		});

		labelContainer.addEventListener("click", (event) => {
			event.stopPropagation();

			if (labelContainer.classList.contains("is-collapsed")) {
				treeItem.classList.remove("is-collapsed");
				labelContainer.classList.remove("is-collapsed");
				treeContainer.style.display = "inherit";
			} else {
				treeItem.classList.add("is-collapsed");
				labelContainer.classList.add("is-collapsed");
				treeContainer.style.display = "none";
			}
		});

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

		this.renderTree(treeContainer, false);
	}
	async onClose() {}
}

function renderDepthNotice(root: HTMLElement) {
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
