MODEL_DIRECTORY="${1:-.}"
PORT="${2:-8080}"

llama-server --embeddings --models-dir $MODEL_DIRECTORY --models-max 1 --sleep-idle-seconds 1200 --port $PORT --jinja --chat-template "message.content" --ctx-size 4096 --temp 1 --top-k 0 --top-p 1 --min-p 0
