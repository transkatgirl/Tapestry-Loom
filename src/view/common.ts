import TapestryLoom, { DOCUMENT_TRIGGER_UPDATE_EVENT } from "main";
import {
	App,
	Editor,
	FuzzySuggestModal,
	getFrontMatterInfo,
	Notice,
} from "obsidian";
import { ULID, ulid } from "ulid";
import { runCompletion } from "client";
import { DEFAULT_DOCUMENT_SETTINGS } from "settings";
import { getNodeContent, WeaveDocumentNode } from "document";

let activeRequests = 0;

function updateStatusBar(plugin: TapestryLoom) {
	if (activeRequests > 0) {
		plugin.statusBar.innerText = activeRequests + " requests";
	} else {
		plugin.statusBar.innerText = "";
	}
}

export async function generateNodeChildren(
	plugin: TapestryLoom,
	parentNode?: ULID
) {
	if (!plugin.document || !plugin.settings.client) {
		return;
	}

	const completionPromises = runCompletion(
		plugin.settings.client,
		plugin.sessionSettings.models,
		{
			prompt: plugin.document.getActiveContent(parentNode),
			count: plugin.sessionSettings.requests,
			parameters: plugin.sessionSettings.parameters,
		}
	);

	activeRequests = activeRequests + completionPromises.length;
	updateStatusBar(plugin);

	const debounceTime =
		plugin.settings.document?.debounce ||
		DEFAULT_DOCUMENT_SETTINGS.debounce;

	let lastUpdate = performance.now();

	for (const completionPromise of completionPromises) {
		completionPromise
			.then((completions) => {
				if (!plugin.document) {
					return;
				}

				for (const completion of completions) {
					if (completion.topProbs && completion.topProbs.length > 1) {
						for (const prob of completion.topProbs) {
							plugin.document.addNode(
								{
									identifier: ulid(),
									content: [prob],
									model: completion.model.ulid,
									parentNode: parentNode,
								},
								completion.model.label
							);
						}
					}

					if (
						typeof completion.completion == "string" ||
						!completion.topProbs ||
						completion.completion.length > 1
					) {
						plugin.document.addNode(
							{
								identifier: ulid(),
								content: completion.completion,
								model: completion.model.ulid,
								parentNode: parentNode,
								metadata: {
									parameters: JSON.stringify(
										plugin.sessionSettings.parameters
									),
								},
							},
							completion.model.label
						);
					}
				}

				const currentTimestamp = performance.now();

				if (currentTimestamp - lastUpdate > debounceTime) {
					plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
					lastUpdate = currentTimestamp;
				}
			})
			.catch((error) => {
				new Notice(error);
			})
			.finally(() => {
				activeRequests = activeRequests - 1;
				updateStatusBar(plugin);
			});
	}

	await Promise.all(completionPromises);

	updateStatusBar(plugin);
	plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
}

export function addNode(plugin: TapestryLoom, parentNode?: ULID) {
	if (!plugin.document) {
		return;
	}

	const identifier = ulid();
	plugin.document.addNode({
		identifier: identifier,
		content: "",
		parentNode: parentNode,
	});
	plugin.document.currentNode = identifier;
	plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
}

export function addNodeSibling(plugin: TapestryLoom, targetNode?: ULID) {
	if (!plugin.document) {
		return;
	}

	let parentNode;
	if (targetNode) {
		const node = plugin.document.getNode(targetNode);
		parentNode = node?.parentNode;
	}

	const identifier = ulid();
	plugin.document.addNode({
		identifier: identifier,
		content: "",
		parentNode: parentNode,
	});
	plugin.document.currentNode = identifier;
	plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
}

export function switchToNode(plugin: TapestryLoom, identifier: ULID) {
	if (!plugin.document) {
		return;
	}

	plugin.document.currentNode = identifier;
	plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
}

export function toggleBookmarkNode(plugin: TapestryLoom, identifier: ULID) {
	if (!plugin.document) {
		return;
	}

	if (plugin.document.bookmarks.has(identifier)) {
		plugin.document.bookmarks.delete(identifier);
	} else {
		plugin.document.bookmarks.add(identifier);
	}

	plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
}

