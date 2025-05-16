import { Editor, ItemView, WorkspaceLeaf } from "obsidian";

export const VIEW_TYPE = "tapestry-loom-view";

export class TapestryLoomView extends ItemView {
	constructor(leaf: WorkspaceLeaf) {
		super(leaf);
	}

	getViewType() {
		return VIEW_TYPE;
	}

	getDisplayText() {
		return "Example view";
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
