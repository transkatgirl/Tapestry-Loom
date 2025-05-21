import { HexString, requestUrl, RequestUrlParam } from "obsidian";
import { ULID } from "ulid";

export interface ClientSettings {
	clientIdentifier?: string;
	models: Array<ModelConfiguration>;
}

export enum EndpointType {
	OpenAICompletionv1Compatible = "openai_completion_v1_compatible",
}

export interface ModelConfiguration {
	baseUrl: string;
	type: EndpointType;
	apiToken?: string;
	modelIdentifier: string;
	label: ModelLabel;
	customHeaders?: Map<string, string>;
	customValues?: Map<string, string>;
}

export interface ModelLabel {
	label: string;
	color?: HexString;
}

export const UNKNOWN_MODEL_LABEL: ModelLabel = {
	label: "Unknown",
};

export interface CompletionRequest {
	prompt: string;
	count: number;
	jsonValues?: Map<string, string>;
}

export interface CompletionResponse {
	top_logprobs: Map<ULID, Array<Array<[number, string]>>>;
	responses: Map<ULID, Array<string> | Array<Array<[number, string]>>>;
}

export async function runCompletion(
	config: ClientSettings,
	request: Request
): Promise<Response> {
	let requests = [];

	for (const model of config.models) {
	}
	//fetch(endpoint.baseUrl);
}
