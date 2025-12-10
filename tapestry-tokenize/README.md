# Tapestry Tokenize

> [!WARNING]
> The current code is a prototype that does not correctly handle invalid UTF-8 and does not support streaming data to reduce memory usage.
>
> **There is *currently* no advantage whatsoever to using this in your Tapestry Loom configuration!**

A server which provides a basic HTTP API for tokenizing and detokenizing inputs.

## Usage

In order to run the server, you will need to create a `models.toml` file with the following structure:

```toml
[[models]] # Add a [[models]] block for every model you want to specify
label = "test" # The label for the model, used in API requests
file  = "./models/test/tokenizer.json" # The path to the model's tokenizer.json file.
```

After being configured, the server can be started using the `cargo run --release` command.

### API Endpoints

The server provides the following API endpoints

- POST `/<model>/tokenize`
	- Input: An HTTP body containing the bytes you want to tokenize
	- Output: A JSON array of token IDs
- POST `/<model>/detokenize`
	- Input: A JSON array of token IDs (same format that is output by the `/tokenize` endpoint)
	- Output: The decoded bytes