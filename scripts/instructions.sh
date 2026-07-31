#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ $# -eq 0 ]]; then
    exec straitjacket instructions
fi

straitjacket instructions | jq -Rs --arg event "$1" \
    '{hookSpecificOutput: {hookEventName: $event, additionalContext: .}}'
