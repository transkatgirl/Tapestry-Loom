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

export function truncateWithEllipses(text: string, max: number) {
	if (text.length > max) {
		return text.substring(0, max) + "…";
	}
	return text;
}

// @ts-expect-error
import fromBase64 from "es-arraybuffer-base64/Uint8Array.fromBase64";

export async function compress(string: string) {
	const byteArray = new TextEncoder().encode(string);
	// @ts-ignore
	const cs = new CompressionStream("deflate");
	const writer = cs.writable.getWriter();
	writer.write(byteArray);
	writer.close();
	const buffer = await new Response(cs.readable).arrayBuffer();
	return base64ArrayBuffer(buffer);
}

export async function decompress(compressed: string) {
	const byteArray = fromBase64(compressed);
	// @ts-ignore
	const cs = new DecompressionStream("deflate");
	const writer = cs.writable.getWriter();
	writer.write(byteArray);
	writer.close();
	return await new Response(cs.readable)
		.arrayBuffer()
		.then(function (arrayBuffer) {
			return new TextDecoder().decode(arrayBuffer);
		});
}

function base64ArrayBuffer(arrayBuffer: ArrayBuffer) {
	let base64 = "";
	const encodings =
		"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
	const bytes = new Uint8Array(arrayBuffer);
	const byteLength = bytes.byteLength;
	const byteRemainder = byteLength % 3;
	const mainLength = byteLength - byteRemainder;
	let a, b, c, d;
	let chunk;
	for (let i = 0; i < mainLength; i = i + 3) {
		chunk = (bytes[i] << 16) | (bytes[i + 1] << 8) | bytes[i + 2];
		a = (chunk & 16515072) >> 18;
		b = (chunk & 258048) >> 12;
		c = (chunk & 4032) >> 6;
		d = chunk & 63;
		base64 += encodings[a] + encodings[b] + encodings[c] + encodings[d];
	}
	if (byteRemainder == 1) {
		chunk = bytes[mainLength];
		a = (chunk & 252) >> 2;
		b = (chunk & 3) << 4;
		base64 += encodings[a] + encodings[b] + "==";
	} else if (byteRemainder == 2) {
		chunk = (bytes[mainLength] << 8) | bytes[mainLength + 1];
		a = (chunk & 64512) >> 10;
		b = (chunk & 1008) >> 4;
		c = (chunk & 15) << 2;
		base64 += encodings[a] + encodings[b] + encodings[c] + "=";
	}
	return base64;
}
