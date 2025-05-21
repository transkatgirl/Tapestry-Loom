import TapestryLoom, {
	DOCUMENT_DROP_EVENT,
	DOCUMENT_LOAD_EVENT,
	DOCUMENT_TRIGGER_UPDATE_EVENT,
	DOCUMENT_UPDATE_EVENT,
} from "main";
import { ItemView, WorkspaceLeaf } from "obsidian";
import { getNodeContent, WeaveDocumentNode } from "document";
import { ULID, ulid } from "ulid";
import cytoscape from "cytoscape";

// TODO: Use HoverPopover

export const GRAPH_VIEW_TYPE = "tapestry-loom-graph-view";

export class TapestryLoomGraphView extends ItemView {
	plugin: TapestryLoom;
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
		container.empty();

		if (document) {
			console.log(container);

			this.renderGraph(container);

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
			this.buildNode(elements, node);
		}

		const cy = cytoscape({
			container: container,
			elements: elements,
			layout: { name: "dagre" },
		});
	}
	private buildNode(
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
			this.buildNode(elements, childNode);
		}
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
