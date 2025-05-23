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
import { getNodeContent, WeaveDocumentNode } from "document";
import { ULID, ulid } from "ulid";
import { generateNodeChildren } from "./common";

// TODO: Add editor commands for more functions, incorporate cursor position into editor commands

export function buildCommands(plugin: TapestryLoom): Array<Command> {
	return [
		{
			id: "run-tapestry-loom-completion",
			name: "Run completion",
			callback: () => {
				generateNodeChildren(
					this.plugin,
					this.plugin.document?.currentNode
				);
			},
		},
	];
}
