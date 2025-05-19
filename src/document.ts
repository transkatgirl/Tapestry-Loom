import { getFrontMatterInfo, parseYaml, Editor, stringifyYaml } from "obsidian";
import serialize from "serialize-javascript";
import { deserialize } from "common";
import { ulid, ULID } from "ulid";
import { ModelLabel, UNKNOWN_MODEL_LABEL } from "client";

export class WeaveDocument {
	models: Map<ULID, ModelLabel> = new Map();
	protected modelNodes: Map<ULID, Set<ULID>> = new Map();
	protected nodes: Map<ULID, WeaveDocumentNode> = new Map();
	protected rootNodes: Set<ULID> = new Set();
	protected nodeChildren: Map<ULID, Set<ULID>> = new Map();
	currentNode: ULID;
	bookmarks: Set<ULID> = new Set();
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

			if (
				content.length >= offset + nodeContent.length &&
				content.substring(offset, offset + nodeContent.length) ==
					nodeContent
			) {
				offset = offset + nodeContent.length;
			} else {
				const identifier = ulid();
				const nodeContent = content.substring(offset);

				this.addCurrentNode({
					identifier: identifier,
					content: nodeContent,
					parentNode: node.parentNode,
				});

				if (
					this.getNodeChildrenCount(nodeList[i].identifier) <= 1 &&
					!this.bookmarks.has(nodeList[i].identifier)
				) {
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

			this.addCurrentNode({
				identifier: identifier,
				content: nodeContent,
				parentNode: this.currentNode,
			});
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
	getRootNodes(): Array<WeaveDocumentNode> {
		const nodes: Array<WeaveDocumentNode> = [];
		for (const identifier of sortIdentifierSet(this.rootNodes)) {
			const node = this.nodes.get(identifier);
			if (node) {
				nodes.push(node);
			}
		}

		return nodes;
	}
	getNodeChildren(node: WeaveDocumentNode): Array<WeaveDocumentNode> {
		const childSet = this.nodeChildren.get(node.identifier);

		if (childSet) {
			const childNodes: Array<WeaveDocumentNode> = [];
			for (const identifier of sortIdentifierSet(childSet)) {
				const node = this.nodes.get(identifier);
				if (node) {
					childNodes.push(node);
				}
			}

			return childNodes;
		} else {
			return [];
		}
	}
	private addCurrentNode(node: WeaveDocumentNode) {
		if (node.parentNode) {
			const parentNode = this.nodes.get(node.parentNode);
			if (parentNode) {
				if (node.content.length == 0) {
					this.currentNode = node.parentNode;
					return;
				}
				if (
					this.getNodeChildrenCount(node.parentNode) == 0 &&
					!this.bookmarks.has(node.parentNode)
				) {
					node.content =
						getNodeContent(parentNode) + getNodeContent(node);
					node.parentNode = parentNode.parentNode;
					this.removeNode(parentNode.identifier);
				} else {
					const parentNodeChildren = this.nodeChildren.get(
						node.parentNode
					);
					if (parentNodeChildren) {
						for (const childIdentifier of parentNodeChildren) {
							const child = this.nodes.get(childIdentifier);

							if (
								child &&
								child.parentNode == node.parentNode &&
								child.content == node.content &&
								child.model == node.model &&
								child.metadata?.entries() ==
									node.metadata?.entries()
							) {
								this.currentNode = child.identifier;
								return;
							}
						}
					}
				}
			}
		}

		this.addNode(node);
		this.currentNode = node.identifier;
	}
	addNode(node: WeaveDocumentNode, model: ModelLabel = UNKNOWN_MODEL_LABEL) {
		if (node.parentNode) {
			const parentNode = this.nodes.get(node.parentNode);

			if (parentNode) {
				if (parentNode.content.length > 0) {
					this.nodes.set(node.identifier, node);
					this.nodeChildren.set(node.identifier, new Set());
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
					if (this.getNodeChildrenCount(parentNode.identifier) == 0) {
						this.removeNode(parentNode.identifier);
					}
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
		const node = this.nodes.get(identifier);

		if (node && index > 0 && index < node.content.length) {
			const splitContent = [
				node.content.slice(0, index),
				node.content.slice(index),
			];

			const secondaryIdentifier = ulid();

			node.content = splitContent[0];
			this.addNode({
				identifier: secondaryIdentifier,
				content: splitContent[1],
				model: node.model,
				parentNode: node.identifier,
				metadata: node.metadata,
			});

			const primaryChildren = this.nodeChildren.get(node.identifier);

			if (primaryChildren) {
				for (const childIdentifier of primaryChildren) {
					const childNode = this.nodes.get(childIdentifier);

					if (childNode) {
						childNode.parentNode = secondaryIdentifier;
					}
				}
				this.nodeChildren.set(secondaryIdentifier, primaryChildren);
				this.nodeChildren.set(
					node.identifier,
					new Set([secondaryIdentifier])
				);
			}
		}
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
			this.bookmarks.delete(identifier);
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

export function getNodeContent(node: WeaveDocumentNode) {
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

export function sortIdentifierSet(set: Set<ULID>): Array<ULID> {
	return Array.from(set).sort(function (a, b) {
		return a.localeCompare(b);
	});
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

export function updateDocument(editor: Editor, document: WeaveDocument) {
	const rawContent = editor.getValue();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	const content = rawContent.substring(frontMatterInfo.contentStart);

	const updated = document.setActiveContent(content);

	if (updated) {
		saveDocument(editor, document);
	}

	return updated;
}

export function saveDocument(editor: Editor, document: WeaveDocument) {
	const rawContent = editor.getValue();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	const frontMatter = parseYaml(frontMatterInfo.frontmatter);
	const content = rawContent.substring(frontMatterInfo.contentStart);

	if (document.getActiveContent().trim() != content.trim()) {
		return;
	}

	if (frontMatterInfo.exists) {
		frontMatter[FRONT_MATTER_KEY] = serialize(document);
		editor.replaceRange(
			stringifyYaml(frontMatter),
			editor.offsetToPos(frontMatterInfo.from),
			editor.offsetToPos(frontMatterInfo.to)
		);
	} else {
		editor.setValue(
			"---\n" +
				stringifyYaml({
					FRONT_MATTER_KEY: serialize(document),
				}) +
				"\n---\n" +
				rawContent
		);
	}
}

export function overrideEditorContent(editor: Editor, document: WeaveDocument) {
	const rawContent = editor.getValue();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	const frontMatter = parseYaml(frontMatterInfo.frontmatter);

	if (frontMatterInfo.exists) {
		frontMatter[FRONT_MATTER_KEY] = serialize(document);
		editor.setValue(
			"---\n" +
				stringifyYaml(frontMatter) +
				"\n---\n" +
				document.getActiveContent()
		);
	} else {
		editor.setValue(
			"---\n" +
				stringifyYaml({
					FRONT_MATTER_KEY: serialize(document),
				}) +
				"\n---\n" +
				document.getActiveContent()
		);
	}
}
