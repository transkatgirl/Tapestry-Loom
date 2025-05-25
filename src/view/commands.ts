import TapestryLoom from "main";
import {
	App,
	Command,
	ItemView,
	Menu,
	Modal,
	Setting,
	WorkspaceLeaf,
	setIcon,
} from "obsidian";
import { WeaveDocumentNode } from "document";
import { ULID, ulid } from "ulid";
import {
	generateNodeChildren,
	getEditorContent,
	getEditorOffset,
} from "./common";

// TODO: Add editor commands for more functions, incorporate cursor position into editor commands

export function buildCommands(plugin: TapestryLoom): Array<Command> {
	return [
		{
			id: "run-tapestry-loom-completion",
			name: "Run completion at current node",
			callback: async () => {
				await runCompletion(plugin, false);
			},
		},
		{
			id: "run-tapestry-loom-completion-precise",
			name: "Run completion at current position",
			callback: async () => {
				await runCompletion(plugin, true);
			},
		},
	];
}

async function runCompletion(plugin: TapestryLoom, split: boolean) {
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
			if (
				node &&
				split &&
				plugin.document.getNodeContent(node).length > active[1]
			) {
				plugin.document.splitNode(active[0], active[1]);
			} else {
				await generateNodeChildren(plugin, active[0]);
			}
		}
	} else {
		await generateNodeChildren(plugin, plugin.document?.currentNode);
	}
}
