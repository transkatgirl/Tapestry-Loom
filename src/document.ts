import { getFrontMatterInfo, parseYaml, Editor, stringifyYaml } from "obsidian";
import serialize from "serialize-javascript";
import { deserialize } from "common";
import { ulid, ULID } from "ulid";
import { ModelLabel, UNKNOWN_MODEL_LABEL } from "client";

class WeaveDocument {
	models: Map<ULID, ModelLabel> = new Map();
	protected nodes: Map<ULID, WeaveDocumentNode> = new Map();
	currentNode: ULID;

	constructor(content: string) {
		const identifier = ulid();

		this.nodes.set(identifier, {
			identifier: identifier,
			content: content,
		});
		this.currentNode = identifier;
	}
	getActiveContent(): string {
		let content = "";

		let node = this.nodes.get(this.currentNode);
		while (node) {
			content = node.content + content;
			if (node.parentNode) {
				node = this.nodes.get(node.parentNode);
			} else {
				node = undefined;
			}
		}

		return content;
	}
	getActiveNodes(): Array<WeaveDocumentNode> {
		const nodeList = [];

		let node = this.nodes.get(this.currentNode);

		while (node) {
			nodeList.push(node);
			if (node.parentNode) {
				node = this.nodes.get(node.parentNode);
			} else {
				node = undefined;
			}
		}
		nodeList.reverse();

		return nodeList;
	}
	getNodeTree() {
		// TODO
	}
	private pruneNodeTree() {
		// TODO: Prune orphaned nodes; only prune root nodes if they do not have children
		// TODO: Prune duplicate nodes, combine nodes w/o children
	}
	addNode(node: WeaveDocumentNode) {
		if (node.parentNode) {
			const parentNode = this.nodes.get(node.parentNode);

			if (parentNode) {
				if (parentNode.content.length > 0 || !parentNode.parentNode) {
					this.nodes.set(node.identifier, node);
				} else {
					node.parentNode = parentNode.parentNode;
					this.addNode(node);
				}
			} else {
				node.parentNode = undefined;
				this.nodes.set(node.identifier, node);
			}
		} else {
			this.nodes.set(node.identifier, node);
		}
		if (node.model) {
			const model = this.models.get(node.model);

			if (!model) {
				this.models.set(node.model, UNKNOWN_MODEL_LABEL);
			}
		}
	}
	removeNode(identifier: ULID) {
		this.nodes.delete(identifier);
	}
}

export interface WeaveDocumentNode {
	identifier: ULID;
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
		return new WeaveDocument(content);
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

function updateDocument(document: WeaveDocument, content: string) {
	let modified = false;

	const nodeList = document.getActiveNodes();

	let offset = 0;

	for (const node of nodeList) {
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
				document.addNode({
					identifier: identifier,
					content: nodeContent,
					parentNode: node.parentNode,
				});

				document.currentNode = identifier;
			} else {
				document.currentNode = node.parentNode;
			}

			offset = offset + nodeContent.length;
			modified = true;
			break;
		}
	}

	if (content.length > offset) {
		const identifier = ulid();
		const nodeContent = content.substring(offset);

		if (nodeList.length > 1) {
			document.addNode({
				identifier: identifier,
				content: nodeContent,
				parentNode: nodeList[nodeList.length - 1].identifier,
			});
		} else {
			document.addNode({
				identifier: identifier,
				content: nodeContent,
			});
		}
		document.currentNode = identifier;
		modified = true;
	}

	return modified;
}
