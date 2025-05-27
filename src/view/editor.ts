import { Extension, Range } from "@codemirror/state";
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

class TapestryLoomPlugin implements PluginValue {
	decorations: DecorationSet;
	document?: WeaveDocument;
	settings?: TapestryLoomSettings;
	constructor(view: EditorView) {
		this.decorations = this.buildDecorations(view);
	}
	update(update: ViewUpdate) {
		if (update.docChanged || update.viewportChanged) {
			this.decorations = this.buildDecorations(update.view);
		}
	}
	buildDecorations(view: EditorView): DecorationSet {
		if (!this.document || !this.settings) {
			return Decoration.none;
		}

		const decorations: Range<Decoration>[] = [];

		let offset = getEditorOffset(view);
		console.log(offset);

		for (const node of this.document.getActiveNodes()) {
			const contentLength = getNodeContent(node).length;

			if (contentLength > 0) {
				const from = offset;
				const to = offset + contentLength;
				offset = offset + contentLength;

				const range = Decoration.mark({
					class: "tapestry_editor-node",
				}).range(from, to);
				decorations.push(range);

				/*const range = Decoration.widget({
					widget: new NodeBorderWidget(),
					side: 0,
				}).range(from, from);
				decorations.push(range);*/
			}
		}

		return Decoration.set(decorations);
	}
	destroy() {}
}

const pluginSpec: PluginSpec<TapestryLoomPlugin> = {
	decorations: (value: TapestryLoomPlugin) => value.decorations,
};

export const EDITOR_PLUGIN = ViewPlugin.fromClass(
	TapestryLoomPlugin,
	pluginSpec
);

class NodeBorderWidget extends WidgetType {
	toDOM() {
		return document.createEl("span", {
			cls: "tapestry_editor-node",
			text: "test",
		});
	}
	eq() {
		return true;
	}
}

export function updateEditorPluginState(
	editor: Editor,
	settings?: TapestryLoomSettings,
	document?: WeaveDocument
) {
	// @ts-expect-error not typed
	const editorView = editor.cm as EditorView;
	const plugin = editorView.plugin(EDITOR_PLUGIN);
	if (!plugin) {
		return;
	}

	if (settings) {
		plugin.settings = settings;
	}
	plugin.document = document;
}

export function getEditorOffset(view: EditorView) {
	const rawContent = view.state.doc.toString();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	return frontMatterInfo.contentStart;
}
