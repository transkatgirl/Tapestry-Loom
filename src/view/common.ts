import {
	getFrontMatterInfo,
	debounce,
	parseYaml,
	Command,
	Editor,
	ItemView,
	EventRef,
	WorkspaceLeaf,
	setIcon,
} from "obsidian";
import TapestryLoom from "main";
import { getNodeContent, WeaveDocumentNode } from "document";
import { ULID, ulid } from "ulid";

export const VIEW_COMMANDS: Array<Command> = [];
