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

export function buildCommands(plugin: TapestryLoom): Array<Command> {
	return [];
}
