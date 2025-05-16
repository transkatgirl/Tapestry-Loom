import { App, Notice, Plugin, WorkspaceLeaf } from "obsidian";
import {
	TapestryLoomSettings,
	TapestryLoomSettingTab,
	DEFAULT_SETTINGS,
} from "settings";
import { TapestryLoomView, VIEW_TYPE } from "view";

export default class TapestryLoom extends Plugin {
	settings: TapestryLoomSettings;

	async onload() {
		await this.loadSettings();

		this.registerView(VIEW_TYPE, (leaf) => new TapestryLoomView(leaf));

		this.addRibbonIcon("dice", "Open Tapestry Loom", () => {
			this.activateView();
		});

		this.addSettingTab(new TapestryLoomSettingTab(this.app, this));
	}

	onunload() {}

	async activateView() {
		const { workspace } = this.app;

		let leaf: WorkspaceLeaf | null = null;
		const leaves = workspace.getLeavesOfType(VIEW_TYPE);

		if (leaves.length > 0) {
			leaf = leaves[0];
		} else {
			leaf = workspace.getRightLeaf(false);
			await leaf?.setViewState({ type: VIEW_TYPE, active: true });
		}

		if (leaf) {
			workspace.revealLeaf(leaf);
		}
	}

	async loadSettings() {
		this.settings = Object.assign(
			{},
			DEFAULT_SETTINGS,
			await this.loadData()
		);
	}

	async saveSettings() {
		await this.saveData(this.settings);
	}
}
