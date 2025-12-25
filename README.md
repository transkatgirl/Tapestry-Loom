# Tapestry Loom

A power user focused interface for LLM base models, inspired by the designs of [loom](https://github.com/socketteer/loom), [loomsidian](https://github.com/cosmicoptima/loom), [exoloom](https://exoloom.io), [logitloom](https://github.com/vgel/logitloom), and [wool](https://github.com/lyramakesmusic/wool).

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

[llama.cpp](https://github.com/ggml-org/llama.cpp)'s llama-server is recommended, as it has been confirmed to work properly with *all* of the features within Tapestry Loom.

**Ollama should *not* be used** due to [bad sampling settings](https://docs.ollama.com/modelfile#valid-parameters-and-values) which [cannot be overridden in API requests](https://github.com/ollama/ollama/issues/11325), along with a lack of available base models.

KoboldCpp is not recommended due to a lack of request queuing and a poor implementation of logprobs (the number of requested logprobs is entirely ignored).

LM Studio is not recommended due to a lack of support for logprobs.

The recommended CLI arguments for [llama-server](https://github.com/ggml-org/llama.cpp/tree/master/tools/server) are listed below:

```bash
llama-server --models-dir $MODEL_DIRECTORY --models-max 1 --sleep-idle-seconds 1200 --jinja --chat-template "message.content" --ctx-size 4096 --temp 1 --top-k 0 --top-p 1 --min-p 0
```

Where `$MODEL_DIRECTORY` is set to the directory where model gguf files are stored.

(Regarding quantization: Benchmarks of how chat models are affected by quantization likely do not generalize to how base models are used. Quantization should be kept as low as reasonably possible, but `q8_0` is likely good enough for most use cases.)

Explanation of arguments:
- Only one model loaded into VRAM at a time; old models are automatically unloaded to make room for new ones
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

### Tokenization server (optional)

See [tapestry-tokenize](./tapestry-tokenize/README.md) for more information on how to configure and use the (optional) tokenization server.

Once a tokenization endpoint is configured for a model, enabling the setting "(Opportunistically) reuse output token IDs" can *slightly* improve output quality. However, the benefit is largest when generating single-token nodes using non-ASCII characters and a single model (output token IDs cannot be reused across models).

This setting requires the inference backend to support returning token IDs (to check if this is working, hover over generated tokens in the text editor to see if they contain a token identifier). This is a non-standard addition to the OpenAI Completions API which is currently supported by very few inference backends ([llama.cpp](https://github.com/ggml-org/llama.cpp) has been confirmed to work properly with this feature).

If your inference backend returns token IDs in OpenAI-style Completions responses but they do not appear in your weaves, please file an issue.

## Plans

At the moment, all major features planned for the initial release have been implemented. Development will slow down for the next few months, as the focus shifts towards fixing bugs and improving documentation.

Development of the next major version of Tapestry Loom will begin in Q1 2026. Please [consider donating](https://github.com/sponsors/transkatgirl) to help fund further development.

### Plans for next major version

- [ ] Support for DAG-based Weaves, similar to this [unreleased loom implementation](https://www.youtube.com/watch?v=xDPKR271jas&list=PLFoZLLI8ZnHCaSyopkws_9344avJQ_VEQ&index=19)
	- [ ] FIM completions
		- [ ] Selected text is used to determine FIM location
	- [ ] Node copying & moving
	- [ ] Perform heavy testing of data structures and/or formal verification to prevent bugs that could result in data loss
	- [ ] Implement node "editing" UI (not actually editing node content, but editing the tree by adding nodes / splitting nodes / merging nodes), similar to [inkstream](https://inkstream.ai)
	- [ ] Fully immutable nodes; Node splitting is implemented through duplication
	- [ ] Prefix-based duplication
	- [ ] Implement counterfactual logprobs choosing, similar to [loom](https://github.com/socketteer/loom)
- [ ] Embedding model support
	- [ ] Node ordering by [seriation](https://www.lesswrong.com/posts/u2ww8yKp9xAB6qzcr/if-you-re-not-sure-how-to-sort-a-list-or-grid-seriate-it)
- [ ] Node confidence calculation
	- [ ] Node ordering by confidence
- [ ] Improve token confidence calculation to work properly with vLLM
- [ ] Request post-processing arguments (using prefix of `T#`)
	- [ ] Single-token node pruning:
		- [ ] `T#keep_top_p`
		- [ ] `T#keep_top_k`
		- [ ] `T#prune_empty`
	- [ ] Node pruning:
		- [ ] `T#node_min_conf`
		- [ ] `T#node_max_conf`
		- [ ] `T#node_min_avg_p`
		- [ ] `T#node_max_avg_p`
		- [ ] `T#prune_empty`
	- [ ] Basic adaptive looming:
		- [ ] `T#min_tokens`
		- [ ] `T#p_threshold`
		- [ ] `T#conf_threshold`
	- [ ] Force single token node creation using `T#force_single_token`
	- [ ] Context window wrapping using `T#ctx_length`
- [ ] Support opening weaves using CLI arguments to tapestry loom
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
- [ ] Further UI improvements
	- [ ] Better handle enter in dialogs
	- [ ] Allow temporarilly overriding color in inference menu
	- [ ] Add model configuration sharing functionality
		- [ ] Automatically redact sensitive information (such as API keys)
		- [ ] Allow the user to manually redact sensitive information
	- [ ] Add ability to manually control refreshing of model tokenization identifier
	- [ ] Improve graph/canvas layout algorithm
		- [ ] Add generate buttons (displayed on hover) to canvas
	- [ ] Support arbitrary color gradients for logprob highlighting
	- [ ] Blind comparison modes
		- [ ] (Hide) Models & token probabilities / boundaries
		- [ ] (Hide) Generated node text (only showing metadata & probabilities)
	- [ ] Improve handling of hovered + omitted/collapsed nodes
	- [ ] Better handle valid UTF-8 character split across multiple nodes
	- [ ] Improve clarity of error messages
	- [ ] Better file manager
	- [ ] Support keyboard shortcuts for all aspects of the UI, not just the weave editor
		- [ ] Aim to support navigating the entirety of the UI without a mouse
	- [ ] Improve built-in color schemes
	- [ ] Node finding
	- [ ] Customizable node sorting
		- [ ] Time added
		- [ ] Alphabetical
		- [ ] Semantic sorting
	- [ ] Customizable node color coding
		- [ ] Probability
		- [ ] Confidence
	- [ ] Node bulk selection
	- [ ] Node custom ordering via drag and drop
		- [ ] Support reordering nodes in canvas and graph views as well
	- [ ] Keyboard shortcut presets
		- [ ] Built-in presets
			- [ ] [loomsidian](https://github.com/cosmicoptima/loom)-like
			- [ ] [exoloom](https://exoloom.io)-like
			- [ ] Tapestry Loom
		- [ ] Saving & loading custom presets
			- [ ] Importing & exporting custom presets
	- [ ] Support touchscreen-only devices
	- [ ] Show hovered child of active node in editor, similar to [exoloom](https://exoloom.io)
	- [ ] Add ability to add custom labels to bookmarks/nodes
	- [ ] Add ability to add custom attributes to nodes, rather than just bookmarks
- [ ] Weave statistical analysis tools
	- [ ] Predictability analysis using logprobs
	- [ ] Statistical analysis of various metrics (model usage, text length, logprobs, number of branches, etc)
- [ ] Token streaming and display of nodes being generated
- [ ] Optimize for performance whenever possible
	- [ ] Aim to have acceptable performance on weaves with ~1 million nodes, ~200k active and ~10MB of active text on low-end hardware (such as a Raspberry Pi)
		- [ ] Implement a special "link" node to allow splitting giant weaves into multiple documents
	- [ ] Optimize memory usage to be as low as reasonably possible
- [ ] Add support for more weave migrations
	- [ ] bonsai (using [damask](https://github.com/tel-0s/damask))
	- [ ] [wool](https://github.com/lyramakesmusic/wool)
	- [ ] [helm](https://github.com/Shoalstone/helm)
	- [ ] (subset of) [miniloom](https://github.com/JD-P/miniloom)
- [ ] Support [Standard Completions](https://standardcompletions.org) (after the specification is finalized)

Note: Tapestry Loom will be *entirely* focused on base and/or embedding models for the foreseeable future.

There are already good chat looms (such as [miniloom](https://github.com/JD-P/miniloom)) and base model looms which heavily integrate assistant functionality (such as [helm](https://github.com/Shoalstone/helm)); Tapestry Loom will **not** be one of them.

### Speculative ideas

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
-->

See also: the [original rewrite plans](https://github.com/transkatgirl/Tapestry-Loom/blob/c8ccca0079ae186fcc7a70b955b2d2b123082d63/README.md)