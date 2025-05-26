import TapestryLoom from "main";
import {
	App,
	Command,
	ItemView,
	Menu,
	Modal,
	Setting,
	WorkspaceLeaf,
	setIcon,
} from "obsidian";
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
import { getNodeContent, WeaveDocument, WeaveDocumentNode } from "document";
import { ULID, ulid } from "ulid";
import { TapestryLoomSettings } from "settings";

export class TapestryLoomPlugin implements PluginValue {
	decorations: DecorationSet;
	constructor(_view: EditorView) {
		this.decorations = Decoration.none;
	}

	update(_update: ViewUpdate) {}
	handleTapestryDocumentLoad(
		document: WeaveDocument,
		settings: TapestryLoomSettings
	) {}
	handleTapestryDocumentUpdate(
		document: WeaveDocument,
		settings: TapestryLoomSettings
	) {}
	handleTapestryDocumentDestroy(settings: TapestryLoomSettings) {}

	destroy() {}
}

export const EDITOR_PLUGIN = ViewPlugin.fromClass(TapestryLoomPlugin);
