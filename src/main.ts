import { Plugin } from "obsidian";
import { TapestryLoomSettings, TapestryLoomSettingTab } from "settings";
import { editorPlugin, TapestryLoomView, VIEW_TYPE } from "view";

export default class TapestryLoom extends Plugin {
	settings: TapestryLoomSettings;

	async onload() {
		await this.loadSettings();

		this.registerView(
			VIEW_TYPE,
			(leaf) => new TapestryLoomView(leaf, this)
		);

		this.registerEditorExtension([editorPlugin]);

		this.addRibbonIcon("list-tree", "Toggle Tapestry Loom", () => {
			this.toggleView();
		});

		this.addSettingTab(new TapestryLoomSettingTab(this.app, this));
	}

	onunload() {}

	closeView() {
		const { workspace } = this.app;

		const leaves = workspace.getLeavesOfType(VIEW_TYPE);

		if (leaves.length > 0) {
			workspace.detachLeavesOfType(VIEW_TYPE);
		}
	}

	async toggleView() {
		const { workspace } = this.app;

		const leaves = workspace.getLeavesOfType(VIEW_TYPE);

		if (leaves.length > 0) {
			workspace.detachLeavesOfType(VIEW_TYPE);
		} else {
			const leaf = workspace.getRightLeaf(false);
			await leaf?.setViewState({ type: VIEW_TYPE, active: true });

			if (leaf) {
				workspace.revealLeaf(leaf);
			}
		}
	}

	async loadSettings() {
		this.settings = await this.loadData();
	}

	async saveSettings() {
		this.closeView();
		await this.saveData(this.settings);
	}
}
