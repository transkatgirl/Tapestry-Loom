import { UUID } from "node:crypto";
import { requestUrl, RequestUrlParam } from "obsidian";

export interface ClientSettings {
	clientIdentifier?: string;
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

export interface CompletionRequest {
	prompt: string;
	completionCount: number;
	completionLength: number;
	modelSet: Map<ConfiguredEndpoint, Array<string>>;
	temperature?: number;
	topK?: number;
	topP?: number;
	minP?: number;
	frequencyPenalty?: number;
	presencePenalty?: number;
	bestOf?: number;
}

export interface CompletionResponse {
	responses: Map<UUID, Array<string> | Array<[number, string]>>;
}

export async function runCompletions(
	request: CompletionRequest
): Promise<CompletionResponse> {
	//fetch(endpoint.baseUrl);
}

export interface LogitCompletionRequest {
	prompt: string;
	completionDepth: number;
	topK?: number;
	topP?: number;
	minP?: number;
	modelSet: Map<ConfiguredEndpoint, Array<string>>;
}

export interface LogitCompletionResponse {
	responses: Map<UUID, Array<Array<[number, string]>>>;
}

export async function runLogitCompletions(
	request: LogitCompletionRequest
): Promise<LogitCompletionResponse> {
	//requestUrl()
}
