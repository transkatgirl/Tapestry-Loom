// @ts-expect-error
import crass from "crass";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function deserialize(serializedJavascript: any) {
	return eval("(" + serializedJavascript + ")");
}

/*function getGlobalCSSVariable(key: string) {
	return window.getComputedStyle(window.document.body).getPropertyValue(key);
}*/

export function getGlobalCSSColorVariable(key: string) {
	let parsed = crass.parse(
		"a{color:" +
			window
				.getComputedStyle(window.document.body)
				.getPropertyValue(key) +
			"}"
	);
	parsed = parsed.optimize();
	return parsed.toString().slice(8, -1);
}
