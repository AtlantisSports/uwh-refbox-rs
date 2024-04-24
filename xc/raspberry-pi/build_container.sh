#!/bin/bash

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REPO_DIR=$(cd "$SCRIPT_DIR/../.." && pwd)

(
    cd "$REPO_DIR" && \
    docker build -t "rpi-xc-bullseye-aarch64" -f "$SCRIPT_DIR/Dockerfile" "$REPO_DIR"
)
