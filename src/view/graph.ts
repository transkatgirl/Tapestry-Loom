import TapestryLoom, {
	DOCUMENT_DROP_EVENT,
	DOCUMENT_LOAD_EVENT,
	DOCUMENT_UPDATE_EVENT,
	SETTINGS_UPDATE_EVENT,
} from "main";
import { ItemView, WorkspaceLeaf } from "obsidian";
import { getNodeContent, WeaveDocument, WeaveDocumentNode } from "document";
import { ULID } from "ulid";
import cytoscape, { Core, StylesheetJsonBlock } from "cytoscape";
import { getGlobalCSSColorVariable, truncateWithEllipses } from "common";
import { switchToNode, toggleBookmarkNode } from "./common";
import { DEFAULT_DOCUMENT_SETTINGS } from "settings";

export const GRAPH_VIEW_TYPE = "tapestry-loom-graph-view";

const GRAPH_LAYOUT = {
	name: "elk",
	nodeDimensionsIncludeLabels: true,
	elk: {
		algorithm: "mrtree",
		interactive: true,
		"mrtree.searchOrder": "BFS",
	},
};

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

			const graphStyle: Array<StylesheetJsonBlock> = [
				{
					selector: "node",
					style: {
						label: "data(content)",
						"text-halign": "center",
						"text-valign": "bottom",
						"font-size": "12px",
						color: getGlobalCSSColorVariable("--graph-text"),
						"text-wrap": "wrap",
						"text-max-width": "140px",
						width: "20px",
						height: "20px",
						"background-color":
							getGlobalCSSColorVariable("--graph-node"),
					},
				},
				{
					selector: "edge",
					style: {
						"line-color": getGlobalCSSColorVariable("--graph-line"),
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
					selector: ".tapestry_graph-selected",
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
			const renderDepth =
				this.plugin.settings.document?.graphDepth ||
				DEFAULT_DOCUMENT_SETTINGS.graphDepth;

			if (incremental && this.graph) {
				this.graph.json({
					elements: elements,
				});

				if (!this.panned) {
					this.graph.fit(
						this.graph.elements(
							getPanSelector(document, renderDepth)
						)
					);
				}
			} else {
				container.empty();
				this.panned = false;

				this.graph = cytoscape({
					container: container,
					elements: elements,
					layout: GRAPH_LAYOUT,
					style: graphStyle,
				});
				this.graph.on("tap", "node", (event) => {
					const node = event.target.data().id;
					if (typeof node == "string") {
						switchToNode(this.plugin, node);
					}
				});
				this.graph.on("cxttap", "node", (event) => {
					const node = event.target.data().id;
					if (typeof node == "string") {
						toggleBookmarkNode(this.plugin, node);
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
				this.graph.on("ready move resize", (_event) => {
					if (!this.graph) {
						return;
					}
					this.graph.fit(
						this.graph.elements(
							getPanSelector(document, renderDepth)
						)
					);
					this.panned = false;
				});
				this.graph.on("add", (_event) => {
					if (!this.graph) {
						return;
					}
					this.graph.layout(GRAPH_LAYOUT).run();
				});
			}
		} else {
			container.empty();
			this.graph = undefined;
			this.panned = false;
		}
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

		const classes: string[] = [];
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

		if (document.currentNode == node.identifier) {
			classes.push("tapestry_graph-selected");
		}

		const edgeClasses: string[] = [];
		if (activeNodes.has(node.identifier)) {
			edgeClasses.push("tapestry_graph-selected");
		}

		elements.push({
			group: "nodes",
			data: {
				id: node.identifier,
				content: truncateWithEllipses(content.trim(), 60),
				model: modelLabel?.label,
			},
			classes: classes,
			style: style,
			selectable: false,
			grabbable: false,
		});
		if (node.parentNode) {
			elements.push({
				group: "edges",
				data: {
					id: node.parentNode + "-" + node.identifier,
					source: node.parentNode,
					target: node.identifier,
				},
				classes: edgeClasses,
				selectable: false,
			});
		}

		for (const childNode of document.getNodeChildren(node.identifier)) {
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
		this.registerEvent(
			workspace.on(
				// ignore ts2769; custom event
				// @ts-expect-error
				SETTINGS_UPDATE_EVENT,
				() => {
					if (this.graph && this.plugin.document) {
						const renderDepth =
							this.plugin.settings.document?.graphDepth ||
							DEFAULT_DOCUMENT_SETTINGS.graphDepth;

						this.graph.fit(
							this.graph.elements(
								getPanSelector(
									this.plugin.document,
									renderDepth
								)
							)
						);
						this.panned = false;
					}
				}
			)
		);

		this.plugin.addCommand({
			id: "reset-tapestry-loom-graph-zoom",
			name: "Reset node graph zoom",
			callback: () => {
				if (this.graph && this.plugin.document) {
					const renderDepth =
						this.plugin.settings.document?.graphDepth ||
						DEFAULT_DOCUMENT_SETTINGS.graphDepth;

					this.graph.fit(
						this.graph.elements(
							getPanSelector(this.plugin.document, renderDepth)
						)
					);
					this.panned = false;
				}
			},
		});

		if (this.plugin.document) {
			this.render(container, false);
		}
	}
	async onClose() {
		this.plugin.removeCommand("reset-tapestry-loom-graph-zoom");
	}
}

function getPanSelector(document: WeaveDocument, renderDepth: number): string {
	let selector = "";
	const activeNodes = document.getActiveNodes();

	if (
		document.currentNode &&
		activeNodes.length > renderDepth - 1 &&
		document.getNodeChildrenCount(document.currentNode) > 0
	) {
		for (const node of activeNodes.slice(-1 * (renderDepth - 1))) {
			if (selector.length > 0) {
				selector = selector + ",#" + node.identifier;
			} else {
				selector = "#" + node.identifier;
			}
		}
		for (const node of activeNodes.slice(-1 * (renderDepth - 1))) {
			for (const child of document.getNodeChildren(node.identifier)) {
				if (selector.length > 0) {
					selector = selector + ",#" + child.identifier;
				} else {
					selector = "#" + child.identifier;
				}
			}
		}
	} else if (document.currentNode && activeNodes.length > renderDepth) {
		for (const node of activeNodes.slice(-1 * renderDepth)) {
			if (selector.length > 0) {
				selector = selector + ",#" + node.identifier;
			} else {
				selector = "#" + node.identifier;
			}
		}
		for (const node of activeNodes.slice(-1 * renderDepth)) {
			for (const child of document.getNodeChildren(node.identifier)) {
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
			for (const child of document.getNodeChildren(node.identifier)) {
				if (selector.length > 0) {
					selector = selector + ",#" + child.identifier;
				} else {
					selector = "#" + child.identifier;
				}
			}
		}
		if (activeNodes.length > 0) {
			for (const node of activeNodes) {
				if (selector.length > 0) {
					selector = selector + ",#" + node.identifier;
				} else {
					selector = "#" + node.identifier;
				}
			}
			for (const node of activeNodes) {
				for (const child of document.getNodeChildren(node.identifier)) {
					if (selector.length > 0) {
						selector = selector + ",#" + child.identifier;
					} else {
						selector = "#" + child.identifier;
					}
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
