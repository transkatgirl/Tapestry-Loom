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
	stringifyYaml,
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
import { ModelLabel } from "client";

export interface WeaveDocument {
	models: Map<ULID, ModelLabel>;
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

	if (frontMatterInfo.exists && FRONT_MATTER_KEY in frontMatter) {
		return frontMatter[FRONT_MATTER_KEY];
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

	if (frontMatterInfo.exists && FRONT_MATTER_KEY in frontMatter) {
		console.log(frontMatterInfo);
		console.log(content);

		// TODO
	} else {
		const nodes: Map<ULID, WeaveDocumentNode> = new Map();

		const identifier = ulid();

		nodes.set(identifier, {
			content: content,
		});

		const document: WeaveDocument = {
			models: new Map(),
			nodes: nodes,
			currentNode: identifier,
		};

		saveDocument(editor, document);
		return document;
	}
}

export function saveDocument(editor: Editor, document: WeaveDocument) {
	const rawContent = editor.getValue();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	const frontMatter = parseYaml(frontMatterInfo.frontmatter);

	if (frontMatterInfo.exists) {
		frontMatter[FRONT_MATTER_KEY] = document;
		editor.replaceRange(
			stringifyYaml(frontMatter),
			editor.offsetToPos(frontMatterInfo.from),
			editor.offsetToPos(frontMatterInfo.to)
		);
	} else {
		const newContent =
			"---\n" +
			stringifyYaml({ FRONT_MATTER_KEY: document }) +
			"\n---\n" +
			rawContent;
		editor.setValue(newContent);
	}
}
