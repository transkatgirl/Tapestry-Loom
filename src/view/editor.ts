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
import { getSegmentContent, WeaveDocument } from "document";
import { TapestryLoomSettings } from "settings";
import { Editor, getFrontMatterInfo } from "obsidian";

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

		for (const segment of this.document.getActiveContentSegmented()) {
			const segmentContent = getSegmentContent(segment);

			if (
				content.length >= offset + segmentContent.length &&
				content.substring(offset, offset + segmentContent.length) ==
					segmentContent
			) {
				const from = offset;
				const to = offset + segmentContent.length;
				offset = to;

				if (segmentContent.length > 0) {
					const attributes: Record<string, string> = {};
					let classString = "tapestry_editor-segment";
					if (segment.model) {
						if (segment.model.color) {
							attributes["style"] =
								"color: " + segment.model.color;
						}
						if (segment.model.label) {
							attributes["title"] = segment.model.label;
							if (segment.parameters) {
								for (const [key, value] of Object.entries(
									segment.parameters
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
							classString + " tapestry_editor-segment-generated";
					}
					if ("title" in attributes) {
						attributes["title"] =
							attributes["title"] +
							"\n" +
							segment.timestamp.toLocaleString();
					} else {
						attributes["title"] =
							segment.timestamp.toLocaleString();
					}

					const range = Decoration.mark({
						class: classString,
						attributes: attributes,
					}).range(from, to);
					decorations.push(range);

					if (
						typeof segment.content == "object" &&
						Array.isArray(segment.content)
					) {
						let innerOffset = from;

						let i = 0;
						for (const [prob, token] of segment.content) {
							const from = innerOffset;
							const to = innerOffset + token.length;
							innerOffset = to;
							i = i + 1;

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

							if (i < segment.content.length) {
								const borderRange = Decoration.widget({
									widget: new TokenBorderWidget(),
									side: -1,
								}).range(to, to);
								decorations.push(borderRange);
							}
						}
					}

					const borderRange = Decoration.widget({
						widget: new SegmentBorderWidget(),
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

class SegmentBorderWidget extends WidgetType {
	toDOM() {
		const span = document.createElement("span");
		span.classList.add("tapestry_editor-segment-border");
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
