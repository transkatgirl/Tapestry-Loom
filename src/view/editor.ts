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
import { getNodeContent, WeaveDocument, WeaveDocumentNode } from "document";
import { ULID, ulid } from "ulid";
import { TapestryLoomSettings } from "settings";
import { Editor } from "obsidian";

class TapestryLoomPlugin implements PluginValue {
	decorations: DecorationSet;
	constructor(_view: EditorView) {
		this.decorations = Decoration.none;
	}
	update(_update: ViewUpdate) {}
	handleTapestryDocumentLoad(
		document: WeaveDocument,
		settings: TapestryLoomSettings
	) {
		// TODO
	}
	handleTapestryDocumentUpdate(
		document: WeaveDocument,
		settings: TapestryLoomSettings
	) {
		// TODO
	}
	handleTapestryDocumentDestroy(settings: TapestryLoomSettings) {
		// TODO
	}
	destroy() {}
}

export const EDITOR_PLUGIN = ViewPlugin.fromClass(TapestryLoomPlugin);

export function updateEditorPluginState(
	editor: Editor,
	settings: TapestryLoomSettings,
	document?: WeaveDocument,
	incremental?: boolean
) {
	// @ts-expect-error not typed
	const editorView = editor.cm as EditorView;
	const plugin = editorView.plugin(EDITOR_PLUGIN);
	if (!plugin) {
		return;
	}

	if (document) {
		if (incremental) {
			plugin.handleTapestryDocumentUpdate(document, settings);
		} else {
			plugin.handleTapestryDocumentLoad(document, settings);
		}
	} else {
		plugin.handleTapestryDocumentDestroy(settings);
	}
}
