import { App, Notice, Plugin, PluginSettingTab, Setting } from "obsidian";
import { ClientSettings, ConfiguredEndpoint } from "client";
import TapestryLoom from "main";

export interface TapestryLoomSettings {
	client?: ClientSettings;
}

export class TapestryLoomSettingTab extends PluginSettingTab {
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
