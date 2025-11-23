import { ulid, type ULID } from "ulid";

interface Node {
	id: ULID;
	from?: ULID;
	to: Array<ULID>;
	active: boolean;
	bookmarked: boolean;
	content: String | Array<NodeToken>;
	metadata: Map<String, String>;
	model?: Model;
}

interface NodeDeclaration {
	id?: ULID;
	from?: ULID;
	active: boolean;
	bookmarked: boolean;
	content: String | Array<NodeToken>;
	metadata: Map<String, String>;
	model?: Model;
}

interface NodeToken {
	content: String;
	metadata: Map<String, String>;
}

interface Model {
	label: String;
	metadata: Map<String, String>;
}

interface Weave {
	backend: WebSocket;
}
