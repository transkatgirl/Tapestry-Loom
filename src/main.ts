import { Plugin } from "obsidian";
import { TapestryLoomSettings, TapestryLoomSettingTab } from "settings";
import {
	TapestryLoomView,
	VIEW_TYPE,
	VIEW_COMMANDS,
	EDITOR_PLUGIN,
} from "view";

export default class TapestryLoom extends Plugin {
	settings: TapestryLoomSettings;

	async onload() {
		await this.loadSettings();

		this.registerView(
			VIEW_TYPE,
			(leaf) => new TapestryLoomView(leaf, this)
		);

		this.showView();

		this.registerEditorExtension([EDITOR_PLUGIN]);

		this.addCommand({
			id: "show-tapestry-loom-view",
			name: "Show Tapestry Loom",
			callback: async () => {
				await this.showView();
			},
		});

		VIEW_COMMANDS.forEach((command) => this.addCommand(command));

		this.addSettingTab(new TapestryLoomSettingTab(this.app, this));
	}

	onunload() {}

	async showView() {
		const { workspace } = this.app;

		const leaves = workspace.getLeavesOfType(VIEW_TYPE);

		if (leaves.length > 0) {
			workspace.revealLeaf(leaves[0]);
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
		await this.saveData(this.settings);
	}
}
