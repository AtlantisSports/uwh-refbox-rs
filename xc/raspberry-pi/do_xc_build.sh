#!/bin/bash

set -x

TARGET_TRIPLE="aarch64-unknown-linux-gnu"
IMAGE_NAME="rpi-xc-bullseye-aarch64"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CARGO_ARGS="build --bin refbox --bin overlay --target $TARGET_TRIPLE --release"
CONTAINER_WORKDIR="/workdir"

CONTAINER_NAME="$(docker create -t -w "$CONTAINER_WORKDIR/" $IMAGE_NAME /root/.cargo/bin/cargo $CARGO_ARGS)"

BASE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

for file in $(ls "$BASE_DIR" | grep -v target | grep -v xc); do
    docker cp "$BASE_DIR/$file" "$CONTAINER_NAME:$CONTAINER_WORKDIR/"
done

docker start -a "$CONTAINER_NAME"

docker cp "$CONTAINER_NAME:$CONTAINER_WORKDIR/target/release/refbox" "$(dirname "$0")/output/$TARGET_TRIPLE/"
docker cp "$CONTAINER_NAME:$CONTAINER_WORKDIR/target/$TARGET_TRIPLE/release/refbox" "$(dirname "$0")/output/"
docker cp "$CONTAINER_NAME:$CONTAINER_WORKDIR/target/$TARGET_TRIPLE/release/overlay" "$(dirname "$0")/output/"

docker rm "$CONTAINER_NAME"
