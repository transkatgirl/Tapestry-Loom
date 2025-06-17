import TapestryLoom, {
	DOCUMENT_DROP_EVENT,
	DOCUMENT_LOAD_EVENT,
	DOCUMENT_TRIGGER_UPDATE_EVENT,
	DOCUMENT_UPDATE_EVENT,
	SETTINGS_UPDATE_EVENT,
} from "main";
import {
	ItemView,
	Menu,
	Setting,
	WorkspaceLeaf,
	debounce,
	setIcon,
} from "obsidian";
import { getNodeContent, WeaveDocumentNode } from "weave/format-v0";
import { ULID } from "ulid";
import { DEFAULT_DOCUMENT_SETTINGS, DEFAULT_SESSION_SETTINGS } from "settings";
import {
	addNode,
	addNodeSibling,
	deleteNode,
	deleteNodeChildren,
	deleteNodeSiblings,
	generateNodeChildren,
	mergeNode,
	switchToNode,
} from "./common";
import { ModelConfiguration, UNKNOWN_MODEL_LABEL } from "client";

export const TREE_LIST_VIEW_TYPE = "tapestry-loom-main-tree-view";

export interface SessionSettings {
	requests: number;
	depth: number;
	models: Array<ULID>;
	parameters: Record<string, string>;
}

export class TapestryLoomMainTreeView extends ItemView {
	plugin: TapestryLoom;
	private collapsedNodes: Set<ULID> = new Set();
	private modelMenu?: CollapsibleMenuElement;
	private bookmarksMenu?: CollapsibleMenuElement;
	private treeMenu?: CollapsibleMenuElement;
	constructor(leaf: WorkspaceLeaf, plugin: TapestryLoom) {
		super(leaf);
		this.plugin = plugin;
	}
	getViewType() {
		return TREE_LIST_VIEW_TYPE;
	}
	getDisplayText() {
		return "Tapestry Loom Tree";
	}
	getIcon(): string {
		return "list-tree";
	}
	private renderTree(container: HTMLElement, incremental?: boolean) {
		container.empty();

		const document = this.plugin.document;
		if (!document) {
			this.collapsedNodes = new Set();
			renderMenuNotice(container, "No document found.");
			return;
		}

		if (!incremental) {
			this.collapsedNodes = new Set();
		}

		const activeNodes = document.getActiveNodes();
		for (const node of activeNodes) {
			if (node.parentNode) {
				this.collapsedNodes.delete(node.parentNode);
			}
		}
		const rootNodes = document.getRootNodes();
		const renderDepth =
			this.plugin.settings.document?.treeDepth ||
			DEFAULT_DOCUMENT_SETTINGS.treeDepth;

		if (
			document.currentNode &&
			activeNodes.length > renderDepth - 1 &&
			document.getNodeChildrenCount(document.currentNode) > 0
		) {
			this.renderNode(
				container,
				activeNodes.slice(-1 * (renderDepth - 1))[0]
			);
		} else if (document.currentNode && activeNodes.length > renderDepth) {
			this.renderNode(container, activeNodes.slice(-1 * renderDepth)[0]);
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
		const children = document.getNodeChildren(node.identifier);
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
			modelLabel = document.models.get(node.model) || UNKNOWN_MODEL_LABEL;
		}

		const tree = renderTree(
			root,
			content,
			document.currentNode == node.identifier,
			children.length > 0,
			this.collapsedNodes.has(node.identifier),
			document.bookmarks.has(node.identifier),
			flair,
			(collapsed) => {
				if (collapsed) {
					this.collapsedNodes.add(node.identifier);
				} else {
					this.collapsedNodes.delete(node.identifier);
				}
			}
		);
		if (modelLabel) {
			tree.label.title = modelLabel.label;
			if (modelLabel.color) {
				tree.label.style.color = modelLabel.color;
			}
			if (node.parameters) {
				for (const [key, value] of Object.entries(node.parameters)) {
					tree.label.title =
						tree.label.title + "\n" + key + ": " + value;
				}
			}
		}

		tree.labelContainer.addEventListener("click", () => {
			switchToNode(this.plugin, node.identifier);
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
				mergeNode(this.plugin, node.identifier);
			});
		}
		if (buttons.bookmarkToggleButton) {
			buttons.bookmarkToggleButton.addEventListener("click", (event) => {
				event.stopPropagation();
				this.toggleBookmarkNode(node.identifier);
			});
		}

