import { getFrontMatterInfo, parseYaml, Editor, stringifyYaml } from "obsidian";
import serialize from "serialize-javascript";
import { deserialize } from "common";
import { ulid, ULID } from "ulid";
import { ModelLabel, UNKNOWN_MODEL_LABEL } from "client";

export class WeaveDocument {
	models: Map<ULID, ModelLabel> = new Map();
	protected modelNodes: Map<ULID, Set<ULID>>;
	protected nodes: Map<ULID, WeaveDocumentNode> = new Map();
	protected rootNodes: Set<ULID> = new Set();
	protected nodeChildren: Map<ULID, Set<ULID>> = new Map();
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
			content = getNodeContent(node) + content;
			if (node.parentNode) {
				node = this.nodes.get(node.parentNode);
			} else {
				node = undefined;
			}
		}

		return content;
	}
	setActiveContent(content: string) {
		let modified = false;

		const nodeList = this.getActiveNodes();

		let offset = 0;

		for (const [i, node] of nodeList.entries()) {
			const nodeContent = getNodeContent(node);

			// TODO: Combine nodes w/o children, combine duplicate nodes

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
					this.addNode({
						identifier: identifier,
						content: nodeContent,
						parentNode: node.parentNode,
					});

					this.currentNode = identifier;
				} else {
					this.currentNode = node.parentNode;
				}

				if (this.getNodeChildrenCount(nodeList[i].identifier) <= 1) {
					this.removeNode(nodeList[i].identifier);
				}

				offset = offset + nodeContent.length;
				modified = true;
				break;
			}
		}

		if (content.length > offset) {
			const identifier = ulid();
			const nodeContent = content.substring(offset);

			if (offset == 0) {
				if (nodeList.length > 0) {
					this.removeNode(nodeList[0].identifier);
				}
				this.addNode({
					identifier: identifier,
					content: nodeContent,
				});
			} else {
				this.addNode({
					identifier: identifier,
					content: nodeContent,
					parentNode: this.currentNode,
				});
			}
			this.currentNode = identifier;
			modified = true;
		}

		return modified;
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
	addNode(node: WeaveDocumentNode, model: ModelLabel = UNKNOWN_MODEL_LABEL) {
		if (node.parentNode) {
			const parentNode = this.nodes.get(node.parentNode);

			if (parentNode) {
				if (parentNode.content.length > 0 || !parentNode.parentNode) {
					this.nodes.set(node.identifier, node);
					const parentChildren = this.nodeChildren.get(
						node.parentNode
					);
					if (parentChildren) {
						parentChildren.add(node.identifier);
						this.nodeChildren.set(node.parentNode, parentChildren);
					} else {
						this.nodeChildren.set(
							node.parentNode,
							new Set([node.identifier])
						);
					}
				} else {
					node.parentNode = parentNode.parentNode;
					this.addNode(node);
				}
			} else {
				node.parentNode = undefined;
				this.nodes.set(node.identifier, node);
				this.rootNodes.add(node.identifier);
				this.nodeChildren.set(node.identifier, new Set());
			}
		} else {
			this.nodes.set(node.identifier, node);
			this.rootNodes.add(node.identifier);
			this.nodeChildren.set(node.identifier, new Set());
		}
		if (node.model) {
			const documentModel = this.models.get(node.model);

			if (!documentModel) {
				this.models.set(node.model, model);
			}

			const modelNodes = this.modelNodes.get(node.model);

			if (modelNodes) {
				modelNodes.add(node.identifier);
				this.modelNodes.set(node.model, modelNodes);
			} else {
				this.modelNodes.set(node.model, new Set([node.identifier]));
			}
		}
	}
	splitNode(identifier: ULID, index: number) {
		// TODO
	}
	getNodeChildrenCount(identifier: ULID) {
		const size = this.nodeChildren.get(identifier)?.size;

		if (size) {
			return size;
		} else {
			return 0;
		}
	}
	removeNode(identifier: ULID) {
		const node = this.nodes.get(identifier);
		if (node) {
			this.nodes.delete(identifier);
			this.rootNodes.delete(identifier);
			if (node.parentNode) {
				if (this.currentNode == identifier) {
					this.currentNode = node.parentNode;
				}
				const parentChildren = this.nodeChildren.get(node.parentNode);
				if (parentChildren) {
					parentChildren.delete(node.identifier);
					this.nodeChildren.set(node.parentNode, parentChildren);
				}
			}
			if (node.model) {
				const modelNodes = this.modelNodes.get(node.model);
				if (modelNodes) {
					modelNodes.delete(identifier);
					if (modelNodes.size == 0) {
						this.models.delete(node.model);
						this.modelNodes.delete(node.model);
					} else {
						this.modelNodes.set(node.model, modelNodes);
					}
				}
			}
			const childNodes = this.nodeChildren.get(identifier);
			if (childNodes) {
				for (const node of childNodes) {
					this.removeNode(node);
				}
				this.nodeChildren.delete(identifier);
			}
		}
	}
}

export interface WeaveDocumentNode {
	identifier: ULID;
	content: string | Array<[number, string]>;
	model?: ULID;
	parentNode?: ULID;
	metadata?: Map<string, string>;
}

function getNodeContent(node: WeaveDocumentNode) {
	let nodeContent = "";

	if (typeof node.content == "string") {
		nodeContent = node.content;
	} else {
		for (const nodeToken of node.content) {
			nodeContent = nodeContent + nodeToken;
		}
	}

	return nodeContent;
}

export const FRONT_MATTER_KEY = "TapestryLoomWeave";

export function loadDocument(editor: Editor) {
	const rawContent = editor.getValue();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	const frontMatter = parseYaml(frontMatterInfo.frontmatter);
	const content = rawContent.substring(frontMatterInfo.contentStart);

	if (frontMatterInfo.exists && FRONT_MATTER_KEY in frontMatter) {
		const document: WeaveDocument = Object.assign(
			new WeaveDocument(""),
			deserialize(frontMatter[FRONT_MATTER_KEY])
		);

		if (document.setActiveContent(content)) {
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

export function overrideEditorContent(editor: Editor, document: WeaveDocument) {
	// TODO
}
