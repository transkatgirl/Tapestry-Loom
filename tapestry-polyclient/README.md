## Tapestry PolyClient

This crate contains [Tapestry Loom](https://github.com/transkatgirl/Tapestry-Loom)'s LLM inference client. Unlike most LLM inference libraries, this library specifically focuses on base model inference (**chat models are unsupported**) and first-class support for open source LLM inference backends.

Unlike Tapestry Loom (which is licensed under AGPLv3), tapestry-polyclient is licensed under the [Unlicense](./LICENSE).

## Supported APIs

At this time, the following APIs are implemented by this library:
- OpenAI Completions (/v1/completions)

At this time, the following LLM inference backends have first-class support:
- [llama-server](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md)
- [vLLM](https://docs.vllm.ai/en/latest/)

Other inference backends will likely work, but any non-standard fields may be ignored.