// @ts-expect-error
import crass from "crass";

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

export function truncateWithEllipses(text: string, max: number) {
	if (text.length > max) {
		return text.substring(0, max) + "…";
	}
	return text;
}
