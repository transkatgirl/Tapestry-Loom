import TapestryLoom, {
	DOCUMENT_DROP_EVENT,
	DOCUMENT_LOAD_EVENT,
	DOCUMENT_TRIGGER_UPDATE_EVENT,
	DOCUMENT_UPDATE_EVENT,
} from "main";
import { ItemView, WorkspaceLeaf } from "obsidian";
import { getNodeContent, WeaveDocument, WeaveDocumentNode } from "document";
import { ULID } from "ulid";
import cytoscape, { Core, StylesheetJsonBlock } from "cytoscape";

// @ts-expect-error
import crass from "crass";

export const GRAPH_VIEW_TYPE = "tapestry-loom-graph-view";

export class TapestryLoomGraphView extends ItemView {
	plugin: TapestryLoom;
	private graph?: Core;
	private panned = false;
	constructor(leaf: WorkspaceLeaf, plugin: TapestryLoom) {
		super(leaf);
		this.plugin = plugin;
	}
	getViewType() {
		return GRAPH_VIEW_TYPE;
	}
	getDisplayText() {
		return "Tapestry Loom Graph";
	}
	getIcon(): string {
		return "network";
	}
	private render(container: HTMLElement, incremental?: boolean) {
		const document = this.plugin.document;

		if (document) {
			const elements: Array<cytoscape.ElementDefinition> = [];

			const activeNodes = getActiveNodeIdentifiers(document);
			for (const node of document.getRootNodes()) {
				this.buildNode(elements, node, activeNodes);
			}
			if (incremental && this.graph) {
				this.graph.startBatch();

				const pan = this.graph.pan();
				const zoom = this.graph.zoom();

				this.graph.remove(this.graph.elements("*"));
				this.graph.add(elements);

				this.graph.endBatch();
				this.graph.createLayout({ name: "dagre" }).run();
				if (this.panned) {
					this.graph.pan(pan);
					this.graph.zoom(zoom);
				} else {
					this.graph.fit(
						this.graph.elements(getPanSelector(document))
					);
				}
			} else {
				container.empty();
				this.panned = false;

				const graphStyle: Array<StylesheetJsonBlock> = [
					{
						selector: "node",
						style: {
							label: "data(content)",
							"text-halign": "center",
							"text-valign": "bottom",
							"font-size":
								getGlobalCSSVariable("--font-ui-smaller"),
							color: getGlobalCSSColorVariable("--graph-text"),
							"text-wrap": "ellipsis",
							"text-max-width": "5em",
							"background-color":
								getGlobalCSSColorVariable("--graph-node"),
						},
					},
					{
						selector: "edge",
						style: {
							"line-color":
								getGlobalCSSColorVariable("--graph-line"),
						},
					},
					{
						selector: ".tapestry_graph-empty-node",
						style: {
							"background-color": getGlobalCSSColorVariable(
								"--graph-node-unresolved"
							),
						},
					},
					{
						selector: ".tapestry_graph-logit-node",
						style: {
							"text-valign": "center",
							"background-color":
								getGlobalCSSColorVariable("--graph-line"),
						},
					},
					{
						selector: ".tapestry_graph-bookmarked-node",
						style: {
							"background-color": getGlobalCSSColorVariable(
								"--graph-node-attachment"
							),
						},
					},
					{
						selector: ":selected",
						style: {
							"line-color": getGlobalCSSColorVariable(
								"--graph-node-focused"
							),
							"background-color": getGlobalCSSColorVariable(
								"--graph-node-focused"
							),
						},
					},
				];

				this.graph = cytoscape({
					container: container,
					elements: elements,
					layout: { name: "dagre" },
					style: graphStyle,
					headless: false,
				});
				this.graph.on("tap", "node", (event) => {
					const node = event.target.data().id;
					if (typeof node == "string") {
						this.switchToNode(node);
					}
				});
				this.graph.on("cxttap", "node", (event) => {
					const node = event.target.data().id;
					if (typeof node == "string") {
						this.toggleBookmarkNode(node);
					}
				});
				this.graph.on("dragpan", (_event) => {
					this.panned = true;
				});
				this.graph.on("pinchzoom", (_event) => {
					this.panned = true;
				});
				this.graph.on("scrollzoom", (_event) => {
					this.panned = true;
				});

				this.graph.fit(this.graph.elements(getPanSelector(document)));
			}
		} else {
			container.empty();
			this.graph = undefined;
			this.panned = false;
		}
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
	private buildNode(
		elements: Array<cytoscape.ElementDefinition>,
		node: WeaveDocumentNode,
		activeNodes: Set<ULID>
	) {
		const document = this.plugin.document;
		if (!document) {
			return;
		}

		const classes = [];
		let content = getNodeContent(node);
		if (content.length == 0) {
			classes.push("tapestry_graph-empty-node");
		}

		let modelLabel;
		let style;
		if (node.model) {
			modelLabel = document.models.get(node.model);
			style = {
				color: modelLabel?.color,
			};
		}

		if (document.bookmarks.has(node.identifier)) {
			classes.push("tapestry_graph-bookmarked-node");
		}

		if (
			node.content.length == 1 &&
			typeof node.content == "object" &&
			Array.isArray(node.content)
		) {
			classes.push("tapestry_graph-logit-node");
			content =
				"(" + (node.content[0][0] * 100).toFixed(0) + "%) " + content;
		}

		elements.push({
			group: "nodes",
			data: {
				id: node.identifier,
				content: content,
				model: modelLabel?.label,
			},
			classes: classes,
			style: style,
			selected: document.currentNode == node.identifier,
			selectable: false,
			grabbable: false,
		});
		if (node.parentNode) {
			elements.push({
				group: "edges",
				data: {
					source: node.parentNode,
					target: node.identifier,
				},
				selected: activeNodes.has(node.identifier),
				selectable: false,
			});
		}

		for (const childNode of document.getNodeChildren(node)) {
			this.buildNode(elements, childNode, activeNodes);
		}
	}
	async onOpen() {
		const root = this.containerEl.children[1] as HTMLElement;
		root.empty();
		root.style.padding = "var(--size-4-3) var(--size-4-3) var(--size-4-3)";

		const container = root.createEl("div", {
			cls: ["tapestry_graph"],
		});

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

function getPanSelector(document: WeaveDocument): string {
	let selector = "";
	const activeNodes = document.getActiveNodes();

	if (
		document.currentNode &&
		activeNodes.length > 3 &&
		document.getNodeChildrenCount(document.currentNode) > 0
	) {
		for (const node of activeNodes.slice(-6)) {
			if (selector.length > 0) {
				selector = selector + ",#" + node.identifier;
			} else {
				selector = "#" + node.identifier;
			}
		}
		for (const node of activeNodes.slice(-3)) {
			for (const child of document.getNodeChildren(node)) {
				if (selector.length > 0) {
					selector = selector + ",#" + child.identifier;
				} else {
					selector = "#" + child.identifier;
				}
			}
		}
	} else if (document.currentNode && activeNodes.length > 4) {
		for (const node of activeNodes.slice(-6)) {
			if (selector.length > 0) {
				selector = selector + ",#" + node.identifier;
			} else {
				selector = "#" + node.identifier;
			}
		}
		for (const node of activeNodes.slice(-4)) {
			for (const child of document.getNodeChildren(node)) {
				if (selector.length > 0) {
					selector = selector + ",#" + child.identifier;
				} else {
					selector = "#" + child.identifier;
				}
			}
		}
	} else {
		for (const node of document.getRootNodes()) {
			if (selector.length > 0) {
				selector = selector + ",#" + node.identifier;
			} else {
				selector = "#" + node.identifier;
			}
			for (const child of document.getNodeChildren(node)) {
				if (selector.length > 0) {
					selector = selector + ",#" + child.identifier;
				} else {
					selector = "#" + child.identifier;
				}
			}
		}
		if (activeNodes.length > 0) {
			for (const child of document.getNodeChildren(
				activeNodes[activeNodes.length - 1]
			)) {
				if (selector.length > 0) {
					selector = selector + ",#" + child.identifier;
				} else {
					selector = "#" + child.identifier;
				}
			}
		}
	}

	return selector;
}

function getActiveNodeIdentifiers(document: WeaveDocument): Set<ULID> {
	const identifiers: Set<ULID> = new Set();

	for (const node of document.getActiveNodes()) {
		identifiers.add(node.identifier);
	}

	return identifiers;
}

function getGlobalCSSVariable(key: string) {
	return window.getComputedStyle(window.document.body).getPropertyValue(key);
}

function getGlobalCSSColorVariable(key: string) {
	let parsed = crass.parse(
		"a{color:" +
			window
				.getComputedStyle(window.document.body)
				.getPropertyValue(key) +
			"}"
	);
	parsed = parsed.optimize();
	return parsed.toString().slice(8, -1);
}
