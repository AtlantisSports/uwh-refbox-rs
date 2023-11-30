set -x

TARGET_TRIPLE="aarch64-unknown-linux-gnu"
IMAGE_NAME="rpi-xc-bullseye-aarch64"

CARGO_ARGS="build --bin refbox --target $TARGET_TRIPLE --release"
CONTAINER_WORKDIR="/workdir/uwh-refbox-rs"

CONTAINER_NAME="$(docker create -t -w "$CONTAINER_WORKDIR/" $IMAGE_NAME /root/.cargo/bin/cargo $CARGO_ARGS)"

BASE_DIR="$(dirname "$0")/../.."

for file in $(ls "$BASE_DIR" | grep -v target | grep -v xc); do
    docker cp "$BASE_DIR/$file" "$CONTAINER_NAME:$CONTAINER_WORKDIR/"
done

docker start -a "$CONTAINER_NAME"

docker cp "$CONTAINER_NAME:$CONTAINER_WORKDIR/target/release/refbox" "$(dirname "$0")/output/$TARGET_TRIPLE/"
docker cp "$CONTAINER_NAME:$CONTAINER_WORKDIR/target/$TARGET_TRIPLE/release/refbox" "$(dirname "$0")/output/"

docker rm "$CONTAINER_NAME"
