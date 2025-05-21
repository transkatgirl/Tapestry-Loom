import { HexString, requestUrl } from "obsidian";
import { ULID } from "ulid";

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
	headers?: Record<string, string>;
	json?: Record<string, string>;
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
	json?: Record<string, string>;
}

export interface CompletionResponse {
	model: ModelConfiguration;
	topProbs?: Array<[number, string]>;
	completion: string | Array<[number, string]>;
}

export async function runCompletion(
	_config: ClientSettings,
	models: Array<ModelConfiguration>,
	request: CompletionRequest
): Promise<Array<CompletionResponse>> {
	const requests: Array<Promise<Array<CompletionResponse>>> = [];

	for (const model of models) {
		const body = {
			text: request.prompt,
			...model.json,
			...request.json,
		};

		if (model.type == EndpointType.OpenAICompletionv1Compatible) {
			requests.push(
				requestUrl({
					url: model.url,
					method: "POST",
					contentType: "application/json",
					body: JSON.stringify(body),
					headers: model.headers,
				}).then((response) => {
					const responses: Array<CompletionResponse> = [];

					for (const result of response.json["choices"]) {
						const logprobs = result["logprobs"];

						if (logprobs) {
							const tokens: Array<[number, string]> = [];
							for (
								let i = 0;
								i < logprobs["tokens"].length;
								i++
							) {
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
						} else {
							responses.push({
								model: model,
								completion: result["text"],
							});
						}
					}

					return responses;
				})
			);
		} else {
			throw new Error("unimplemented!");
		}
	}

	return Promise.all(requests).then((responses) => responses.flat());
}
