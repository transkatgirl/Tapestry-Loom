llama-server --models-dir $1 --models-max 1 --sleep-idle-seconds 1200 --jinja --chat-template "message.content" --ctx-size 4096 --temp 1 --top-k 0 --top-p 1 --min-p 0
