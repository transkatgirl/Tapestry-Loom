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
		if (!this.document || !this.settings) {
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

				if (nodeContent.length > 0) {
					const range = Decoration.mark({
						class: "tapestry_editor-node",
					}).range(from, to);
					decorations.push(range);
				}

				/*const range = Decoration.widget({
					widget: new NodeBorderWidget(),
					side: 0,
				}).range(from, from);
				decorations.push(range);*/
			} else {
				break;
			}
		}

		return Decoration.set(decorations);
	}
	destroy() {}
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
}

export function getEditorOffset(content: string) {
	const frontMatterInfo = getFrontMatterInfo(content);
	return frontMatterInfo.contentStart;
}
