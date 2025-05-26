import TapestryLoom from "main";
import { Command } from "obsidian";
import { getNodeContent } from "document";
import {
	addNode,
	addNodeSibling,
	deleteNode,
	deleteNodeChildren,
	deleteNodeSiblings,
	generateNodeChildren,
	getEditorContent,
	getEditorOffset,
	mergeNode,
	moveToChild,
	moveToNextSibling,
	moveToParent,
	moveToPreviousSibling,
	toggleBookmarkNode,
} from "./common";

export function buildCommands(plugin: TapestryLoom): Array<Command> {
	return [
		{
			id: "node-tapestry-loom-generate-completion-current",
			name: "Generate completions at current node",
			callback: async () => {
				await runCompletion(plugin, false);
			},
		},
		{
			id: "node-tapestry-loom-generate-completion-cursor",
			name: "Generate completions at current position",
			callback: async () => {
				await runCompletion(plugin, true);
			},
		},
		{
			id: "node-tapestry-loom-add-child",
			name: "Add child node to current node",
			callback: () => {
				const identifier = plugin.document?.currentNode;
				addNode(plugin, identifier);
			},
		},
		{
			id: "node-tapestry-loom-add-sibling",
			name: "Add sibling node to current node",
			callback: () => {
				const identifier = plugin.document?.currentNode;
				addNodeSibling(plugin, identifier);
			},
		},
		{
			id: "node-tapestry-loom-toggle-bookmarked",
			name: "Toggle whether current node is bookmarked",
			callback: () => {
				const identifier = plugin.document?.currentNode;
				if (identifier) {
					toggleBookmarkNode(plugin, identifier);
				}
			},
		},
		{
			id: "node-tapestry-loom-toggle-bookmarked",
			name: "Toggle whether node at current position is bookmarked",
			callback: () => {
				const identifier = getCursorNode(plugin);
				if (identifier) {
					toggleBookmarkNode(plugin, identifier);
				}
			},
		},
		{
			id: "node-tapestry-loom-split-current",
			name: "Split node at current position",
			callback: () => {
				splitNode(plugin);
			},
		},
		{
			id: "node-tapestry-loom-merge-with-parent",
			name: "Merge current node with parent",
			callback: () => {
				const identifier = plugin.document?.currentNode;
				if (identifier) {
					mergeNode(plugin, identifier);
				}
			},
		},
		{
			id: "node-tapestry-loom-delete-current",
			name: "Delete current node",
			callback: () => {
				const identifier = plugin.document?.currentNode;
				if (identifier) {
					deleteNode(plugin, identifier);
				}
			},
		},
		{
			id: "node-tapestry-loom-delete-children",
			name: "Delete all children of current node",
			callback: () => {
				const identifier = plugin.document?.currentNode;
				if (identifier) {
					deleteNodeChildren(plugin, identifier);
				}
			},
		},
		{
			id: "node-tapestry-loom-delete-other-siblings",
			name: "Delete other siblings of current node",
			callback: () => {
				const identifier = plugin.document?.currentNode;
				if (identifier) {
					deleteNodeSiblings(plugin, identifier, true);
				}
			},
		},
		{
			id: "node-tapestry-loom-delete-all-siblings",
			name: "Delete all siblings of current node",
			callback: () => {
				const identifier = plugin.document?.currentNode;
				if (identifier) {
					deleteNodeSiblings(plugin, identifier, false);
				}
			},
		},
		{
			id: "node-tapestry-loom-move-to-parent",
			name: "Move to parent node",
			callback: () => {
				moveToParent(plugin);
			},
		},
		{
			id: "node-tapestry-loom-move-to-child",
			name: "Move to child node",
			callback: () => {
				moveToChild(plugin);
			},
		},
		{
			id: "node-tapestry-loom-move-to-next-sibling",
			name: "Move to next sibling node",
			callback: () => {
				moveToNextSibling(plugin);
			},
		},
		{
			id: "node-tapestry-loom-move-to-previous-sibling",
			name: "Move to previous sibling node",
			callback: () => {
				moveToPreviousSibling(plugin);
			},
		},
	];
}

async function runCompletion(plugin: TapestryLoom, useOffset: boolean) {
	if (!plugin.document || !plugin.editor) {
		return;
	}

	const offset = getEditorOffset(plugin.editor);
	if (offset && useOffset) {
		const active = plugin.document.getActiveIdentifier(
			getEditorContent(plugin.editor),
			offset
		);
		if (active) {
			const node = plugin.document.getNode(active[0]);
			if (node && getNodeContent(node).length > active[1]) {
				plugin.document.splitNode(active[0], active[1]);
			}

			await generateNodeChildren(plugin, active[0]);
		} else {
			await generateNodeChildren(plugin, plugin.document?.currentNode);
		}
	} else {
		await generateNodeChildren(plugin, plugin.document?.currentNode);
	}
}

function splitNode(plugin: TapestryLoom) {
	if (!plugin.document || !plugin.editor) {
		return;
	}

	const offset = getEditorOffset(plugin.editor);
	if (offset) {
		const active = plugin.document.getActiveIdentifier(
			getEditorContent(plugin.editor),
			offset
		);
		if (active) {
			const node = plugin.document.getNode(active[0]);
			if (node && getNodeContent(node).length > active[1]) {
				plugin.document.splitNode(active[0], active[1]);
			}
		}
	}
}

function getCursorNode(plugin: TapestryLoom) {
	if (!plugin.document || !plugin.editor) {
		return;
	}

	const offset = getEditorOffset(plugin.editor);
	if (offset) {
		const active = plugin.document.getActiveIdentifier(
			getEditorContent(plugin.editor),
			offset
		);
		if (active) {
			return active[0];
		} else {
			return plugin.document.currentNode;
		}
	} else {
		return plugin.document.currentNode;
	}
}
