import { Range } from "@codemirror/state";
import {
	Decoration,
	DecorationSet,
	ViewUpdate,
	EditorView,
	ViewPlugin,
	PluginSpec,
	PluginValue,
	WidgetType,
} from "@codemirror/view";
import { getNodeContent, WeaveDocument } from "document";
import { TapestryLoomSettings } from "settings";
import { Editor, getFrontMatterInfo } from "obsidian";
import { decodeTime } from "ulid";
import { UNKNOWN_MODEL_LABEL } from "client";

class TapestryLoomPlugin implements PluginValue {
	decorations: DecorationSet;
	document?: WeaveDocument;
	settings?: TapestryLoomSettings;
	constructor(
		view: EditorView,
		settings: TapestryLoomSettings,
		document?: WeaveDocument
	) {
		this.settings = settings;
		this.document = document;
		this.decorations = this.buildDecorations(view);
	}
	update(update: ViewUpdate) {
		if (update.docChanged || update.viewportChanged) {
			this.decorations = this.buildDecorations(update.view);
		}
	}
	buildDecorations(view: EditorView): DecorationSet {
		if (!this.document || !this.settings?.document?.renderOverlay) {
			return Decoration.none;
		}

		const decorations: Range<Decoration>[] = [];

		const content = view.state.doc.toString();
		let offset = getEditorOffset(content);

		for (const node of this.document.getActiveNodes()) {
			const nodeContent = getNodeContent(node);

			if (
				content.length >= offset + nodeContent.length &&
				content.substring(offset, offset + nodeContent.length) ==
					nodeContent
			) {
				const from = offset;
				const to = offset + nodeContent.length;
				offset = to;

				if (nodeContent.length == 0) {
					continue;
				}

				if (nodeContent.length > 0) {
					const attributes: Record<string, string> = {};
					let classString = "tapestry_editor-node";
					if (node.model) {
						const model =
							this.document.models.get(node.model) ||
							UNKNOWN_MODEL_LABEL;
						if (model?.color) {
							attributes["style"] = "color: " + model?.color;
						}
						if (model?.label) {
							attributes["title"] = model?.label;
							if (node.parameters) {
								for (const [key, value] of Object.entries(
									node.parameters
								)) {
									attributes["title"] =
										attributes["title"] +
										"\n" +
										key +
										": " +
										value;
								}
							}
						}
						classString =
							classString + " tapestry_editor-node-generated";
					}
					if ("title" in attributes) {
						attributes["title"] =
							attributes["title"] +
							"\n" +
							new Date(
								decodeTime(node.identifier)
							).toLocaleString();
					} else {
						attributes["title"] = new Date(
							decodeTime(node.identifier)
						).toLocaleString();
					}

					const range = Decoration.mark({
						class: classString,
						attributes: attributes,
					}).range(from, to);
					decorations.push(range);

					if (
						typeof node.content == "object" &&
						Array.isArray(node.content)
					) {
						let innerOffset = from;

						let i = 0;
						for (const [prob, token] of node.content) {
							const from = innerOffset;
							const to = innerOffset + token.length;
							innerOffset = to;
							i = i + 1;

							if (token.length == 0) {
								continue;
							}

							const range = Decoration.mark({
								class: "tapestry_editor-token",
								attributes: {
									style:
										"opacity: " +
										Math.max(
											1 - Math.log10(1 / prob) / 4,
											0.25
										).toString(),
								},
							}).range(from, to);
							decorations.push(range);

							if (i < node.content.length) {
								const borderRange = Decoration.widget({
									widget: new TokenBorderWidget(),
									side: -1,
								}).range(to, to);
								decorations.push(borderRange);
							}
						}
					}

					const borderRange = Decoration.widget({
						widget: new NodeBorderWidget(),
						side: -1,
					}).range(to, to);
					decorations.push(borderRange);
				}
			} else {
				break;
			}
		}

		return Decoration.set(decorations);
	}
	destroy() {}
}

class NodeBorderWidget extends WidgetType {
	toDOM() {
		const span = document.createElement("span");
		span.classList.add("tapestry_editor-node-border");
		return span;
	}
	eq() {
		return true;
	}
}

class TokenBorderWidget extends WidgetType {
	toDOM() {
		const span = document.createElement("span");
		span.classList.add("tapestry_editor-token-border");
		return span;
	}
	eq() {
		return true;
	}
}

export type EditorPlugin = ViewPlugin<TapestryLoomPlugin>;

export function buildEditorPlugin(
	settings: TapestryLoomSettings,
	document?: WeaveDocument
): EditorPlugin {
	const pluginSpec: PluginSpec<TapestryLoomPlugin> = {
		decorations: (value: TapestryLoomPlugin) => value.decorations,
	};

	return ViewPlugin.define((view) => {
		const plugin = new TapestryLoomPlugin(view, settings, document);

		return plugin;
	}, pluginSpec);
}

export function updateEditorPluginState(
	editorPlugin: EditorPlugin,
	editor: Editor,
	settings: TapestryLoomSettings,
	document?: WeaveDocument
) {
	// @ts-expect-error not typed
	const editorView = editor.cm as EditorView;
	const plugin = editorView.plugin(editorPlugin);
	if (!plugin) {
		return;
	}

	plugin.settings = settings;
	plugin.document = document;
	plugin.decorations = plugin.buildDecorations(editorView);
}

export function getEditorOffset(content: string) {
	const frontMatterInfo = getFrontMatterInfo(content);
	return frontMatterInfo.contentStart;
}
