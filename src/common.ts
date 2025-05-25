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

export function joinByteArrays(input: Array<Uint8Array>) {
	const totalLength = input.reduce(
		(total, uint8array) => total + uint8array.byteLength,
		0
	);

	const result = new Uint8Array(totalLength);

	let offset = 0;
	input.forEach((uint8array) => {
		result.set(uint8array, offset);
		offset += uint8array.byteLength;
	});

	return result;
}
