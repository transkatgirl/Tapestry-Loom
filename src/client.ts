import { HexString, requestUrl } from "obsidian";
import { ulid, ULID } from "ulid";

export interface ClientSettings {
	models: Array<ModelConfiguration>;
}

export enum EndpointType {
	OpenAICompletionv1Compatible = "openai_completion_v1_compatible",
}

export interface ModelConfiguration {
	ulid: ULID;
	label: ModelLabel;
	url: string;
	type: EndpointType;
	headers: Record<string, string>;
	parameters: Record<string, string>;
}

export interface ModelLabel {
	label: string;
	color?: HexString;
}

export function newModel(
	type: EndpointType,
	url?: string,
	identifier?: string,
	apiKey?: string
): ModelConfiguration | void {
	if (url && type == EndpointType.OpenAICompletionv1Compatible) {
		const headers: Record<string, string> = {};
		if (apiKey) {
			headers["Authorization"] = "Bearer " + apiKey;
		}

		const parameters: Record<string, string> = {};
		if (identifier) {
			parameters["model"] = identifier;
		}

		return {
			ulid: ulid(),
			label: {
				label: identifier || url,
			},
			url: url,
			type: type,
			headers: headers,
			parameters: parameters,
		};
	}
}

export const UNKNOWN_MODEL_LABEL: ModelLabel = {
	label: "Unknown",
};

export interface CompletionRequest {
	prompt: string;
	count: number;
	parameters?: Record<string, string>;
}

export interface CompletionResponse {
	model: ModelConfiguration;
	topProbs?: Array<[number, string]>;
	completion: string | Array<[number, string]>;
}

export async function runCompletion(
	config: ClientSettings,
	models: Array<ULID>,
	request: CompletionRequest
): Promise<Array<CompletionResponse>> {
	const modelIdentifiers = new Set(models);
	const modelObjects = [];

	for (const model of config.models) {
		if (modelIdentifiers.has(model.ulid)) {
			modelObjects.push(model);
		}
	}

	const requests: Array<Promise<Array<CompletionResponse>>> = [];

	for (const model of modelObjects) {
		for (let i = 0; i < request.count; i++) {
			requests.push(inferenceRequest(config, model, request));
		}
	}

	return Promise.all(requests).then((responses) => responses.flat());
}

async function inferenceRequest(
	_config: ClientSettings,
	model: ModelConfiguration,
	request: CompletionRequest
): Promise<Array<CompletionResponse>> {
	if (model.type == EndpointType.OpenAICompletionv1Compatible) {
		const body: Record<string, string | number> = {
			prompt: request.prompt,
			...model.parameters,
			...request.parameters,
		};

		for (const [key, value] of Object.entries(body)) {
			if (
				typeof value == "string" &&
				!isNaN(value as never) &&
				!isNaN(parseFloat(value))
			) {
				body[key] = +value;
			}
		}

		const headers: Record<string, string> = {
			Accept: "application/json",
			...model.headers,
		};

		return requestUrl({
			url: model.url,
			method: "POST",
			contentType: "application/json",
			body: JSON.stringify(body),
			headers: headers,
		}).then((response) => {
			const responses: Array<CompletionResponse> = [];

			for (const result of response.json["choices"]) {
				const logprobs = result["logprobs"];

				if (logprobs) {
					if ("content" in logprobs && logprobs["content"]) {
						const tokens: Array<[number, string]> = [];
						const probs: Array<[number, string]> = [];
						for (let i = 0; i < logprobs["content"].length; i++) {
							const prob = logprobs["content"][i];

							tokens.push([
								Math.exp(prob["logprob"]),
								prob["token"],
							]);

							if (i == 0) {
								for (const topProb of prob["top_logprobs"]) {
									probs.push([
										Math.exp(topProb["logprob"]),
										topProb["token"],
									]);
								}
							}
						}

						responses.push({
							model: model,
							completion: tokens,
							topProbs: probs,
						});
					} else {
						const tokens: Array<[number, string]> = [];
						for (let i = 0; i < logprobs["tokens"].length; i++) {
							tokens.push([
								Math.exp(logprobs["logprob"][i]),
								logprobs["tokens"][i],
							]);
						}
						const topLogprobs = logprobs["top_logprobs"];
						if (topLogprobs && topLogprobs.length > 0) {
							const probs: Array<[number, string]> = [];
							for (const [token, logprob] of topLogprobs) {
								probs.push([Math.exp(logprob), token]);
							}
							probs.sort((a, b) => {
								return a[0] - b[0];
							});

							responses.push({
								model: model,
								completion: tokens,
								topProbs: probs,
							});
						} else {
							responses.push({
								model: model,
								completion: tokens,
							});
						}
					}
				} else {
					responses.push({
						model: model,
						completion: result["text"],
					});
				}
			}

			return responses;
		});
	} else {
		throw new Error("unimplemented!");
	}
}
