import { getFrontMatterInfo, parseYaml, Editor, stringifyYaml } from "obsidian";
import serialize from "serialize-javascript";
import { deserialize } from "common";
import { ulid, ULID } from "ulid";
import { ModelLabel, UNKNOWN_MODEL_LABEL } from "client";

export class WeaveDocument {
	identifier: ULID = ulid();
	models: Map<ULID, ModelLabel> = new Map();
	protected modelNodes: Map<ULID, Set<ULID>> = new Map();
	protected nodes: Map<ULID, WeaveDocumentNode> = new Map();
	protected rootNodes: Set<ULID> = new Set();
	protected nodeChildren: Map<ULID, Set<ULID>> = new Map();
	currentNode?: ULID;
	bookmarks: Set<ULID> = new Set();
	constructor(content?: string) {
		if (content) {
			const identifier = ulid();

			this.addNode({
				identifier: identifier,
				content: content,
			});
			this.currentNode = identifier;
		}
	}
	getActiveContent(identifier?: ULID): string {
		let content = "";

		let node: WeaveDocumentNode | undefined;
		if (identifier) {
			node = this.nodes.get(identifier);
		} else if (this.currentNode) {
			node = this.nodes.get(this.currentNode);
		}
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
	getActiveNodes(identifier?: ULID): Array<WeaveDocumentNode> {
		const nodeList = [];

		let node: WeaveDocumentNode | undefined;
		if (identifier) {
			node = this.nodes.get(identifier);
		} else if (this.currentNode) {
			node = this.nodes.get(this.currentNode);
		}

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
		for (const identifier of this.rootNodes) {
			const node = this.nodes.get(identifier);
			if (node) {
				nodes.push(node);
			}
		}
		sortNodeList(nodes);

		return nodes;
	}
	getNodeChildren(node: WeaveDocumentNode): Array<WeaveDocumentNode> {
		const childSet = this.nodeChildren.get(node.identifier);

		if (childSet) {
			const childNodes: Array<WeaveDocumentNode> = [];
			for (const identifier of childSet) {
				const node = this.nodes.get(identifier);
				if (node) {
					childNodes.push(node);
				}
			}
			sortNodeList(childNodes);

			return childNodes;
		} else {
			return [];
		}
	}
	private addCurrentNode(node: WeaveDocumentNode) {
		if (node.parentNode) {
			const parentNode = this.nodes.get(node.parentNode);
			if (parentNode) {
				if (
					this.getNodeChildrenCount(node.parentNode) == 0 &&
					node.model == parentNode.model &&
					node.metadata?.entries() ==
						parentNode.metadata?.entries() &&
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
									node.metadata?.entries() &&
								this.getNodeChildrenCount(child.identifier) == 0
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
				const parentNodeChildren =
					this.nodeChildren.get(node.parentNode) || new Set();

				for (const childIdentifier of parentNodeChildren) {
					const child = this.nodes.get(childIdentifier);

					if (
						child &&
						child.parentNode == node.parentNode &&
						JSON.stringify(child.content) ==
							JSON.stringify(node.content) &&
						child.model == node.model &&
						JSON.stringify(child.metadata?.entries()) ==
							JSON.stringify(node.metadata?.entries())
					) {
						return;
					}
				}

				if (getNodeContent(parentNode).length > 0) {
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
			this.models.set(node.model, model);

			const modelNodes = this.modelNodes.get(node.model);

			if (modelNodes) {
				modelNodes.add(node.identifier);
				this.modelNodes.set(node.model, modelNodes);
			} else {
				this.modelNodes.set(node.model, new Set([node.identifier]));
			}
		}
	}
	getNode(identifier: ULID) {
		return this.nodes.get(identifier);
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
	isNodeMergeable(primaryIdentifier: ULID, secondaryIdentifier: ULID) {
		const primaryNode = this.nodes.get(primaryIdentifier);
		const secondaryNode = this.nodes.get(secondaryIdentifier);

		if (primaryNode && secondaryNode) {
			return (
				secondaryNode.parentNode == primaryNode.identifier &&
				this.getNodeChildrenCount(primaryNode.identifier) == 1 &&
				primaryNode.model == secondaryNode.model &&
				((typeof primaryNode.content == "string" &&
					typeof secondaryNode.content == "string") ||
					(typeof primaryNode.content == "object" &&
						Array.isArray(primaryNode.content) &&
						typeof secondaryNode.content == "object" &&
						Array.isArray(secondaryNode.content))) &&
				primaryNode.metadata?.entries() ==
					secondaryNode.metadata?.entries()
			);
		} else {
			return false;
		}
	}
	mergeNode(primaryIdentifier: ULID, secondaryIdentifier: ULID) {
		if (this.isNodeMergeable(primaryIdentifier, secondaryIdentifier)) {
			const primaryNode = this.nodes.get(primaryIdentifier);
			const secondaryNode = this.nodes.get(secondaryIdentifier);

			if (!primaryNode || !secondaryNode) {
				return;
			}

			if (
				typeof primaryNode.content == "string" &&
				typeof secondaryNode.content == "string"
			) {
				secondaryNode.content =
					primaryNode.content + secondaryNode.content;
			} else if (
				typeof primaryNode.content == "object" &&
				Array.isArray(primaryNode.content) &&
				typeof secondaryNode.content == "object" &&
				Array.isArray(secondaryNode.content)
			) {
				secondaryNode.content = primaryNode.content.concat(
					secondaryNode.content
				);
			} else {
				return;
			}
			secondaryNode.parentNode = primaryNode.parentNode;
			if (primaryNode.parentNode) {
				let parentNodeChildren = this.nodeChildren.get(
					primaryNode.parentNode
				);
				if (!parentNodeChildren) {
					parentNodeChildren = new Set();
				}
				parentNodeChildren.add(secondaryNode.identifier);
				this.nodeChildren.set(
					primaryNode.parentNode,
					parentNodeChildren
				);
			} else {
				this.rootNodes.add(secondaryNode.identifier);
			}

			this.nodeChildren.set(primaryNode.identifier, new Set());

			if (this.bookmarks.has(primaryNode.identifier)) {
				this.bookmarks.add(secondaryNode.identifier);
			}

			if (this.currentNode == primaryNode.identifier) {
				this.currentNode = secondaryNode.identifier;
			}

			this.removeNode(primaryNode.identifier);
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
		for (const [_nodeProb, nodeToken] of node.content) {
			nodeContent = nodeContent + nodeToken;
		}
	}

	return nodeContent;
}

function sortNodeList(nodes: Array<WeaveDocumentNode>) {
	nodes.sort(function (a, b) {
		const x =
			typeof a.content == "object" &&
			Array.isArray(a.content) &&
			a.content.length == 1;
		const y =
			typeof b.content == "object" &&
			Array.isArray(b.content) &&
			b.content.length == 1;

		if (x && y) {
			return (
				(a.model || "")?.localeCompare(b.model || "") ||
				(a.content[0][0] as number) - (b.content[0][0] as number)
			);
		} else {
			return (
				(a.model || "")?.localeCompare(b.model || "") ||
				(x === y ? 0 : x ? 1 : -1) ||
				a.identifier.localeCompare(b.identifier)
			);
		}
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
			new WeaveDocument(),
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
	let frontMatter = parseYaml(frontMatterInfo.frontmatter);
	const content = rawContent.substring(frontMatterInfo.contentStart);

	if (document.getActiveContent().trim() != content.trim()) {
		return;
	}

	if (!frontMatter) {
		frontMatter = {};
	}

	frontMatter[FRONT_MATTER_KEY] = serialize(document);

	if (frontMatterInfo.exists) {
		editor.replaceRange(
			stringifyYaml(frontMatter),
			editor.offsetToPos(frontMatterInfo.from),
			editor.offsetToPos(frontMatterInfo.to)
		);
	} else {
		editor.setValue(
			"---\n" + stringifyYaml(frontMatter) + "---\n" + rawContent
		);
	}
}

export function overrideEditorContent(editor: Editor, document: WeaveDocument) {
	const rawContent = editor.getValue();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	let frontMatter = parseYaml(frontMatterInfo.frontmatter);

	if (!frontMatter) {
		frontMatter = {};
	}

	frontMatter[FRONT_MATTER_KEY] = serialize(document);

	editor.setValue(
		"---\n" +
			stringifyYaml(frontMatter) +
			"---\n" +
			document.getActiveContent()
	);
}
