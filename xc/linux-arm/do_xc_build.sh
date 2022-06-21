CONTAINER_CMD="cargo build --bin uwh-refbox --target aarch64-unknown-linux-gnu --release"
CONTAINER_WORKDIR="/workdir/uwh-refbox-rs"

CONTAINER_NAME="$(docker create -t -w "$CONTAINER_WORKDIR/" linux-rust-1.61-xc /bin/bash -c "$CONTAINER_CMD")"

BASE_DIR="$(dirname "$0")/../.."

for file in $(ls "$BASE_DIR" | grep -v target | grep -v xc); do
    docker cp "$BASE_DIR/$file" "$CONTAINER_NAME:$CONTAINER_WORKDIR/"
done

docker start -a "$CONTAINER_NAME"

docker cp "$CONTAINER_NAME:$CONTAINER_WORKDIR/target/aarch64-unknown-linux-gnu/release/uwh-refbox" "$(dirname "$0")/output/"

docker rm "$CONTAINER_NAME"
