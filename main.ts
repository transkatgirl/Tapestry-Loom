import { App, Notice, Plugin, PluginSettingTab, Setting } from "obsidian";
import { ClientSettings, ConfiguredEndpoint } from "client";

// Remember to rename these classes and interfaces!

interface TapestryLoomSettings {
	client: ClientSettings;
	endpoints: Array<ConfiguredEndpoint>;
}

const DEFAULT_SETTINGS: TapestryLoomSettings = {
	client: {},
	endpoints: [],
};

export default class TapestryLoom extends Plugin {
	settings: TapestryLoomSettings;

	async onload() {
		await this.loadSettings();

		// This creates an icon in the left ribbon.
		const ribbonIconEl = this.addRibbonIcon(
			"dice",
			"Sample Plugin",
			(evt: MouseEvent) => {
				// Called when the user clicks the icon.
				new Notice("This is a notice!");
			}
		);
		// Perform additional things with the ribbon
		ribbonIconEl.addClass("my-plugin-ribbon-class");

		// This adds a status bar item to the bottom of the app. Does not work on mobile apps.
		const statusBarItemEl = this.addStatusBarItem();
		statusBarItemEl.setText("Status Bar Text");

		// This adds a settings tab so the user can configure various aspects of the plugin
		this.addSettingTab(new TapestryLoomSettingTab(this.app, this));
	}

	onunload() {}

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

class TapestryLoomSettingTab extends PluginSettingTab {
	plugin: TapestryLoom;

	constructor(app: App, plugin: TapestryLoom) {
		super(app, plugin);
		this.plugin = plugin;
	}

	display(): void {
		const { containerEl } = this;

		containerEl.empty();

		containerEl.createEl("h1", { text: "Heading 1" });

		//new Setting(containerEl)
		//	.setName("Setting #1")
		//	.setDesc("It's a secret")
		//	.addText((text) =>
		//		text
		//			.setPlaceholder("Enter your secret")
		//			.setValue(this.plugin.settings.mySetting)
		//			.onChange(async (value) => {
		//				this.plugin.settings.mySetting = value;
		//				await this.plugin.saveSettings();
		//			})
		//	);
	}
}
