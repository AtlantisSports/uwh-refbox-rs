CONTAINER_CMD="cargo build --bin refbox --target x86_64-pc-windows-msvc --release"
CONTAINER_WORKDIR="/workdir/uwh-refbox-rs"

CONTAINER_NAME="$(docker create -t -w "$CONTAINER_WORKDIR/" windows-rust-1.66-xc /bin/bash -c "$CONTAINER_CMD")"

BASE_DIR="$(dirname "$0")/../.."

for file in $(ls "$BASE_DIR" | grep -v target | grep -v xc); do
    docker cp "$BASE_DIR/$file" "$CONTAINER_NAME:$CONTAINER_WORKDIR/"
done

docker start -a "$CONTAINER_NAME"

docker cp "$CONTAINER_NAME:$CONTAINER_WORKDIR/target/x86_64-pc-windows-msvc/release/refbox.exe" "$(dirname "$0")/output/"

docker rm "$CONTAINER_NAME"
