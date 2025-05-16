import { Editor, ItemView, WorkspaceLeaf } from "obsidian";
import TapestryLoom from "main";

export const VIEW_TYPE = "tapestry-loom-view";

export class TapestryLoomView extends ItemView {
	plugin: TapestryLoom;

	constructor(leaf: WorkspaceLeaf, plugin: TapestryLoom) {
		super(leaf);
		this.plugin = plugin;
	}

	getViewType() {
		return VIEW_TYPE;
	}

	getDisplayText() {
		return "Tapestry Loom";
	}

	getIcon(): string {
		return "list-tree";
	}

	async updateDocument(editor: Editor) {}

	async onOpen() {
		const container = this.containerEl.children[1];
		container.empty();
		container.createEl("h4", { text: "Created on " + Date.now() });
	}

	async onClose() {
		// Nothing to clean up.
	}
}
