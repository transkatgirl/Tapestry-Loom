const input = eval("(" + input_data + ")");

const SetCodec = {
	serialize: (value) => [...value],
	deserialize: (values) => new Set(values),
};

const MapCodec = {
	serialize: (value) => Object.fromEntries(value.entries()),
	deserialize: (values) => new Map(Object.entries(values)),
};

JSON.stringify(input, function (key, value) {
	if (value instanceof Set) return SetCodec.serialize(value);
	if (value instanceof Map) return MapCodec.serialize(value);

	return value;
});
