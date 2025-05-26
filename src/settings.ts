import { App, debounce, HexString, PluginSettingTab, Setting } from "obsidian";
import { arrayMoveMutable } from "array-move";
import { ClientSettings, EndpointType, newModel } from "client";
import TapestryLoom from "main";
import { SessionSettings } from "view/tree";

export interface TapestryLoomSettings {
	client?: ClientSettings;
	document?: DocumentSettings;
	defaultSession?: SessionSettings;
}

export const DEFAULT_CLIENT_SETTINGS: ClientSettings = { models: [] };
export const DEFAULT_DOCUMENT_SETTINGS: DocumentSettings = {
	debounce: 500,
	treeDepth: 4,
	graphDepth: 4,
};
export const DEFAULT_SESSION_SETTINGS: SessionSettings = {
	requests: 6,
	models: [],
	parameters: { temperature: "1", max_tokens: "10" },
};

const DEFAULT_LABEL_COLOR: HexString = "#000000";

export interface DocumentSettings {
	debounce: number;
	treeDepth: number;
	graphDepth: number;
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
				"Milliseconds to wait after last document update before refreshing nodes. Requires plugin restart to take effect."
			)
			.addText((text) =>
				text
					.setPlaceholder(
						DEFAULT_DOCUMENT_SETTINGS.debounce.toString()
					)
					.setValue(document.debounce.toString())
					.onChange(async (value) => {
						document.debounce =
							parseInt(value) ||
							DEFAULT_DOCUMENT_SETTINGS.debounce;
						if (document.debounce < 30) {
							document.debounce = 30;
						}
						this.plugin.settings.document = document;
						await this.plugin.saveSettings();
					})
			);

		new Setting(containerEl)
			.setName("Displayed Tree Depth")
			.setDesc("")
			.addText((text) =>
				text
					.setPlaceholder(
						DEFAULT_DOCUMENT_SETTINGS.treeDepth.toString()
					)
					.setValue(document.treeDepth.toString())
					.onChange(async (value) => {
						document.treeDepth =
							parseInt(value) ||
							DEFAULT_DOCUMENT_SETTINGS.treeDepth;
						if (document.treeDepth < 2) {
							document.treeDepth = 2;
						}

						this.plugin.settings.document = document;
						await this.plugin.saveSettings();
					})
			);

		new Setting(containerEl)
			.setName("Displayed Graph Depth")
			.setDesc("")
			.addText((text) =>
				text
					.setPlaceholder(
						DEFAULT_DOCUMENT_SETTINGS.graphDepth.toString()
					)
					.setValue(document.graphDepth.toString())
					.onChange(async (value) => {
						document.graphDepth =
							parseInt(value) ||
							DEFAULT_DOCUMENT_SETTINGS.graphDepth;
						if (document.graphDepth < 2) {
							document.graphDepth = 2;
						}
						this.plugin.settings.document = document;
						await this.plugin.saveSettings();
					})
			);

		new Setting(containerEl)
			.setName("Inference parameter defaults")
			.setDesc(
				"Modify the inference parameters used when starting a new session."
			)
			.addButton((button) => {
				button
					.setButtonText("Replace defaults with current values")
					.onClick(async (_event) => {
						this.plugin.settings.defaultSession =
							this.plugin.sessionSettings;
						await this.plugin.saveSettings();
						this.display();
					});
			})
			.addButton((button) => {
				button
					.setButtonText("Reset defaults")
					.onClick(async (_event) => {
						this.plugin.settings.defaultSession = undefined;
						await this.plugin.saveSettings();
						this.display();
					});
			});

		new Setting(containerEl).setHeading().setName("Models");
		const modelForm = {
			url: "",
			identifier: "",
			apiKey: "",
			type: "openai_completion_v1_compatible",
		};
		new Setting(containerEl)
			.addText((text) => {
				text.setPlaceholder("Request URL").onChange((value) => {
					modelForm.url = value;
				});
			})
			.addText((text) => {
				text.setPlaceholder("Model Identifier (Optional)").onChange(
					(value) => {
						modelForm.identifier = value;
					}
				);
			})
			.addText((text) => {
				text.setPlaceholder("API Key (Optional)").onChange((value) => {
					modelForm.apiKey = value;
				});
			})
			.addDropdown((dropdown) => {
				dropdown
					.addOption(
						"openai_completion_v1_compatible",
						"OpenAI v1 (or similar) Completion"
					)
					.setValue(modelForm.type)
					.onChange((value) => {
						modelForm.type = value;
					});
			})
			.addButton((button) => {
				button.setButtonText("Add model").onClick(async (_event) => {
					if (modelForm.type.length > 0) {
						const model = newModel(
							modelForm.type as EndpointType,
							modelForm.url,
							modelForm.identifier,
							modelForm.apiKey
						);
						if (model) {
							client.models.push(model);
							this.plugin.settings.client = client;
							await this.plugin.saveSettings();
							this.display();
						}
					}
				});
			});

		for (let i = 0; i < client.models.length; i++) {
			new Setting(containerEl).setHeading().setName("Edit model");
			const labelSetting = new Setting(containerEl)
				.setName("Label")
				.addText((text) => {
					text.setPlaceholder("Label")
						.setValue(client.models[i].label.label)
						.onChange(async (value) => {
							client.models[i].label.label = value;
							this.plugin.settings.client = client;
							await this.plugin.saveSettings();
						});
				});
			const color = client.models[i].label.color;
			if (color) {
				labelSetting
					.addColorPicker((colorPicker) => {
						colorPicker.setValue(color).onChange(async (value) => {
							client.models[i].label.color = value;
							this.plugin.settings.client = client;
							await this.plugin.saveSettings();
						});
					})
					.addButton((button) => {
						button.setIcon("rotate-ccw");
						button.onClick(async (_event) => {
							client.models[i].label.color = undefined;
							this.plugin.settings.client = client;
							await this.plugin.saveSettings();
							this.display();
						});
					});
			} else {
				labelSetting.addButton((button) => {
					button.setIcon("palette");
					button.onClick(async (_event) => {
						client.models[i].label.color = DEFAULT_LABEL_COLOR;
						this.plugin.settings.client = client;
						await this.plugin.saveSettings();
						this.display();
					});
				});
			}

			new Setting(containerEl)
				.setName("Endpoint")
				.addText((text) => {
					text.setPlaceholder("Request URL")
						.setValue(client.models[i].url)
						.onChange(async (value) => {
							client.models[i].url = value;
							this.plugin.settings.client = client;
							await this.plugin.saveSettings();
						});
				})
				.addDropdown((dropdown) => {
					dropdown
						.addOption(
							"openai_completion_v1_compatible",
							"OpenAI v1 (or similar) Completion"
						)
						.setValue(client.models[i].type)
						.onChange(async (value) => {
							client.models[i].type = value as EndpointType;
							this.plugin.settings.client = client;
							await this.plugin.saveSettings();
						});
					dropdown.selectEl.style.maxWidth = "max-content";
				});

			for (let [headerKey, headerValue] of Object.entries(
				client.models[i].headers
			)) {
				new Setting(containerEl)
					.setName("Request header")
					.addText((text) => {
						text.setPlaceholder("key")
							.setValue(headerKey)
							.onChange(
								debounce(async (value) => {
									if (value.length > 0) {
										delete client.models[i].headers[
											headerKey
										];
										client.models[i].headers[value] =
											headerValue;
										headerKey = value;
										this.plugin.settings.client = client;
										await this.plugin.saveSettings();
									} else {
										delete client.models[i].headers[
											headerKey
										];
										this.plugin.settings.client = client;
										await this.plugin.saveSettings();
										this.display();
									}
								}, document.debounce)
							);
					})
					.addText((text) => {
						text.setPlaceholder("value")
							.setValue(headerValue)
							.onChange(async (value) => {
								client.models[i].headers[headerKey] = value;
								headerValue = value;
								this.plugin.settings.client = client;
								await this.plugin.saveSettings();
							});
					});
			}
			let headerFormValue = "";
			new Setting(containerEl)
				.setName("Request header")
				.addText((text) => {
					text.setPlaceholder("key").onChange(
						debounce(async (value) => {
							if (value.length > 0) {
								client.models[i].headers[value] =
									headerFormValue;
								this.plugin.settings.client = client;
								await this.plugin.saveSettings();
								this.display();
							}
						}, document.debounce)
					);
				})
				.addText((text) => {
					text.setPlaceholder("value").onChange(async (value) => {
						headerFormValue = value;
					});
				});

			for (let [parameterKey, parameterValue] of Object.entries(
				client.models[i].parameters
			)) {
				new Setting(containerEl)
					.setName("Request parameter")
					.addText((text) => {
						text.setPlaceholder("key")
							.setValue(parameterKey)
							.onChange(
								debounce(async (value) => {
									if (value.length > 0) {
										delete client.models[i].parameters[
											parameterKey
										];
										client.models[i].parameters[value] =
											parameterValue;
										parameterKey = value;
										this.plugin.settings.client = client;
										await this.plugin.saveSettings();
									} else {
										delete client.models[i].parameters[
											parameterKey
										];
										this.plugin.settings.client = client;
										await this.plugin.saveSettings();
										this.display();
									}
								}, document.debounce)
							);
					})
					.addText((text) => {
						text.setPlaceholder("value")
							.setValue(parameterValue)
							.onChange(async (value) => {
								client.models[i].parameters[parameterKey] =
									value;
								parameterValue = value;
								this.plugin.settings.client = client;
								await this.plugin.saveSettings();
							});
					});
			}

			let parameterFormValue = "";
			new Setting(containerEl)
				.setName("Request parameter")
				.addText((text) => {
					text.setPlaceholder("key").onChange(
						debounce(async (value) => {
							if (value.length > 0) {
								client.models[i].parameters[value] =
									parameterFormValue;
								this.plugin.settings.client = client;
								await this.plugin.saveSettings();
								this.display();
							}
						}, document.debounce)
					);
				})
				.addText((text) => {
					text.setPlaceholder("value").onChange(async (value) => {
						parameterFormValue = value;
					});
				});

			const moveSetting = new Setting(containerEl);
			if (i > 0) {
				moveSetting.addButton((button) => {
					button
						.setIcon("arrow-up-from-dot")
						.onClick(async (_event) => {
							arrayMoveMutable(client.models, i, i - 1);
							this.plugin.settings.client = client;
							await this.plugin.saveSettings();
							this.display();
						});
				});
			}
			if (i < client.models.length - 1) {
				moveSetting.addButton((button) => {
					button
						.setIcon("arrow-down-to-dot")
						.onClick(async (_event) => {
							arrayMoveMutable(client.models, i, i + 1);
							this.plugin.settings.client = client;
							await this.plugin.saveSettings();
							this.display();
						});
				});
			}
			moveSetting.addButton((button) => {
				button.setIcon("trash").onClick(async (_event) => {
					client.models.splice(i, 1);
					this.plugin.settings.client = client;
					await this.plugin.saveSettings();
					this.display();
				});
			});
		}
	}
}
