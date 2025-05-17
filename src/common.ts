// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function deserialize(serializedJavascript: any) {
	return eval("(" + serializedJavascript + ")");
}
