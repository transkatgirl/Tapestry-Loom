import { UUID } from "node:crypto";
import { requestUrl, RequestUrlParam } from "obsidian";

export interface ClientSettings {
	clientIdentifier?: string;
	endpoints: Array<ConfiguredEndpoint>;
}

export interface ConfiguredEndpoint {
	baseUrl: string;
	type: EndpointType;
	apiToken?: string;
	organization?: string;
	models: Map<UUID, ModelConfiguration>;
	customHeaders?: Map<string, string>;
	customValues?: Map<string, string>;
}

export enum EndpointType {
	OpenRouter = "open_router",
	OpenAIv1Compatible = "openai_v1_compatible",
	VLLM_v0 = "vllm_v0",
	TGI_v3 = "tgi_v3",
	LlamaCppServer = "llama_cpp",
	Ollama_v0 = "ollama_v0",
	LMStudio_v0 = "lm_studio_v0",
}

export interface ModelConfiguration {
	type: ModelType;
	identifier: string;
	supportedValues: Set<string>;
	maxLogits?: number;
	maxContext?: number;
	customHeaders?: Map<string, string>;
	customValues?: Map<string, string>;
}

export enum ModelType {
	completion = "completion",
}

export interface BranchRequest {
	prompt: string;
	completionCount: number;
	completionLength: number;
	modelSet: Set<UUID>;
	temperature?: number;
	topK?: number;
	topP?: number;
	minP?: number;
	frequencyPenalty?: number;
	presencePenalty?: number;
	bestOf?: number;
}

export interface BranchResponse {
	responses: Map<UUID, Array<string> | Array<Array<[number, string]>>>;
}

export async function runBranches(
	config: ClientSettings,
	request: BranchRequest
): Promise<BranchResponse> {
	let requests = [];

	for (const endpoint of config.endpoints) {
		for (const [identifier, config] of endpoint.models) {
		}
	}
	//fetch(endpoint.baseUrl);
}

export interface LogitBranchRequest {
	prompt: string;
	completionDepth: number;
	topK?: number;
	topP?: number;
	minP?: number;
	modelSet: Set<UUID>;
}

export interface LogitBranchResponse {
	responses: Map<UUID, Array<Array<[number, string]>>>;
}

export async function runLogitBranches(
	config: ClientSettings,
	request: LogitBranchRequest
): Promise<LogitBranchResponse> {
	throw new Error("unimplemented");
}
