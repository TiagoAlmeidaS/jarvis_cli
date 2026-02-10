#!/bin/bash

# Set "chatgpt.cliExecutable": "/Users/<USERNAME>/code/jarvis/scripts/debug-jarvis.sh" in VSCode settings to always get the 
# latest jarvis-rs binary when debugging Jarvis Extension.


set -euo pipefail

JARVIS_RS_DIR=$(realpath "$(dirname "$0")/../jarvis-rs")
(cd "$JARVIS_RS_DIR" && cargo run --quiet --bin jarvis -- "$@")