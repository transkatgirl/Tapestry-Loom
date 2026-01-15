# Tapestry Loom

A power user focused interface for LLM base models, inspired by the designs of [loom](https://github.com/socketteer/loom), [loomsidian](https://github.com/cosmicoptima/loom), [exoloom](https://exoloom.io), [logitloom](https://github.com/vgel/logitloom), and [wool](https://github.com/lyramakesmusic/wool).<!-- and [mikupad](https://github.com/lmg-anon/mikupad) -->

<details>

<summary>Screenshots</summary>

![](./docs/screenshots/tree+edit.png)
![](./docs/screenshots/graph+edit.png)
![](./docs/screenshots/canvas+edit.png)
![](./docs/screenshots/inference-settings.png)

</details>

## Known issues

- Some documents may cause the text editor to render token boundaries incorrectly
	- This is due to a bug in egui regarding textedit underline rendering
- Tab bars are not read by screen readers
	- This is due to a bug in egui_tiles
- UI state for closed weaves persists in memory after the weave editor UI is closed, creating a slow memory leak
	- This is due to a bug in egui
- CPU usage is high when the window is not visible and not minimized
	- This is due to a [bug in egui](https://github.com/emilk/egui/issues/7776)
- Root nodes containing long text may overlap in the canvas view

If you are experiencing an issue not listed here or in this [repository's active issues](https://github.com/transkatgirl/Tapestry-Loom/issues), please file an issue so that it can be fixed.

## Getting started

> [!IMPORTANT]
> This application is a work in progress; Please make backups and [report any bugs that you find](https://github.com/transkatgirl/Tapestry-Loom/issues).

### Binary releases

Compiled binaries can be found on the [releases page](https://github.com/transkatgirl/Tapestry-Loom/releases).

#### MacOS-specific instructions

Before using the app, you will need to run the following CLI command in the extracted folder:

```bash
xattr -d com.apple.quarantine tapestry*
```

### Compiling from source

Requires the [Rust Programming Language](https://rust-lang.org/tools/install/) and a working C compiler to be installed.

```bash
git clone --recurse-submodules https://github.com/transkatgirl/Tapestry-Loom.git
cd Tapestry-Loom
cargo build --release
```

The compiled binary can be found in the ./target/release/ folder.

#### Updating

Run the following commands in the repository folder:

```bash
git pull
git submodule update --init --recursive
cargo build --release
```

## Usage

See [Getting Started](./Getting%20Started.md) for more information on how to use the application.

The rest of this README covers the usage of external tools which Tapestry Loom can interface with.

### Migrating weaves from other Loom implementations

See [migration-assistant](./migration-assistant/README.md) for more information on how to migrate weaves from other Loom implementations to Tapestry Loom.

### Local inference

[llama.cpp](https://github.com/ggml-org/llama.cpp)'s llama-server is recommended, as it has been confirmed to work properly with *all* of the features within Tapestry Loom (except [returning prompt logprobs](https://github.com/ggml-org/llama.cpp/pull/17935)).

[vLLM](https://vllm.ai) requires additional request arguments to work properly with Tapestry Loom:
- /v1/completions
	- `return_token_ids` = `true`
		- Optional; Allows (partial) reuse of output token IDs when using Tapestry Tokenize. However, (unlike llama.cpp) token IDs are only returned for the selected token, not for all top_logprobs.
		- Must be removed when using `echo` = `true`
- /v1/chat/completions
	- `return_token_ids` = `true`
		- Optional; Allows (partial) reuse of output token IDs when using Tapestry Tokenize. However, (unlike llama.cpp) token IDs are only returned for the selected token, not for all top_logprobs.
	- `continue_final_message` = `true`
	- `add_generation_prompt` = `false`

**Ollama should *not* be used** due to [bad sampling settings](https://docs.ollama.com/modelfile#valid-parameters-and-values) which [cannot be overridden in API requests](https://github.com/ollama/ollama/issues/11325), along with a lack of available base models.

KoboldCpp is not recommended due to a lack of request queuing and a poor implementation of logprobs (the number of requested logprobs is entirely ignored).

LM Studio is not recommended due to a lack of support for logprobs.

The recommended CLI arguments for [llama-server](https://github.com/ggml-org/llama.cpp/tree/master/tools/server) are listed below:

```bash
llama-server --embeddings --models-dir $MODEL_DIRECTORY --models-max 1 --sleep-idle-seconds 1200 --jinja --chat-template "message.content" --ctx-size 4096 --temp 1 --top-k 0 --top-p 1 --min-p 0
```

Where `$MODEL_DIRECTORY` is set to the directory where model gguf files are stored.

(Regarding quantization: Benchmarks of how chat models are affected by quantization likely do not generalize to how base models are used. Quantization should be kept as low as reasonably possible, but `q8_0` is likely good enough for most use cases.)

Explanation of arguments:
- Only one model loaded into VRAM at a time; old models are automatically unloaded to make room for new ones
	- If you plan on using an embedding model, you should start a second server instance to avoid swapping out your text generation model when generating embeddings
- Models are automatically unloaded after 20 minutes of inactivity
- The specified chat template passes user input directly to the model without further changes.
- Reducing the maximum context length helps reduce VRAM usage without sacrificing quality.
- The default sampling parameters (those specified by the CLI arguments) should leave the model's output distribution unchanged. **Sampling parameter defaults for chat models do not generalize to how base models are used.**
	- The sampling parameters specified in the CLI arguments will be overridden by any sampling parameters that are specified in a request.

Additional useful arguments (depending on your use case):
- `--no-cont-batching`
	- Disabling continuous batching significantly improves response determinism at the expense of performance. Should be used if you plan on analyzing logprobs or using greedy sampling.

If you are running llama-server on the same device as Tapestry Loom (and you are using the default port), you do not need to explicitly specify an endpoint URL when filling out the "OpenAI-style Completions" and "OpenAI-style ChatCompletions" templates.

#### Recommended models

If you are new to working with LLM base models, [Trinity-Mini-Base-Pre-Anneal](https://huggingface.co/mradermacher/Trinity-Mini-Base-Pre-Anneal-GGUF) or ([Trinity-Nano-Base-Pre-Anneal](https://huggingface.co/mradermacher/Trinity-Nano-Base-Pre-Anneal-GGUF) if you have <32GB of VRAM) is a good first model to try.

If you plan on using seriation, [embeddinggemma-300m](https://huggingface.co/unsloth/embeddinggemma-300m-GGUF) is a good small embedding model.

### Inference providers

Most inference providers support OpenAI-compatible clients and should work with minimal configuration.

However, every inference provider implements OpenAI compatibility in their own way, which may cause unexpected issues. Known issues with popular inference providers are listed below:

- OpenRouter
	- Some providers on OpenRouter will return errors if `logprobs` is included as a request argument
- Featherless
	- Untested; Logprobs are not supported according to documentation

### Tokenization server (optional)

See [tapestry-tokenize](./tapestry-tokenize/README.md) for more information on how to configure and use the (optional) tokenization server.

Once a tokenization endpoint is configured for a model, enabling the setting "(Opportunistically) reuse output token IDs" can *slightly* improve output quality. However, the benefit is largest when generating single-token nodes using non-ASCII characters and a single model (output token IDs cannot be reused across models).

This setting requires the inference backend to support returning token IDs (to check if this is working, hover over generated tokens in the text editor to see if they contain a token identifier). This is a non-standard addition to the OpenAI Completions API which is currently supported by very few inference backends ([llama.cpp](https://github.com/ggml-org/llama.cpp) has been confirmed to work properly with this feature).

If your inference backend returns token IDs in OpenAI-style Completions responses but they do not appear in your weaves, please file an issue.

## Development roadmap

Please [consider donating](https://github.com/sponsors/transkatgirl) to help fund further development.

### Milestone 0

Goal: Completion before Feb 1st, 2026

- [ ] Improve handling of hovered + omitted/collapsed nodes
- [ ] Implement counterfactual logprobs, similar to [mikupad](https://github.com/lmg-anon/mikupad)
- [ ] Store generation seed in node
- [ ] Release version 0.12.0

### Milestone 1

- [ ] Implement new DAG-based Weave format, similar to this [unreleased loom implementation](https://www.youtube.com/watch?v=xDPKR271jas&list=PLFoZLLI8ZnHCaSyopkws_9344avJQ_VEQ&index=19)
	- [ ] FIM completions
		- [ ] Selected text is used to determine FIM location
	- [ ] Diff-based editor content application
	- [ ] Implement node "editing" UI (not actually editing node content, but editing the tree by adding nodes / splitting nodes / merging nodes), similar to [inkstream](https://inkstream.ai)
	- [ ] Implement a special "link" node to allow splitting giant weaves into multiple documents
- [ ] Enable counterfactual logprobs by default
- [ ] Implement fully immutable nodes using node duplication instead of direct modification

### Milestone 2

- [ ] UI improvements
	- [ ] Add content copying to node context menu
	- [ ] Add setting to swap shift-click and normal click behavior
	- [ ] Add sorting submenu to node context menu
	- [ ] Add alphabetical sorting
	- [ ] Add right click handling to node list background
- [ ] Request post-processing arguments (using prefix of `TL#`)
	- [ ] Single-token node pruning:
		- [ ] `TL#keep_top_p`
		- [ ] `TL#keep_top_k`
		- [ ] `TL#prune_empty`
	- [ ] Node pruning:
		- [ ] `TL#node_min_conf`
		- [ ] `TL#node_max_conf`
		- [ ] `TL#node_min_avg_p`
		- [ ] `TL#node_max_avg_p`
		- [ ] `TL#prune_empty`
	- [ ] Basic adaptive looming:
		- [ ] `TL#min_tokens`
		- [ ] `TL#p_threshold`
		- [ ] `TL#conf_threshold`
	- [ ] Force single token node creation using `TL#force_single_token`
	- [ ] Context window wrapping using `TL#ctx_length`
- [ ] Implement BERT FIM server using nonstandard `fim_tokens` parameter

### Milestone 3

- [ ] Better handle valid UTF-8 characters split across multiple nodes
- [ ] Support arbitrary color gradients for logprob highlighting
- [ ] Add blind comparison modes
	- [ ] (Hide) Models & token probabilities / boundaries
	- [ ] (Hide) Generated node text (only showing metadata & probabilities)
- [ ] Add weave statistical analysis tools
- [ ] Add customizable node color coding
	- [ ] Probability
	- [ ] Confidence

### Milestone 4

- [ ] Show hovered child of active node in editor, similar to [exoloom](https://exoloom.io)
- [ ] Add "autoloom" mode where clicking a node generates children, similar to [inkstream](https://inkstream.ai)
- [ ] Add node finding
- [ ] Perform UX testing with all built-in color schemes

### Milestone 5

- [ ] Improve Weave saving & loading
	- [ ] Initially load weaves using zero-copy deserialization, performing full deserialization in the background
	- [ ] Perform weave saving in the background without visual glitches
	- [ ] Support read-only weave editors using zero-copy deserialization and file memory mapping
- [ ] Optimize performance whenever reasonably possible
- [ ] Review and refactor application modules
	- [ ] settings
	- [ ] editor
- [ ] Support opening weaves using CLI arguments to tapestry loom
- [ ] Review and refactor main module

### Milestone 6

- [ ] Allow temporarilly overriding color in inference menu
- [ ] Add model configuration sharing functionality
	- [ ] Automatically redact sensitive information (such as API keys)
	- [ ] Allow the user to manually redact sensitive information
- [ ] Add ability to manually control refreshing of model tokenization identifier
- [ ] Improve API response building
	- [ ] Add support for OpenAI Responses
	- [ ] Add support for Anthropic Complete
	- [ ] Add support for Anthropic Messages
	- [ ] Add support for Gemini generateText
	- [ ] Add support for Gemini generateContent
	- [ ] Add support for Gemini embedContent

### Milestone 7

Goal: Completion before June 1st, 2026

- [ ] Add support for response streaming
- [ ] Review and refactor settings/inference module
- [ ] Perform API client testing with commonly used inference backends
	- [ ] llama-cpp
	- [ ] ollama
	- [ ] vllm
	- [ ] sglang
	- [ ] tensorrt-llm
	- [ ] text-generation-inference
	- [ ] text-embeddings-inference
	- [ ] koboldcpp
	- [ ] lm-studio
	- [ ] litellm
- [ ] Perform API client testing with less commonly used inference backends
	- [ ] lemonade
	- [ ] infinity
	- [ ] swama
	- [ ] exllamav2
	- [ ] lmdeploy
	- [ ] mlc-llm
	- [ ] shimmy
- [ ] Improve clarity of error messages

### Milestone 8

- [ ] Perform heavy unit testing and/or formal verification of `universal-weave` to prevent bugs that could result in data loss
- [ ] Release `universal-weave` version 1.0.0
- [ ] Write unit tests for response parser
- [ ] Release Tapestry Loom version 1.0.0-rc.1

### Milestone 9

- [ ] Implement token healing
- [ ] Implement support for instruct templating, similar to [mikupad](https://github.com/lmg-anon/mikupad)
- [ ] Do testing with using llamafile for easier onboarding?
- [ ] Create video-based documentation
- [ ] Add support for more weave migrations
	- [ ] improve [loom](https://github.com/socketteer/loom) migration
	- [ ] bonsai (using [damask](https://github.com/tel-0s/damask))
	- [ ] [wool](https://github.com/lyramakesmusic/wool)
	- [ ] [helm](https://github.com/Shoalstone/helm)
	- [ ] (subset of) [miniloom](https://github.com/JD-P/miniloom)

### Post-v1 plans

- [ ] Improve graph/canvas layout algorithm
- [ ] Improve file manager
- [ ] Support keyboard shortcuts for all aspects of the UI, not just the weave editor
	- [ ] Aim to support navigating the entirety of the UI without a mouse
- [ ] Improve built-in color schemes
- [ ] Node bulk selection
- [ ] Node custom ordering via drag and drop in all views
- [ ] Keyboard shortcut presets
	- [ ] Built-in presets
		- [ ] [loomsidian](https://github.com/cosmicoptima/loom)-like
		- [ ] [exoloom](https://exoloom.io)-like
		- [ ] Tapestry Loom
	- [ ] Saving & loading custom presets
		- [ ] Importing & exporting custom presets
- [ ] Support touchscreen-only devices
- [ ] Add ability to add custom labels to bookmarks/nodes
- [ ] Add ability to add custom attributes to nodes, rather than just bookmarks

### Speculative ideas for v2

- [ ] Prefix-based deduplication
- [ ] Collaborative weave editing
- [ ] WASM version of Tapestry Loom
- [ ] Support multimodal weaves
- [ ] Support weaves of arbitrarily large size using a database-based format
- [ ] Self-contained packaging: All documentation and tools in one app, rather than being spread out over multiple
- [ ] Server-client, multi-user WebUI
- [ ] Efficiently store full edit history in weave for lossless unbounded undo/redo
- [ ] Alternate input devices
	- [ ] Talon Voice
	- [ ] Controllers / Gamepads
	- [ ] USB DDR Pads

<!--
- [ ] Server-client, multi-user WebUI
	- [ ] Support collaborating on Weaves
	- [ ] User authentication
	- [ ] User permissions
	- [ ] User rate limiting
- [ ] Event-based server-client communication to reduce bandwidth usage
- [ ] Automatic color palette generation in settings
- [ ] HTTPS support
- [ ] Compression support
	- [ ] Brotli compression for static assets
	- [ ] LZ4 compression for websocket data
- [ ] Add a plugin API & custom inference API
	- [ ] Support the following use cases:
		- [ ] LLM research
			- [ ] Support adding custom UI elements and editor subviews
		- [ ] Autolooms (looms where node choices are picked by a user-determined algorithm)
		- [ ] Adaptive looming (node lengths are picked by a user-determined algorithm)
	- [ ] Implement an optional inference server using llama.cpp
		- [ ] Adaptive looming using token entropy or [confidence](https://arxiv.org/pdf/2508.15260)
		- [ ] Context window wrapping
		- [ ] Allow adjusting proportion of completions from each model
		- [ ] When working with multiple models, allow dynamically adjusting proportions based on usage
			- [ ] Flatten proportion bias when increasing number of completions, do the inverse when reducing completion count
-->

See also: the [original rewrite plans](https://github.com/transkatgirl/Tapestry-Loom/blob/c8ccca0079ae186fcc7a70b955b2d2b123082d63/README.md)