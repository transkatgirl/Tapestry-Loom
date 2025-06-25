import { getFrontMatterInfo, parseYaml, Editor, stringifyYaml } from "obsidian";
import serialize from "serialize-javascript";
import { compress, decompress, deserialize } from "common";
import { decodeTime, ulid, ULID } from "ulid";
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
			this.currentNode = this.addNode({
				identifier: ulid(),
				content: content,
			});
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
	getActiveIdentifier(
		content: string,
		position: number
	): [ULID, number] | undefined {
		const nodeList = this.getActiveNodes();

		let offset = 0;

		for (const [_, node] of nodeList.entries()) {
			const nodeContent = getNodeContent(node);

			if (
				content.length >= offset + nodeContent.length &&
				content.substring(offset, offset + nodeContent.length) ==
					nodeContent
			) {
				if (position > offset + nodeContent.length) {
					offset = offset + nodeContent.length;
				} else {
					return [node.identifier, position - offset];
				}
			}
		}
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
					!this.bookmarks.has(nodeList[i].identifier) &&
					!node.model &&
					!node.parameters
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
	getAllNodes(): Array<WeaveDocumentNode> {
		const nodes: Array<WeaveDocumentNode> = [];
		for (const [_identifier, node] of this.nodes.entries()) {
			nodes.push(node);
		}
		nodes.sort(function (a, b) {
			return a.identifier.localeCompare(b.identifier);
		});

		return nodes;
	}
	getNodeChildren(identifier: ULID): Array<WeaveDocumentNode> {
		const childSet = this.nodeChildren.get(identifier);

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
					JSON.stringify(node.parameters) ==
						JSON.stringify(parentNode.parameters) &&
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
								JSON.stringify(child.parameters) ==
									JSON.stringify(node.parameters) &&
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

		this.currentNode = this.addNode(node);
	}
	addNode(node: WeaveDocumentNode, model?: ModelLabel): ULID {
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
						JSON.stringify(child.parameters) ==
							JSON.stringify(node.parameters)
					) {
						return child.identifier;
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
			if (model || !this.models.has(node.model)) {
				this.models.set(node.model, model || UNKNOWN_MODEL_LABEL);
			}

			const modelNodes = this.modelNodes.get(node.model);

			if (modelNodes) {
				modelNodes.add(node.identifier);
				this.modelNodes.set(node.model, modelNodes);
			} else {
				this.modelNodes.set(node.model, new Set([node.identifier]));
			}
		}
		return node.identifier;
	}
	getNode(identifier: ULID) {
		return this.nodes.get(identifier);
	}
	splitNode(identifier: ULID, index: number) {
		const node = this.nodes.get(identifier);

		if (node && index > 0 && getNodeContent(node).length > index) {
			if (typeof node.content == "string") {
				const splitContent = [
					node.content.slice(0, index),
					node.content.slice(index),
				];

				const primaryChildren = structuredClone(
					this.nodeChildren.get(node.identifier)
				);

				node.content = splitContent[0];
				const secondaryIdentifier = this.addNode({
					identifier: ulid(decodeTime(node.identifier)),
					content: splitContent[1],
					model: node.model,
					parentNode: structuredClone(node.identifier),
					parameters: structuredClone(node.parameters),
				});

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
			} else if (node) {
				const nodeContent = getNodeContent(node);
				const splitContent = [
					nodeContent.slice(0, index),
					nodeContent.slice(index),
				];
				const timestamp = decodeTime(node.identifier);

				const primaryIdentifier = this.addNode({
					identifier: ulid(timestamp),
					content: splitContent[0],
					model: node.model,
					parentNode: node.parentNode,
					parameters: structuredClone(node.parameters),
				});
				this.addNode({
					identifier: ulid(timestamp),
					content: splitContent[1],
					model: node.model,
					parentNode: primaryIdentifier,
					parameters: structuredClone(node.parameters),
				});

				if (this.currentNode == node.identifier) {
					this.currentNode = primaryIdentifier;
				}
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
				JSON.stringify(primaryNode.parameters) ==
					JSON.stringify(secondaryNode.parameters)
			);
		} else {
			return false;
		}
	}
	mergeNode(primaryIdentifier: ULID, secondaryIdentifier: ULID) {
		const primaryNode = this.nodes.get(primaryIdentifier);
		const secondaryNode = this.nodes.get(secondaryIdentifier);

		if (!primaryNode || !secondaryNode) {
			return;
		}

		if (this.isNodeMergeable(primaryIdentifier, secondaryIdentifier)) {
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
		} else {
			let content: string | [number, string][];

			if (
				typeof primaryNode.content == "string" &&
				typeof secondaryNode.content == "string"
			) {
				content = primaryNode.content + secondaryNode.content;
			} else if (
				typeof primaryNode.content == "object" &&
				Array.isArray(primaryNode.content) &&
				typeof secondaryNode.content == "object" &&
				Array.isArray(secondaryNode.content) &&
				primaryNode.model == secondaryNode.model
			) {
				content = primaryNode.content.concat(secondaryNode.content);
			} else {
				content =
					getNodeContent(primaryNode) + getNodeContent(secondaryNode);
			}

			let model;
			if (primaryNode.model == secondaryNode.model) {
				model = secondaryNode.model;
			}

			let parameters;
			if (
				JSON.stringify(primaryNode.parameters) ==
				JSON.stringify(secondaryNode.parameters || {})
			) {
				parameters = secondaryNode.parameters;
			}

			const identifier = this.addNode({
				identifier: ulid(decodeTime(secondaryNode.identifier)),
				content: content,
				model: model,
				parentNode: primaryNode.parentNode,
				parameters: structuredClone(parameters),
			});

			if (
				this.currentNode == primaryIdentifier ||
				this.currentNode == secondaryIdentifier
			) {
				this.currentNode = identifier;
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
			if (this.currentNode == identifier) {
				this.currentNode = node.parentNode;
			}
			if (node.parentNode) {
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
	parameters?: Record<string, string>;
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
				(b.content[0][0] as number) - (a.content[0][0] as number)
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

export const UNCOMPRESSED_FRONT_MATTER_KEY = "TapestryLoomWeave";
export const COMPRESSED_FRONT_MATTER_KEY = "TapestryLoomWeaveCompressed";

export async function loadDocument(editor: Editor, storeCompressed: boolean) {
	const rawContent = editor.getValue();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	const frontMatter = parseYaml(frontMatterInfo.frontmatter);
	const content = rawContent.substring(frontMatterInfo.contentStart);

	if (frontMatterInfo.exists && COMPRESSED_FRONT_MATTER_KEY in frontMatter) {
		const document: WeaveDocument = Object.assign(
			new WeaveDocument(),
			deserialize(
				await decompress(frontMatter[COMPRESSED_FRONT_MATTER_KEY])
			)
		);

		if (document.setActiveContent(content)) {
			await saveDocument(editor, document, storeCompressed);
		}

		return document;
	} else if (
		frontMatterInfo.exists &&
		UNCOMPRESSED_FRONT_MATTER_KEY in frontMatter
	) {
		const document: WeaveDocument = Object.assign(
			new WeaveDocument(),
			deserialize(frontMatter[UNCOMPRESSED_FRONT_MATTER_KEY])
		);

		if (document.setActiveContent(content)) {
			await saveDocument(editor, document, storeCompressed);
		}

		return document;
	} else {
		return new WeaveDocument(content);
	}
}

export async function updateDocument(
	editor: Editor,
	document: WeaveDocument,
	storeCompressed: boolean
) {
	const rawContent = editor.getValue();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	const content = rawContent.substring(frontMatterInfo.contentStart);

	const updated = document.setActiveContent(content);

	if (updated) {
		await saveDocument(editor, document, storeCompressed);
	}

	return updated;
}

export async function saveDocument(
	editor: Editor,
	document: WeaveDocument,
	storeCompressed: boolean
) {
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

	if (storeCompressed) {
		frontMatter[COMPRESSED_FRONT_MATTER_KEY] = await compress(
			serialize(document)
		);
		if (UNCOMPRESSED_FRONT_MATTER_KEY in frontMatter) {
			delete frontMatter[UNCOMPRESSED_FRONT_MATTER_KEY];
		}
	} else {
		frontMatter[UNCOMPRESSED_FRONT_MATTER_KEY] = serialize(document);
		if (COMPRESSED_FRONT_MATTER_KEY in frontMatter) {
			delete frontMatter[COMPRESSED_FRONT_MATTER_KEY];
		}
	}

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

export async function overrideEditorContent(
	editor: Editor,
	document: WeaveDocument,
	storeCompressed: boolean
) {
	const rawContent = editor.getValue();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	let frontMatter = parseYaml(frontMatterInfo.frontmatter);

	if (!frontMatter) {
		frontMatter = {};
	}

	if (storeCompressed) {
		frontMatter[COMPRESSED_FRONT_MATTER_KEY] = await compress(
			serialize(document)
		);
		if (UNCOMPRESSED_FRONT_MATTER_KEY in frontMatter) {
			delete frontMatter[UNCOMPRESSED_FRONT_MATTER_KEY];
		}
	} else {
		frontMatter[UNCOMPRESSED_FRONT_MATTER_KEY] = serialize(document);
		if (COMPRESSED_FRONT_MATTER_KEY in frontMatter) {
			delete frontMatter[COMPRESSED_FRONT_MATTER_KEY];
		}
	}

	editor.setValue(
		"---\n" +
			stringifyYaml(frontMatter) +
			"---\n" +
			document.getActiveContent()
	);
}