		buttons.generateButton.addEventListener("click", async (event) => {
			event.stopPropagation();
			await generateNodeChildren(this.plugin, node.identifier);
		});
		buttons.addButton.addEventListener("click", (event) => {
			event.stopPropagation();
			addNode(this.plugin, node.identifier);
		});
		buttons.deleteButton.addEventListener("click", (event) => {
			event.stopPropagation();
			deleteNode(this.plugin, node.identifier);
		});
		tree.labelContainer.addEventListener("contextmenu", (event) => {
			event.preventDefault();
			this.renderMenu(node, event);
		});

		const renderDepth =
			this.plugin.settings.document?.treeDepth ||
			DEFAULT_DOCUMENT_SETTINGS.treeDepth;

		if (children.length > 0 && depth > renderDepth) {
			renderDepthNotice(tree);
			tree.childrenContainer.addEventListener("click", () => {
				switchToNode(this.plugin, node.identifier);
			});
		} else {
			for (const childNode of document.getNodeChildren(node.identifier)) {
				this.renderNode(tree.childrenContainer, childNode, depth + 1);
			}
		}
	}
	private renderMenu(node: WeaveDocumentNode, event: MouseEvent) {
		const identifier = node.identifier;
		const hasChildren =
			(this.plugin.document?.getNodeChildrenCount(identifier) || 0) > 0;

		const menu = new Menu();

		menu.addItem((item) => {
			item.setTitle("Generate");
			item.onClick(() => {
				generateNodeChildren(this.plugin, identifier);
			});
		});

		menu.addItem((item) => {
			if (
				this.plugin.document &&
				this.plugin.document.bookmarks.has(node.identifier)
			) {
				item.setTitle("Remove bookmark");
			} else {
				item.setTitle("Bookmark");
			}
			item.onClick(() => {
				this.toggleBookmarkNode(identifier);
			});
		});

		menu.addSeparator();

		menu.addItem((item) => {
			item.setTitle("Create child");
			item.onClick(() => {
				addNode(this.plugin, identifier);
			});
		});
		menu.addItem((item) => {
			item.setTitle("Create sibling");
			item.onClick(() => {
				addNodeSibling(this.plugin, identifier);
			});
		});

		if (hasChildren || node.parentNode) {
			menu.addSeparator();
		}

		if (hasChildren) {
			menu.addItem((item) => {
				item.setTitle("Collapse all children");
				item.onClick(() => {
					this.collapseNodeChildren(identifier);
				});
			});
			menu.addItem((item) => {
				item.setTitle("Expand all children");
				item.onClick(() => {
					this.expandNodeChildren(identifier);
				});
			});

			menu.addSeparator();

			menu.addItem((item) => {
				item.setTitle("Delete all children");
				item.onClick(() => {
					deleteNodeChildren(this.plugin, identifier);
				});
			});
		}

		if (node.parentNode) {
			menu.addItem((item) => {
				item.setTitle("Delete all siblings");
				item.onClick(() => {
					deleteNodeSiblings(this.plugin, identifier, true);
				});
			});

			menu.addSeparator();

			menu.addItem((item) => {
				item.setTitle("Merge with parent");
				item.onClick(() => {
					mergeNode(this.plugin, identifier);
				});
			});
		}

		menu.addSeparator();

		menu.addItem((item) => {
			item.setTitle("Delete");
			item.onClick(() => {
				deleteNode(this.plugin, identifier);
			});
		});

		menu.showAtMouseEvent(event);
	}
	private toggleNodeExpansion(identifier: ULID) {
		if (!this.treeMenu) {
			return;
		}

		if (this.collapsedNodes.has(identifier)) {
			this.collapsedNodes.delete(identifier);
		} else {
			this.collapsedNodes.add(identifier);
		}
		this.renderTree(this.treeMenu.childrenContainer, true);
	}
	private collapseOtherNodes() {
		if (!this.treeMenu || !this.plugin.document) {
			return;
		}

		const activeNodes = this.plugin.document.getActiveNodes();

		for (let i = 0; i < activeNodes.length; i++) {
			const activeNode = activeNodes[i];
			let nextActiveNode;
			if (i + 1 < activeNodes.length) {
				nextActiveNode = activeNodes[i + 1];
			}
			const nodes = this.plugin.document.getNodeChildren(
				activeNode.identifier
			);

			for (const node of nodes) {
				if (nextActiveNode?.identifier != node.identifier) {
					this.collapsedNodes.add(node.identifier);
				}
			}
		}

		this.renderTree(this.treeMenu.childrenContainer, true);
	}
	private expandNodes() {
		if (!this.treeMenu || !this.plugin.document) {
			return;
		}

		const activeNodes = this.plugin.document.getActiveNodes();

		for (const activeNode of activeNodes) {
			const nodes = this.plugin.document.getNodeChildren(
				activeNode.identifier
			);
			for (const node of nodes) {
				this.collapsedNodes.delete(node.identifier);
			}
		}

		this.renderTree(this.treeMenu.childrenContainer, true);
	}
	private collapseNodeChildren(identifier: ULID) {
		if (!this.treeMenu || !this.plugin.document) {
			return;
		}

		const nodes = this.plugin.document.getNodeChildren(identifier);
		for (const node of nodes) {
			this.collapsedNodes.add(node.identifier);
		}
		this.renderTree(this.treeMenu.childrenContainer, true);
	}
	private expandNodeChildren(identifier: ULID) {
		if (!this.treeMenu || !this.plugin.document) {
			return;
		}

		const nodes = this.plugin.document.getNodeChildren(identifier);
		for (const node of nodes) {
			this.collapsedNodes.delete(node.identifier);
		}
		this.renderTree(this.treeMenu.childrenContainer, true);
	}
	private renderBookmarks(container: HTMLElement) {
		container.empty();

		const document = this.plugin.document;
		if (!document) {
			renderMenuNotice(container, "No document found.");
			return;
		}

		if (document.bookmarks.size > 0) {
			for (const identifier of document.bookmarks) {
				const node = document.getNode(identifier);
				if (node) {
					this.renderBookmarkedNode(container, node);
				}
			}
		} else {
			renderMenuNotice(container, "No bookmarks found.");
		}
	}
	private renderBookmarkedNode(root: HTMLElement, node: WeaveDocumentNode) {
		const document = this.plugin.document;
		if (!document) {
			return;
		}

		const content = getNodeContent(node);

		let modelLabel;
		if (node.model) {
			modelLabel = document.models.get(node.model) || UNKNOWN_MODEL_LABEL;
		}

		const tree = renderBookmarkNode(
			root,
			content,
			document.currentNode == node.identifier
		);
		if (modelLabel) {
			tree.label.title = modelLabel.label;
			if (modelLabel.color) {
				tree.label.style.color = modelLabel.color;
			}
			if (node.parameters) {
				for (const [key, value] of Object.entries(node.parameters)) {
					tree.label.title =
						tree.label.title + "\n" + key + ": " + value;
				}
			}
		}

		tree.labelContainer.addEventListener("click", () => {
			switchToNode(this.plugin, node.identifier);
		});

		const bookmarkToggleButton = renderBookmarkNodeButton(tree);

		bookmarkToggleButton.addEventListener("click", (event) => {
			event.stopPropagation();
			this.toggleBookmarkNode(node.identifier);
		});

		tree.labelContainer.addEventListener("contextmenu", (event) => {
			event.preventDefault();
			this.renderMenu(node, event);
		});
	}
	private renderModels(container: HTMLElement) {
		container.empty();

		if (
			!this.plugin.settings.client ||
			this.plugin.settings.client.models.length == 0
		) {
			renderMenuNotice(
				container,
				"No models found. Models can be added in the plugin's settings menu."
			);
			return;
		}

		const debounceTime =
			this.plugin.settings.document?.debounce ||
			DEFAULT_DOCUMENT_SETTINGS.debounce;

		const models = this.plugin.settings.client.models;

		const modelMap: Map<ULID, ModelConfiguration> = new Map();
		for (const model of models) {
			modelMap.set(model.ulid, model);
		}

		new Setting(container)
			.setName("Requests per iteration")
			.addText((text) => {
				text.setPlaceholder((1).toString())
					.setValue(this.plugin.sessionSettings.requests.toString())
					.onChange(async (value) => {
						this.plugin.sessionSettings.requests =
							parseInt(value) || 1;
						if (this.plugin.sessionSettings.requests < 1) {
							this.plugin.sessionSettings.requests = 1;
						}
					});
			});

		new Setting(container).setName("Recursion depth").addText((text) => {
			text.setPlaceholder((1).toString())
				.setValue(this.plugin.sessionSettings.depth.toString())
				.onChange(async (value) => {
					this.plugin.sessionSettings.depth = parseInt(value) || 1;
					if (this.plugin.sessionSettings.depth < 1) {
						this.plugin.sessionSettings.depth = 1;
					}
				});
		});

		new Setting(container).setHeading().setName("Models");
		for (let i = 0; i < this.plugin.sessionSettings.models.length; i++) {
			new Setting(container)
				.addDropdown((dropdown) => {
					for (const modelOption of models) {
						dropdown.addOption(
							modelOption.ulid,
							modelOption.label.label
						);
					}
					dropdown
						.setValue(this.plugin.sessionSettings.models[i])
						.onChange((value) => {
							this.plugin.sessionSettings.models[i] = value;
							const model = modelMap.get(value);
							if (model?.label.color) {
								dropdown.selectEl.style.color =
									model?.label.color;
							} else {
								dropdown.selectEl.style.color = "inherit";
							}
							if (model?.url) {
								dropdown.selectEl.title = model.url;
							} else {
								dropdown.selectEl.title = "";
							}
						});
					const model = modelMap.get(
						this.plugin.sessionSettings.models[i]
					);
					if (model?.label.color) {
						dropdown.selectEl.style.color = model?.label.color;
					}
					if (model?.url) {
						dropdown.selectEl.title = model.url;
					}
				})
				.addExtraButton((button) => {
					button.setIcon("x").onClick(() => {
						this.plugin.sessionSettings.models.splice(i, 1);
						this.renderModels(container);
					});
				});
		}

		new Setting(container).addButton((button) => {
			button.setButtonText("Add model").onClick((_event) => {
				this.plugin.sessionSettings.models.push(models[0].ulid);
				this.renderModels(container);
			});
			button.buttonEl.style.width = "100%";
		});

		new Setting(container).setHeading().setName("Request parameters");

		for (let [parameterKey, parameterValue] of Object.entries(
			this.plugin.sessionSettings.parameters
		)) {
			new Setting(container)
				.addText((text) => {
					text.setPlaceholder("key")
						.setValue(parameterKey)
						.onChange(
							debounce(
								async (value) => {
									if (value.length > 0) {
										delete this.plugin.sessionSettings
											.parameters[parameterKey];
										this.plugin.sessionSettings.parameters[
											value
										] = parameterValue;
										parameterKey = value;
									} else {
										delete this.plugin.sessionSettings
											.parameters[parameterKey];
										this.renderModels(container);
									}
								},
								debounceTime,
								true
							)
						);
				})
				.addText((text) => {
					text.setPlaceholder("value")
						.setValue(parameterValue)
						.onChange(async (value) => {
							this.plugin.sessionSettings.parameters[
								parameterKey
							] = value;
							parameterValue = value;
						});
				});
		}

		let parameterFormValue = "";
		new Setting(container)
			.addText((text) => {
				text.setPlaceholder("key").onChange(
					debounce(
						async (value) => {
							if (value.length > 0) {
								this.plugin.sessionSettings.parameters[value] =
									parameterFormValue;
								this.renderModels(container);
							}
						},
						debounceTime,
						true
					)
				);
			})
			.addText((text) => {
				text.setPlaceholder("value").onChange(async (value) => {
					parameterFormValue = value;
				});
			});
	}
	private toggleBookmarkNode(identifier: ULID) {
		if (!this.plugin.document) {
			return;
		}

		if (this.plugin.document.bookmarks.has(identifier)) {
			this.plugin.document.bookmarks.delete(identifier);
		} else {
			this.plugin.document.bookmarks.add(identifier);
			if (this.bookmarksMenu) {
				updateCollapsibleMenu(this.bookmarksMenu, true);
			}
		}

		this.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
	}
	async onOpen() {
		const container = this.containerEl.children[1] as HTMLElement;
		container.empty();

		const { workspace } = this.app;

		this.modelMenu = renderCollapsibleMenu(
			container,
			"Inference parameters"
		);
		this.bookmarksMenu = renderCollapsibleMenu(
			container,
			"Bookmarked nodes",
			["tapestry_tree", "tapestry_bookmarks"]
		);
		this.treeMenu = renderCollapsibleMenu(container, "Nearby nodes", [
			"tapestry_tree",
		]);

		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_LOAD_EVENT,
				() => {
					if (!this.bookmarksMenu || !this.treeMenu) {
						return;
					}
					this.renderBookmarks(this.bookmarksMenu.childrenContainer);
					this.renderTree(this.treeMenu.childrenContainer, false);
					if (this.plugin.document) {
						if (this.plugin.document.bookmarks.size > 0) {
							updateCollapsibleMenu(this.bookmarksMenu, true);
						} else {
							updateCollapsibleMenu(this.bookmarksMenu, false);
						}
					}
				}
			)
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_UPDATE_EVENT,
				() => {
					if (!this.bookmarksMenu || !this.treeMenu) {
						return;
					}
					this.renderBookmarks(this.bookmarksMenu.childrenContainer);
					this.renderTree(this.treeMenu.childrenContainer, true);
				}
			)
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_DROP_EVENT,
				() => {
					if (!this.bookmarksMenu || !this.treeMenu) {
						return;
					}
					this.renderBookmarks(this.bookmarksMenu.childrenContainer);
					this.renderTree(this.treeMenu.childrenContainer, false);
				}
			)
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				SETTINGS_UPDATE_EVENT,
				() => {
					if (
						!this.modelMenu ||
						!this.bookmarksMenu ||
						!this.treeMenu
					) {
						return;
					}
					this.renderModels(this.modelMenu.childrenContainer);
					this.renderBookmarks(this.bookmarksMenu.childrenContainer);
					this.renderTree(this.treeMenu.childrenContainer, false);
					if (this.plugin.document) {
						if (this.plugin.document.bookmarks.size > 0) {
							updateCollapsibleMenu(this.bookmarksMenu, true);
						} else {
							updateCollapsibleMenu(this.bookmarksMenu, false);
						}
					}
				}
			)
		);

		this.plugin.addCommand({
			id: "reset-tapestry-loom-tree-parameters",
			name: "Reset inference parameters to defaults",
			callback: () => {
				if (!this.modelMenu) {
					return;
				}
				this.plugin.sessionSettings = structuredClone(
					this.plugin.settings.defaultSession ||
						DEFAULT_SESSION_SETTINGS
				);
				this.renderModels(this.modelMenu.childrenContainer);
			},
		});
		this.plugin.addCommand({
			id: "node-tapestry-loom-toggle-folding",
			name: "Toggle whether current node is collapsed",
			callback: () => {
				const identifier = this.plugin.document?.currentNode;
				if (identifier) {
					this.toggleNodeExpansion(identifier);
				}
			},
		});
		this.plugin.addCommand({
			id: "node-tapestry-loom-collapse-folding-children",
			name: "Collapse children of current node",
			callback: () => {
				const identifier = this.plugin.document?.currentNode;
				if (identifier) {
					this.collapseNodeChildren(identifier);
				}
			},
		});
		this.plugin.addCommand({
			id: "node-tapestry-loom-expand-folding-children",
			name: "Expand children of current node",
			callback: () => {
				const identifier = this.plugin.document?.currentNode;
				if (identifier) {
					this.expandNodeChildren(identifier);
				}
			},
		});
		this.plugin.addCommand({
			id: "node-tapestry-loom-collapse-folding-all-inactive",
			name: "Collapse all inactive nodes down to current layer",
			callback: () => {
				this.collapseOtherNodes();
			},
		});
		this.plugin.addCommand({
			id: "node-tapestry-loom-expand-folding-all",
			name: "Expand all nodes down to current layer",
			callback: () => {
				this.expandNodes();
			},
		});

		if (this.plugin.document) {
			if (this.plugin.document.bookmarks.size > 0) {
				updateCollapsibleMenu(this.bookmarksMenu, true);
			} else {
				updateCollapsibleMenu(this.bookmarksMenu, false);
			}
		}
		this.renderModels(this.modelMenu.childrenContainer);
		this.renderBookmarks(this.bookmarksMenu.childrenContainer);
		this.renderTree(this.treeMenu.childrenContainer, false);
	}
	async onClose() {
		this.plugin.removeCommand("reset-tapestry-loom-tree-parameters");
		this.plugin.removeCommand("node-tapestry-loom-toggle-folding");
	}
}

