import TapestryLoom, {
	DOCUMENT_DROP_EVENT,
	DOCUMENT_LOAD_EVENT,
	DOCUMENT_TRIGGER_UPDATE_EVENT,
	DOCUMENT_UPDATE_EVENT,
} from "main";
import { ItemView, WorkspaceLeaf } from "obsidian";
import { getNodeContent, WeaveDocument, WeaveDocumentNode } from "document";
import { ULID, ulid } from "ulid";
import cytoscape, { Core, StylesheetJsonBlock } from "cytoscape";
// @ts-expect-error
import crass from "crass";

// TODO: Use HoverPopover

export const GRAPH_VIEW_TYPE = "tapestry-loom-graph-view";

const GRAPH_STYLE: Array<StylesheetJsonBlock> = [
	{
		selector: "node",
		style: {
			label: "data(content)",
			"font-size": getGlobalCSSVariable("--font-ui-smaller"),
			color: getGlobalCSSColorVariable("--graph-text"),
			"text-wrap": "ellipsis",
			"text-max-width": "8em",
			"background-color": getGlobalCSSColorVariable("--graph-node"),
		},
	},
	{
		selector: ".tapestry_graph-empty-node",
		style: {
			"background-color": getGlobalCSSColorVariable("--text-faint"),
		},
	},
	{
		selector: ":grabbed",
		style: {
			color: getGlobalCSSColorVariable("--nav-item-color-hover"),
			"line-color": getGlobalCSSColorVariable(
				"--nav-item-background-hover"
			),
			"background-color": getGlobalCSSColorVariable(
				"--nav-item-background-hover"
			),
		},
	},
	{
		selector: ":selected",
		style: {
			color: getGlobalCSSColorVariable("--nav-item-color-selected"),
			"line-color": getGlobalCSSColorVariable("--graph-node-focused"),
			"background-color": getGlobalCSSColorVariable(
				"--graph-node-focused"
			),
		},
	},
];

export class TapestryLoomGraphView extends ItemView {
	plugin: TapestryLoom;
	graph?: Core;
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
	render(container: HTMLElement, incremental?: boolean) {
		const document = this.plugin.document;

		if (document) {
			if (incremental && this.graph) {
				this.graph.startBatch();

				this.graph.remove(this.graph.elements("*"));

				const elements: Array<cytoscape.ElementDefinition> = [];

				const activeNodes = getActiveNodeIdentifiers(document);
				for (const node of document.getRootNodes()) {
					this.buildNode(elements, node, activeNodes);
				}

				this.graph.add(elements);

				this.graph.endBatch();
				this.graph.createLayout({ name: "dagre" }).run();
			} else {
				container.empty();

				const elements: Array<cytoscape.ElementDefinition> = [];

				const activeNodes = getActiveNodeIdentifiers(document);
				for (const node of document.getRootNodes()) {
					this.buildNode(elements, node, activeNodes);
				}

				this.graph = cytoscape({
					container: container,
					elements: elements,
					layout: { name: "dagre" },
					style: GRAPH_STYLE,
				});
				this.graph.on("tap", "node", (event) => {
					const node = event.target.data().id;
					if (typeof node == "string") {
						this.switchToNode(node);
					}
				});
			}
		} else {
			container.empty();
			this.graph = undefined;
		}
	}
	switchToNode(identifier: ULID) {
		if (!this.plugin.document) {
			return;
		}

		this.plugin.document.currentNode = identifier;
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

		let classes;
		const content = getNodeContent(node);
		if (content.length == 0) {
			classes = ["tapestry_graph-empty-node"];
		}

		let modelLabel;
		let style;
		if (node.model) {
			modelLabel = document.models.get(node.model);
			style = {
				color: "modelLabel?.color",
			};
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
