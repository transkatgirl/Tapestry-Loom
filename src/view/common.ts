import TapestryLoom, { DOCUMENT_TRIGGER_UPDATE_EVENT } from "main";
import { Notice } from "obsidian";
import { ULID, ulid } from "ulid";
import { runCompletion } from "client";
import { DEFAULT_DOCUMENT_SETTINGS } from "settings";

export async function generateNodeChildren(
	plugin: TapestryLoom,
	parentNode?: ULID
) {
	if (!plugin.document || !plugin.settings.client) {
		return;
	}

	const completionPromises = runCompletion(
		plugin.settings.client,
		plugin.sessionSettings.models,
		{
			prompt: plugin.document.getActiveContent(parentNode),
			count: plugin.sessionSettings.requests,
			parameters: plugin.sessionSettings.parameters,
		}
	);

	const debounceTime =
		plugin.settings.document?.debounce ||
		DEFAULT_DOCUMENT_SETTINGS.debounce;

	let lastUpdate = performance.now();

	for (const completionPromise of completionPromises) {
		completionPromise
			.then((completions) => {
				if (!plugin.document) {
					return;
				}

				for (const completion of completions) {
					if (completion.topProbs && completion.topProbs.length > 1) {
						for (const prob of completion.topProbs) {
							plugin.document.addNode(
								{
									identifier: ulid(),
									content: [prob],
									model: completion.model.ulid,
									parentNode: parentNode,
								},
								completion.model.label
							);
						}
					}

					if (
						typeof completion.completion == "string" ||
						!completion.topProbs ||
						completion.completion.length > 1
					) {
						plugin.document.addNode(
							{
								identifier: ulid(),
								content: completion.completion,
								model: completion.model.ulid,
								parentNode: parentNode,
							},
							completion.model.label
						);
					}
				}

				const currentTimestamp = performance.now();

				if (currentTimestamp - lastUpdate > debounceTime) {
					plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
					lastUpdate = currentTimestamp;
				}
			})
			.catch((error) => {
				new Notice(error);
			});
	}

	await Promise.all(completionPromises);

	plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
}

export function addNode(plugin: TapestryLoom, parentNode?: ULID) {
	if (!plugin.document) {
		return;
	}

	const identifier = ulid();
	plugin.document.addNode({
		identifier: identifier,
		content: "",
		parentNode: parentNode,
	});
	plugin.document.currentNode = identifier;
	plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
}

export function switchToNode(plugin: TapestryLoom, identifier: ULID) {
	if (!plugin.document) {
		return;
	}

	plugin.document.currentNode = identifier;
	plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
}

export function toggleBookmarkNode(plugin: TapestryLoom, identifier: ULID) {
	if (!plugin.document) {
		return;
	}

	if (plugin.document.bookmarks.has(identifier)) {
		plugin.document.bookmarks.delete(identifier);
	} else {
		plugin.document.bookmarks.add(identifier);
	}

	plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
}

export function mergeNode(
	plugin: TapestryLoom,
	primaryIdentifier: ULID,
	secondaryIdentifier: ULID
) {
	if (!plugin.document) {
		return;
	}

	plugin.document.mergeNode(primaryIdentifier, secondaryIdentifier);
	plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
}

export function deleteNode(plugin: TapestryLoom, identifier: ULID) {
	if (!plugin.document) {
		return;
	}

	plugin.document.removeNode(identifier);
	plugin.app.workspace.trigger(DOCUMENT_TRIGGER_UPDATE_EVENT);
}