export function mergeNode(plugin: TapestryLoom, childIdentifier: ULID) {
	if (!plugin.document) {
		return;
	}

	const childNode = plugin.document.getNode(childIdentifier);
	if (childNode?.parentNode) {
		plugin.document.mergeNode(childNode?.parentNode, childIdentifier);
	}

	plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
}

export function deleteNode(plugin: TapestryLoom, identifier: ULID) {
	if (!plugin.document) {
		return;
	}

	plugin.document.removeNode(identifier);
	plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
}

export function deleteNodeChildren(plugin: TapestryLoom, identifier: ULID) {
	if (!plugin.document) {
		return;
	}

	const children = plugin.document.getNodeChildren(identifier);
	for (const node of children) {
		plugin.document.removeNode(node.identifier);
	}

	plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
}

export function deleteNodeSiblings(
	plugin: TapestryLoom,
	identifier: ULID,
	excludeTarget: boolean
) {
	if (!plugin.document) {
		return;
	}

	const node = plugin.document.getNode(identifier);

	if (!node?.parentNode) {
		return;
	}

	const siblings = plugin.document.getNodeChildren(node.parentNode);
	for (const node of siblings) {
		if (node.identifier != identifier || !excludeTarget) {
			plugin.document.removeNode(node.identifier);
		}
	}

	plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
}

export function getEditorContent(editor: Editor) {
	const rawContent = editor.getValue();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	const content = rawContent.substring(frontMatterInfo.contentStart);

	return content;
}

export function getEditorOffset(editor: Editor) {
	const rawContent = editor.getValue();
	const frontMatterInfo = getFrontMatterInfo(rawContent);
	const offset =
		editor.posToOffset(editor.getCursor("head")) -
		frontMatterInfo.contentStart;

	if (offset >= 0) {
		return offset;
	}
}

export function moveToParent(plugin: TapestryLoom) {
	if (!plugin.document || !plugin.document.currentNode) {
		return;
	}

	const node = plugin.document.getNode(plugin.document.currentNode);
	plugin.document.currentNode = node?.parentNode;

	plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
}

export function moveToChild(plugin: TapestryLoom) {
	if (!plugin.document || !plugin.document.currentNode) {
		return;
	}

	const children = plugin.document.getNodeChildren(
		plugin.document.currentNode
	);
	if (children.length > 0) {
		plugin.document.currentNode = children[0].identifier;

		plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
	}
}

export function moveToPreviousSibling(plugin: TapestryLoom) {
	if (!plugin.document || !plugin.document.currentNode) {
		return;
	}

	const node = plugin.document.getNode(plugin.document.currentNode);
	if (!node || !node.parentNode) {
		return;
	}

	const siblings = plugin.document.getNodeChildren(node.parentNode);
	const index = siblings.indexOf(node);
	if (index > 0) {
		plugin.document.currentNode = siblings[index - 1].identifier;

		plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
	}
}

export function moveToNextSibling(plugin: TapestryLoom) {
	if (!plugin.document || !plugin.document.currentNode) {
		return;
	}

	const node = plugin.document.getNode(plugin.document.currentNode);
	if (!node || !node.parentNode) {
		return;
	}

	const siblings = plugin.document.getNodeChildren(node.parentNode);
	const index = siblings.indexOf(node);
	if (index < siblings.length - 1) {
		plugin.document.currentNode = siblings[index + 1].identifier;

		plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
	}
}

export class WeaveSearchModal extends FuzzySuggestModal<WeaveDocumentNode> {
	plugin: TapestryLoom;
	constructor(app: App, plugin: TapestryLoom) {
		super(app);
		this.plugin = plugin;
	}
	getItems(): WeaveDocumentNode[] {
		return this.plugin.document?.getAllNodes() || [];
	}
	getItemText(node: WeaveDocumentNode): string {
		return getNodeContent(node).trim();
	}
	onChooseItem(node: WeaveDocumentNode, _evt: MouseEvent | KeyboardEvent) {
		if (this.plugin.document) {
			this.plugin.document.currentNode = node.identifier;
			this.plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
		}
	}
}
