import TapestryLoom, {
	DOCUMENT_DROP_EVENT,
	DOCUMENT_LOAD_EVENT,
	DOCUMENT_UPDATE_EVENT,
	SETTINGS_UPDATE_EVENT,
} from "main";
import { ItemView, WorkspaceLeaf } from "obsidian";
import { getNodeContent, WeaveDocumentNode } from "document";
import { ULID } from "ulid";
import { switchToNode } from "./common";
import { UNKNOWN_MODEL_LABEL } from "client";

export const LIST_VIEW_TYPE = "tapestry-loom-sibling-list-view";

export interface SessionSettings {
	requests: number;
	models: Array<ULID>;
	parameters: Record<string, string>;
}

export class TapestryLoomListView extends ItemView {
	plugin: TapestryLoom;
	constructor(leaf: WorkspaceLeaf, plugin: TapestryLoom) {
		super(leaf);
		this.plugin = plugin;
	}
	getViewType() {
		return LIST_VIEW_TYPE;
	}
	getDisplayText() {
		return "Tapestry Loom Node List";
	}
	getIcon(): string {
		return "layout-list";
	}
	private renderNoteLists(
		siblingContainer: HTMLElement,
		childContainer: HTMLElement
	) {
		const document = this.plugin.document;
		if (!document || !document.currentNode) {
			this.renderNodeList(siblingContainer, []);
			this.renderNodeList(childContainer, []);
			return;
		}

		const currentNode = document.getNode(document.currentNode);
		if (!currentNode) {
			this.renderNodeList(siblingContainer, []);
			this.renderNodeList(childContainer, []);
			return;
		}

		if (currentNode.parentNode) {
			this.renderNodeList(
				siblingContainer,
				document.getNodeChildren(currentNode.parentNode)
			);
		} else {
			this.renderNodeList(siblingContainer, document.getRootNodes());
		}

		this.renderNodeList(
			childContainer,
			document.getNodeChildren(currentNode.identifier)
		);
	}
	private renderNodeList(
		container: HTMLElement,
		nodes: Array<WeaveDocumentNode>
	) {
		container.empty();

		const document = this.plugin.document;
		if (!document) {
			renderMenuNotice(container, "No document found.");
			return;
		}

		if (nodes.length > 0) {
			const wrapper = container.createEl("div", {
				cls: ["search-result-file-matches"],
			});
			for (const node of nodes) {
				this.renderNode(wrapper, node);
			}
		} else {
			renderMenuNotice(container, "No nodes found.");
			return;
		}
	}
	private renderNode(container: HTMLElement, node: WeaveDocumentNode) {
		const document = this.plugin.document;
		if (!document) {
			return;
		}

		const content = getNodeContent(node);
		let flair;
		if (
			node.content.length == 1 &&
			typeof node.content == "object" &&
			Array.isArray(node.content)
		) {
			flair = (node.content[0][0] * 100).toPrecision(3) + "%";
		}

		const item = container.createEl("div", {
			cls: ["search-result-file-match tappable"],
		});
		item.createEl("span", {
			text: content.trim(),
		});
		if (document.currentNode == node.identifier) {
			item.classList.add("tapestry_list-selected");
		}
		if (flair) {
			item.createEl("div", {
				text: flair,
				cls: ["tree-item-flair"],
			});
		}

		if (node.model) {
			const modelLabel =
				document.models.get(node.model) || UNKNOWN_MODEL_LABEL;

			item.title = modelLabel.label;
			if (modelLabel.color) {
				item.style.color = modelLabel.color;
			}
			if (node.parameters) {
				for (const [key, value] of Object.entries(node.parameters)) {
					item.title = item.title + "\n" + key + ": " + value;
				}
			}
		}

		item.addEventListener("click", () => {
			switchToNode(this.plugin, node.identifier);
		});
	}
	async onOpen() {
		const container = this.containerEl.children[1] as HTMLElement;
		container.empty();

		const { workspace } = this.app;

		const wrapper = container.createEl("div", {
			cls: ["search-results-children"],
		});

		const siblingDropdown = buildDropdown(wrapper, "Sibling nodes", false);
		const childDropdown = buildDropdown(wrapper, "Child nodes", true);

		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_LOAD_EVENT,
				() => {
					this.renderNoteLists(siblingDropdown, childDropdown);
				}
			)
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_UPDATE_EVENT,
				() => {
					this.renderNoteLists(siblingDropdown, childDropdown);
				}
			)
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_DROP_EVENT,
				() => {
					this.renderNoteLists(siblingDropdown, childDropdown);
				}
			)
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				SETTINGS_UPDATE_EVENT,
				() => {
					this.renderNoteLists(siblingDropdown, childDropdown);
				}
			)
		);

		this.renderNoteLists(siblingDropdown, childDropdown);
	}
	async onClose() {}
}

function buildDropdown(root: HTMLElement, label: string, collapsed: boolean) {
	const tree = root.createEl("div", { cls: ["tree-item", "search-result"] });
	const labelContainer = tree.createEl("div", {
		cls: [
			"tree-item-self",
			"mod-collapsible",
			"search-result-file-title",
			"is-clickable",
		],
		attr: { dragable: false },
	});
	const iconContainer = labelContainer.createEl("div", {
		cls: ["tree-item-icon", "collapse-icon"],
	});
	iconContainer.innerHTML =
		'<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="svg-icon right-triangle"><path d="M3 8L12 17L21 8"></path></svg>';
	labelContainer.createEl("div", {
		text: label,
		cls: ["tree-item-inner"],
	});
	const container = root.createEl("div", {
		cls: ["tapestry_list"],
	});
	labelContainer.addEventListener("click", (event) => {
		event.stopPropagation();

		if (iconContainer.classList.contains("is-collapsed")) {
			tree.classList.remove("is-collapsed");
			iconContainer.classList.remove("is-collapsed");
			container.style.display = "inherit";
		} else {
			tree.classList.add("is-collapsed");
			iconContainer.classList.add("is-collapsed");
			container.style.display = "none";
		}
	});
	if (collapsed) {
		tree.classList.add("is-collapsed");
		iconContainer.classList.add("is-collapsed");
		container.style.display = "none";
	} else {
		tree.classList.remove("is-collapsed");
		iconContainer.classList.remove("is-collapsed");
		container.style.display = "inherit";
	}

	return container;
}

function renderMenuNotice(root: HTMLElement, text: string) {
	root.createEl("div", {
		text: text,
		cls: ["search-empty-state"],
	});
}
