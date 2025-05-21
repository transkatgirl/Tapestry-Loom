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

// TODO: Eliminate (mis)use of Obsidian's internal CSS classes

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
	render(container: HTMLElement, _incremental?: boolean) {
		const document = this.plugin.document;
		container.empty();

		if (document) {
			console.log(container);

			this.renderTree(container);

			console.log(this.plugin.document);
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

		const activeNodes = document.getActiveNodes();

		if (
			document.currentNode &&
			activeNodes.length > 3 &&
			document.getNodeChildrenCount(document.currentNode) > 0
		) {
			this.renderNode(list, activeNodes.slice(-3)[0]);
		} else if (document.currentNode && activeNodes.length > 4) {
			this.renderNode(list, activeNodes.slice(-4)[0]);
		} else {
			for (const node of document.getRootNodes()) {
				this.renderNode(list, node);
			}
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
			label.innerHTML = "<em>No text</em>";
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
		this.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
	}
	switchToNode(identifier: ULID) {
		if (!this.plugin.document) {
			return;
		}

		this.plugin.document.currentNode = identifier;
		this.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
	}
	toggleBookmarkNode(identifier: ULID) {
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
	mergeNode(primaryIdentifier: ULID, secondaryIdentifier: ULID) {
		if (!this.plugin.document) {
			return;
		}

		this.plugin.document.mergeNode(primaryIdentifier, secondaryIdentifier);
		this.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
	}
	deleteNode(identifier: ULID) {
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

		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_LOAD_EVENT,
				() => {
					this.render(container, false);
				}
			)
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_UPDATE_EVENT,
				() => {
					this.render(container, true);
				}
			)
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_DROP_EVENT,
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
	iconContainer.style.color = "var(--nav-collapse-icon-color)";
	setIcon(iconContainer, "arrow-up");

	const label = labelContainer.createEl("div", {
		cls: ["tree-item-inner"],
	});

	label.innerHTML = "Show more";
	label.style.color = "var(--text-faint)";
}
