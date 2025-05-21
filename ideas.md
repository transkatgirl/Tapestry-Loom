# custom loom implementation

- color coding by model used (or by human collaborator)
- color coding by token probability
    - color coding by entropy
- mix of logitloom, exoloom and vanilla loom in terms of branching functionality
- running completions on multiple models at a time, color coding by model
- toggling what JSON options a model supports
- visualizing token distribution and how sampling parameters affect it
- automatically adjust visible branches
- built in user guide

## toggleable branching interfaces:

- full tree view (similar to exoloom); no inference parameters displayed
- recent nodes/logits (similar to loomsidian + logitloom, but with automatic hoisting) + inference parameters

maybe the last two can be combined somehow?

## user guide

- not just an ordinary manual; contains compelling narratives and good aesthetics to help get the user interested in base models
- explain how LLMs work
- explain what base models are
- explain how to use LLM samplers
- explain how to use base models effectively
    - writing
        - leveraging both what the human is good at and what the LLM is good at
            - human is better at navigating the "edge of chaos" that creativity thrives on than LLMs
            - low probability / high temperature LLM predictions can be useful to introduce chaos
        - using LLM pattern recognition to your advantage in writing stories
            - LLM can help when the human is stuck, but generated text can be too predictable
            - human can notice patterns in LLM predictions to create less predictable texts
    - using base models for the sake of understanding them

## settings

- interface customization
- HTTP client
- API endpoints
    - type
        - OpenAI
        - Anthropic
        - OpenRouter
        - OpenAI-compatible (Completions)
		- OpenAI-compatible (ChatCompletions)
        - vLLM
        - TGI
        - llama.cpp
        - LM Studio
        - Custom JSON
    - authorization
        - organization
    - rate limits
    - supported request options
        - samplers
        - n
        - logprobs
- models
    - endpoint
    - identifier
    - type
