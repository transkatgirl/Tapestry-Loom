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
	content: string | Array<[number, string]>;
	model?: ULID;
	parentNode?: ULID;
	metadata?: Map<string, string>;
}

export const FRONT_MATTER_KEY = "TapestryLoomWeave";

function cleanDocument(document: WeaveDocument): WeaveDocument {
	// TODO
	document.nodes = new Map(Object.entries(document.nodes));

	return document;
}

export function loadDocument(editor: Editor) {
	const rawContent = editor.getValue();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	const frontMatter = parseYaml(frontMatterInfo.frontmatter);
	const content = rawContent.substring(frontMatterInfo.contentStart);

	if (frontMatterInfo.exists && FRONT_MATTER_KEY in frontMatter) {
		const document = cleanDocument(frontMatter[FRONT_MATTER_KEY]);

		const nodeList: Array<WeaveDocumentNode> = [];
		const identifierList: Array<ULID> = [];

		let node = document.nodes.get(document.currentNode);
		identifierList.push(document.currentNode);
		while (node) {
			nodeList.push(node);
			if (node.parentNode) {
				identifierList.push(node.parentNode);
				node = document.nodes.get(node.parentNode);
			} else {
				node = undefined;
			}
		}
		nodeList.reverse();
		identifierList.reverse();

		let offset = 0;

		for (const [i, node] of nodeList.entries()) {
			let nodeContent = "";

			if (typeof node.content == "string") {
				nodeContent = node.content;
			} else {
				for (const nodeToken of node.content) {
					nodeContent = nodeContent + nodeToken;
				}
			}

			if (
				content.length >= offset + nodeContent.length &&
				content.substring(offset, offset + nodeContent.length) ==
					nodeContent
			) {
				offset = offset + nodeContent.length;
			} else {
				const identifier = ulid();
				const nodeContent = content.substring(offset);

				document.nodes.set(identifier, {
					content: nodeContent,
					parentNode: node.parentNode,
				});

				document.currentNode = identifier;
				if (identifierList.length == i + 1) {
					document.nodes.delete(identifierList[i]);
				}
				offset = offset + nodeContent.length;
				saveDocument(editor, document);
				break;
			}
		}

		if (content.length > offset) {
			const identifier = ulid();
			const nodeContent = content.substring(offset);

			if (identifierList.length > 0) {
				const parentNodeIdentifier =
					identifierList[identifierList.length - 1];
				const parentNode = document.nodes.get(parentNodeIdentifier);
				if (parentNode) {
					if (parentNode.content.length > 0) {
						document.nodes.set(identifier, {
							content: nodeContent,
							parentNode: parentNodeIdentifier,
						});
					} else {
						document.nodes.set(identifier, {
							content: nodeContent,
							parentNode: parentNode.parentNode,
						});
						document.nodes.delete(parentNodeIdentifier);
					}
				} else {
					document.nodes.set(identifier, {
						content: nodeContent,
					});
				}
			} else {
				document.nodes.set(identifier, {
					content: nodeContent,
				});
			}
			document.currentNode = identifier;
			saveDocument(editor, document);
		}

		return document;
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
