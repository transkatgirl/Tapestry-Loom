import TapestryLoom, {
	DOCUMENT_DROP_EVENT,
	DOCUMENT_LOAD_EVENT,
	DOCUMENT_TRIGGER_UPDATE_EVENT,
	DOCUMENT_UPDATE_EVENT,
	SETTINGS_UPDATE_EVENT,
} from "main";
import { ItemView, Setting, WorkspaceLeaf, debounce, setIcon } from "obsidian";
import { getNodeContent, WeaveDocumentNode } from "document";
import { ULID, ulid } from "ulid";
import { runCompletion } from "client";
import { DEFAULT_DOCUMENT_SETTINGS } from "settings";

export const TREE_VIEW_TYPE = "tapestry-loom-view";

export interface SessionSettings {
	requests: number;
	models: Array<ULID>;
	parameters: Record<string, string>;
}

export const DEFAULT_SESSION_SETTINGS: SessionSettings = {
	requests: 6,
	models: [],
	parameters: { temperature: "1", max_tokens: "10" },
};

export class TapestryLoomTreeView extends ItemView {
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
		return TREE_VIEW_TYPE;
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

		buttons.generateButton.addEventListener("click", async (event) => {
			event.stopPropagation();
			await this.generateNodeChildren(node.identifier);
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
			modelLabel = document.models.get(node.model);
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
		}

		tree.labelContainer.addEventListener("click", () => {
			this.switchToNode(node.identifier);
		});

		const bookmarkToggleButton = renderBookmarkNodeButton(tree);

		bookmarkToggleButton.addEventListener("click", (event) => {
			event.stopPropagation();
			this.toggleBookmarkNode(node.identifier);
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

		const modelColors = new Map();
		for (const model of models) {
			if (model.label.color) {
				modelColors.set(model.ulid, model.label.color);
			}
		}

		new Setting(container).setName("Requests").addText((text) => {
			text.setPlaceholder((1).toString())
				.setValue(this.plugin.sessionSettings.requests.toString())
				.onChange(async (value) => {
					this.plugin.sessionSettings.requests = parseInt(value) || 1;
					if (this.plugin.sessionSettings.requests > 1) {
						this.plugin.sessionSettings.requests = 1;
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
							dropdown.selectEl.style.color =
								modelColors.get(value);
						});
					dropdown.selectEl.style.color = modelColors.get(
						this.plugin.sessionSettings.models[i]
					);
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
							debounce(async (value) => {
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
							}, debounceTime)
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
					debounce(async (value) => {
						if (value.length > 0) {
							this.plugin.sessionSettings.parameters[value] =
								parameterFormValue;
							this.renderModels(container);
						}
					}, debounceTime)
				);
			})
			.addText((text) => {
				text.setPlaceholder("value").onChange(async (value) => {
					parameterFormValue = value;
				});
			});
	}
	private async generateNodeChildren(parentNode?: ULID) {
		if (!this.plugin.document || !this.plugin.settings.client) {
			return;
		}

		const completions = await runCompletion(
			this.plugin.settings.client,
			this.plugin.sessionSettings.models,
			{
				prompt: this.plugin.document.getActiveContent(parentNode),
				count: this.plugin.sessionSettings.requests,
				parameters: this.plugin.sessionSettings.parameters,
			}
		);

		for (const completion of completions) {
			if (completion.topProbs && completion.topProbs.length > 0) {
				for (const prob of completion.topProbs) {
					this.plugin.document.addNode(
						{
							identifier: ulid(),
							content: [prob],
							model: completion.model.ulid,
							parentNode: parentNode,
						},
						completion.model.label
					);
				}
			} else {
				this.plugin.document.addNode(
					{
						identifier: ulid(),
						content: completion.completion,
						model: completion.model.ulid,
						parentNode: parentNode,
					},
					completion.model.label
				);
			}
		}

		this.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
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
			if (this.bookmarksMenu) {
				updateCollapsibleMenu(this.bookmarksMenu, true);
			}
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

		const modelMenu = renderCollapsibleMenu(
			container,
			"Inference parameters"
		);
		const bookmarksMenu = renderCollapsibleMenu(
			container,
			"Bookmarked nodes",
			["tapestry_tree", "tapestry_bookmarks"]
		);
		const treeMenu = renderCollapsibleMenu(container, "Nearby nodes", [
			"tapestry_tree",
		]);
		this.modelMenu = modelMenu;
		this.bookmarksMenu = bookmarksMenu;
		this.treeMenu = treeMenu;

		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_LOAD_EVENT,
				() => {
					this.renderBookmarks(bookmarksMenu.childrenContainer);
					this.renderTree(treeMenu.childrenContainer, false);
					if (this.plugin.document) {
						if (this.plugin.document.bookmarks.size > 0) {
							updateCollapsibleMenu(bookmarksMenu, true);
						} else {
							updateCollapsibleMenu(bookmarksMenu, false);
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
					this.renderBookmarks(bookmarksMenu.childrenContainer);
					this.renderTree(treeMenu.childrenContainer, true);
				}
			)
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				DOCUMENT_DROP_EVENT,
				() => {
					this.renderBookmarks(bookmarksMenu.childrenContainer);
					this.renderTree(treeMenu.childrenContainer, false);
				}
			)
		);
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				SETTINGS_UPDATE_EVENT,
				() => {
					this.renderModels(modelMenu.childrenContainer);
				}
			)
		);

		if (this.plugin.document) {
			if (this.plugin.document.bookmarks.size > 0) {
				updateCollapsibleMenu(bookmarksMenu, true);
			} else {
				updateCollapsibleMenu(bookmarksMenu, false);
			}
		}
		this.renderModels(modelMenu.childrenContainer);
		this.renderBookmarks(bookmarksMenu.childrenContainer);
		this.renderTree(treeMenu.childrenContainer, false);
	}
	async onClose() {}
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
		label.textContent = text;
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
