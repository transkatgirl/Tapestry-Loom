import { getFrontMatterInfo, parseYaml, Editor, stringifyYaml } from "obsidian";
import serialize from "serialize-javascript";
import { deserialize } from "common";
import { ulid, ULID } from "ulid";
import { ModelLabel, UNKNOWN_MODEL_LABEL } from "client";

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

export function loadDocument(editor: Editor) {
	const rawContent = editor.getValue();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	const frontMatter = parseYaml(frontMatterInfo.frontmatter);
	const content = rawContent.substring(frontMatterInfo.contentStart);

	if (frontMatterInfo.exists && FRONT_MATTER_KEY in frontMatter) {
		const document = deserialize(frontMatter[FRONT_MATTER_KEY]);

		if (updateDocument(document, content)) {
			saveDocument(editor, document);
		}

		return document;
	} else {
		return newDocument(content);
	}
}

export function saveDocument(editor: Editor, document: WeaveDocument) {
	const rawContent = editor.getValue();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	const frontMatter = parseYaml(frontMatterInfo.frontmatter);

	if (frontMatterInfo.exists) {
		frontMatter[FRONT_MATTER_KEY] = serialize(document, { space: "\t" });
		editor.replaceRange(
			stringifyYaml(frontMatter),
			editor.offsetToPos(frontMatterInfo.from),
			editor.offsetToPos(frontMatterInfo.to)
		);
	} else {
		const newContent =
			"---\n" +
			stringifyYaml({
				FRONT_MATTER_KEY: serialize(document, { space: "\t" }),
			}) +
			"\n---\n" +
			rawContent;
		editor.setValue(newContent);
	}
}

export function overrideEditorContent(
	editor: Editor,
	document: WeaveDocument
) {}

export function getContent(document: WeaveDocument): string {
	let content = "";

	let node = document.nodes.get(document.currentNode);
	while (node) {
		content = node.content + content;
		if (node.parentNode) {
			node = document.nodes.get(node.parentNode);
		} else {
			node = undefined;
		}
	}

	return content;
}

function updateDocument(document: WeaveDocument, content: string) {
	let modified = false;

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

			if (nodeContent.length > 0 || !node.parentNode) {
				document.nodes.set(identifier, {
					content: nodeContent,
					parentNode: node.parentNode,
				});

				document.currentNode = identifier;
			} else {
				document.currentNode = node.parentNode;
			}

			if (identifierList.length == i + 1) {
				deleteNode(document, identifierList[i]);
			}
			offset = offset + nodeContent.length;
			modified = true;
			break;
		}
	}

	if (content.length > offset) {
		const identifier = ulid();
		const nodeContent = content.substring(offset);

		if (identifierList.length > 0) {
			appendNode(document, identifier, {
				content: nodeContent,
				parentNode: identifierList[identifierList.length - 1],
			});
		} else {
			appendNode(document, identifier, {
				content: nodeContent,
			});
		}
		document.currentNode = identifier;
		modified = true;
	}

	if (modified) {
		pruneDocument(document);
	}

	return modified;
}

function pruneDocument(document: WeaveDocument) {
	// TODO: Prune orphaned nodes; only prune root nodes if they do not have children
	// TODO: Prune duplicate nodes, combine nodes w/o children
}

export function newDocument(content: string): WeaveDocument {
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

export function appendNode(
	document: WeaveDocument,
	identifier: ULID,
	node: WeaveDocumentNode
) {
	if (node.parentNode) {
		const parentNode = document.nodes.get(node.parentNode);

		if (parentNode) {
			if (parentNode.content.length > 0 || !parentNode.parentNode) {
				document.nodes.set(identifier, node);
			} else {
				node.parentNode = parentNode.parentNode;
				appendNode(document, identifier, node);
			}
		} else {
			node.parentNode = undefined;
			document.nodes.set(identifier, node);
		}
	} else {
		document.nodes.set(identifier, node);
	}
	if (node.model) {
		const model = document.models.get(node.model);

		if (!model) {
			document.models.set(node.model, UNKNOWN_MODEL_LABEL);
		}
	}
}

export function deleteNode(document: WeaveDocument, identifier: ULID) {
	document.nodes.delete(identifier);
}
