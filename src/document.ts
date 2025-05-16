import {
	getFrontMatterInfo,
	debounce,
	parseYaml,
	Command,
	Editor,
	ItemView,
	EventRef,
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
import { ULID } from "ulid";

export interface WeaveDocument {
	models: Map<ULID, string>;
	nodes: Map<ULID, WeaveDocumentNode>;
	currentNode: ULID;
}

export interface WeaveDocumentNode {
	content: string | Array<[number, string]> | Array<Array<[number, string]>>;
	model?: ULID;
	metadata: Map<string, string>;
	children: Set<ULID>;
}

export const FRONT_MATTER_KEY = "TapestryLoomWeave";

export function loadDocument() {
	const { workspace } = this.app;
	const editor = workspace.activeEditor?.editor;

	if (!editor) {
		return;
	}

	const rawContent = editor.getValue();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	const frontMatter = parseYaml(frontMatterInfo.frontmatter);
	const content = rawContent.substring(frontMatterInfo.contentStart);

	if (
		frontMatter &&
		typeof frontMatter === "object" &&
		FRONT_MATTER_KEY in frontMatter
	) {
		return frontMatter[FRONT_MATTER_KEY];
	}
}

export function refreshDocument() {
	const { workspace } = this.app;
	const editor = workspace.activeEditor?.editor;

	if (!editor) {
		return;
	}

	const rawContent = editor.getValue();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	const frontMatter = parseYaml(frontMatterInfo.frontmatter);
	const content = rawContent.substring(frontMatterInfo.contentStart);

	/*if (typeof frontMatter === "object" && FRONT_MATTER_KEY in frontMatter) {
	}*/

	console.log(frontMatterInfo);
	console.log(content);
}
