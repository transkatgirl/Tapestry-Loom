# Tapestry Tokenize

> [!WARNING]
> The current tokenization backend (huggingface tokenizers) does not correctly handle invalid UTF-8 characters.

A server which provides a basic HTTP API for tokenizing and detokenizing inputs.

## Getting started

### Binary releases

See [tapestry-loom's documentation](../README.md) in order to download a binary release.

### Compiling from source

Once you are in the tapestry-tokenize folder, you can run the server with the following command:

```bash
cargo run --release
```

## Usage

In order to start the server, you will need to create a `models.toml` file in the server's working directory.

This file should contain the following structure:

```toml
[[models]] # Add a [[models]] block for every model you want to specify
label = "test" # The label for the model, used in API requests
file  = "./models/test/tokenizer.json" # The path to the model's tokenizer.json file.
```

### API Endpoints

The server provides the following API endpoints

- POST `/<model>`
	- Input: An HTTP body containing the bytes you want to tokenize
	- Output: A JSON array of token IDs
- POST `/<model>/tokenize`
	- Input: An HTTP body containing the bytes you want to tokenize
	- Output: A JSON array of token IDs
- POST `/<model>/detokenize`
	- Input: A JSON array of token IDs (same format that is output by the `/tokenize` endpoint)
	- Output: The decoded bytes

### Using Tapestry Tokenize within Tapestry Loom

You can configure Tapestry Loom to use Tapestry Tokenize on OpenAI-style Completion APIs by opening the model's "Non-standard API modifications" dropdown and inputting the tokenization URL in the "Tapestry-Tokenize Endpoint" input box.

(Example: if your model label is `myfavoritellm` and the server is running on `http://127.0.0.1:8000` (the default), you would input this URL into the input box: `http://127.0.0.1:8000/myfavoritellm`)