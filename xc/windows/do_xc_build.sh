#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONTAINER_CMD="cargo build --bin refbox --target x86_64-pc-windows-msvc --release"
CONTAINER_WORKDIR="/workdir"

CONTAINER_NAME="$(docker create -t -w "$CONTAINER_WORKDIR/" windows-rust-1.77-xc /bin/bash -c "$CONTAINER_CMD")"

for file in $(ls "$SCRIPT_DIR/../.." | grep -v target | grep -v xc); do
    docker cp "$SCRIPT_DIR/../../$file" "$CONTAINER_NAME:$CONTAINER_WORKDIR/"
done

docker start -a "$CONTAINER_NAME"

docker cp "$CONTAINER_NAME:$CONTAINER_WORKDIR/target/x86_64-pc-windows-msvc/release/refbox.exe" "$SCRIPT_DIR/output/"

docker rm "$CONTAINER_NAME"
