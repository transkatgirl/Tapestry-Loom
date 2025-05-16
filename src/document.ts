import {
	getFrontMatterInfo,
	debounce,
	parseYaml,
	Command,
	Editor,
	ItemView,
	EventRef,
	Workspace,
	WorkspaceLeaf,
} from "obsidian";
import TapestryLoom from "main";
import {
	App,
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
import { ulid, ULID } from "ulid";

export interface WeaveDocument {
	models: Map<ULID, string>;
	nodes: Map<ULID, WeaveDocumentNode>;
	currentNode: ULID;
}

export interface WeaveDocumentNode {
	content: string | Array<[number, string]> | Array<Array<[number, string]>>;
	model?: ULID;
	parentNode?: ULID;
	metadata?: Map<string, string>;
}

export const FRONT_MATTER_KEY = "TapestryLoomWeave";

export function loadDocument(editor: Editor): WeaveDocument {
	const rawContent = editor.getValue();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	const frontMatter = parseYaml(frontMatterInfo.frontmatter);
	const content = rawContent.substring(frontMatterInfo.contentStart);

	if (
		frontMatter &&
		typeof frontMatter === "object" &&
		FRONT_MATTER_KEY in frontMatter
	) {
		return JSON.parse(frontMatter[FRONT_MATTER_KEY]);
	} else {
		const nodes: Map<ULID, WeaveDocumentNode> = new Map();

		const identifier = ulid();

		nodes.set(identifier, {
			content: content,
		});

		return {
			models: new Map(),
			nodes: nodes,
			currentNode: identifier,
		};
	}
}

export function refreshDocument(editor: Editor) {
	const rawContent = editor.getValue();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	const frontMatter = parseYaml(frontMatterInfo.frontmatter);
	const content = rawContent.substring(frontMatterInfo.contentStart);

	/*if (typeof frontMatter === "object" && FRONT_MATTER_KEY in frontMatter) {
	}*/

	console.log(frontMatterInfo);
	console.log(content);
}

export function saveDocument(editor: Editor, document: WeaveDocument) {}
