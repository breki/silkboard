#!/bin/sh
# Operator-owned paths only; native payloads cannot choose code or credentials.
publisher=${SILKBOARD_PUBLISH_BIN:-"$HOME/.local/bin/silkboard-publish"}
config=${SILKBOARD_PUBLISH_CONFIG:-"$HOME/.config/silkboard-publish/config.json"}
if [ ! -x "$publisher" ] || [ ! -f "$config" ]; then
    printf '{}\n'
    exit 0
fi
"$publisher" codex-hook --config "$config" 2>/dev/null || printf '{}\n'
exit 0
