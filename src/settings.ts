import { App, PluginSettingTab, Setting } from "obsidian";
import { ClientSettings, ConfiguredEndpoint } from "client";
import TapestryLoom from "main";

export interface TapestryLoomSettings {
	client?: ClientSettings;
	document?: DocumentSettings;
}

export const DEFAULT_CLIENT_SETTINGS: ClientSettings = { models: [] };
export const DEFAULT_DOCUMENT_SETTINGS: DocumentSettings = { debounce: 500 };

export interface DocumentSettings {
	debounce: number;
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

		this.plugin.settings.client =
			this.plugin.settings.client || DEFAULT_CLIENT_SETTINGS;
		this.plugin.settings.document =
			this.plugin.settings.document || DEFAULT_DOCUMENT_SETTINGS;

		const client = this.plugin.settings.client;
		const document = this.plugin.settings.document;

		new Setting(containerEl)
			.setName("Debounce time")
			.setDesc(
				"Time to wait after last document update before refreshing nodes. Requires plugin restart to take effect."
			)
			.addText((text) =>
				text
					.setPlaceholder(
						DEFAULT_DOCUMENT_SETTINGS.debounce.toString()
					)
					.setValue(document.debounce.toString())
					.onChange(async (value) => {
						this.plugin.settings.document =
							this.plugin.settings.document ||
							DEFAULT_DOCUMENT_SETTINGS;
						this.plugin.settings.document.debounce =
							parseInt(value);
						await this.plugin.saveSettings();
						//this.display();
					})
			);

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