interface CollapsibleMenuElement {
	item: HTMLElement;
	labelContainer: HTMLElement;
	childrenContainer: HTMLElement;
}

function renderCollapsibleMenu(
	root: HTMLElement,
	title: string,
	classes: Array<string> = []
): CollapsibleMenuElement {
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

	return {
		item: item,
		labelContainer: labelContainer,
		childrenContainer: container,
	};
}

function updateCollapsibleMenu(menu: CollapsibleMenuElement, open: boolean) {
	if (open) {
		menu.item.classList.remove("is-collapsed");
		menu.labelContainer.classList.remove("is-collapsed");
		menu.childrenContainer.style.display = "inherit";
	} else {
		menu.item.classList.add("is-collapsed");
		menu.labelContainer.classList.add("is-collapsed");
		menu.childrenContainer.style.display = "none";
	}
}

function renderMenuNotice(root: HTMLElement, text: string) {
	root.createEl("div", {
		text: text,
		cls: ["search-empty-state"],
	});
}

interface TreeElement {
	label: HTMLElement;
	labelContainer: HTMLElement;
	flairContainer?: HTMLElement;
	childrenContainer: HTMLElement;
}

function renderTree(
	root: HTMLElement,
	text: string,
	selected: boolean,
	collapsible: boolean,
	collapsed?: boolean,
	bookmarked?: boolean,
	flair?: string,
	collapseCallback?: (collapsed: boolean) => void
): TreeElement {
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
			if (collapseCallback) {
				collapseCallback(
					iconContainer.classList.contains("is-collapsed")
				);
			}
		});

		if (collapsed) {
			item.classList.add("is-collapsed");
			iconContainer.classList.add("is-collapsed");
			childrenContainer.style.display = "none";
		} else {
			item.classList.remove("is-collapsed");
			iconContainer.classList.remove("is-collapsed");
			childrenContainer.style.display = "inherit";
		}
	}

	const label = labelContainer.createEl("div", {
		cls: ["tree-item-inner"],
	});
	if (text.length > 0) {
		label.textContent = text.trim();
	} else {
		label.innerHTML = "<em>No text</em>";
		label.classList.add("tapestry_tree-notice");
	}

	const flairContainer = labelContainer.createEl("div", {
		cls: ["tree-item-flair-outer"],
	});

	if (bookmarked) {
		const bookmarkIcon = flairContainer.createEl("div", {
			text: flair,
			cls: ["tree-item-flair"],
		});
		setIcon(bookmarkIcon, "bookmark");
	}

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

function renderBookmarkNode(
	root: HTMLElement,
	text: string,
	selected: boolean
): TreeElement {
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

	const iconContainer = labelContainer.createEl("div", {
		cls: ["tree-item-icon"],
	});
	setIcon(iconContainer, "bookmark");

	const childrenContainer = item.createEl("div", {
		cls: ["tree-item-children"],
		attr: { dragable: false },
	});

	const label = labelContainer.createEl("div", {
		cls: ["tree-item-inner"],
	});
	if (text.length > 0) {
		label.textContent = text;
	} else {
		label.innerHTML = "<em>No text</em>";
		label.classList.add("tapestry_tree-notice");
	}

	return {
		label: label,
		labelContainer: labelContainer,
		childrenContainer: childrenContainer,
	};
}

function renderDepthNotice(tree: TreeElement) {
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
	tree: TreeElement,
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
		title: "Generate completions",
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

function renderBookmarkNodeButton(
	tree: TreeElement,
	bookmarked = true
): HTMLElement {
	const buttonContainer = tree.labelContainer.createEl("div", {
		cls: ["tapestry_tree-buttons"],
	});

	let bookmarkButton;

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

	return bookmarkButton;
}
