RUST_VERSION="1.75"

if [ -z $1 ]; then
    TARGET_TRIPLE="aarch64-unknown-linux-gnu"
    IMAGE_NAME="linux-rust-$RUST_VERSION-xc-aarch64"
elif [ $1 = '--aarch64' ]; then
    TARGET_TRIPLE="aarch64-unknown-linux-gnu"
    IMAGE_NAME="linux-rust-$RUST_VERSION-xc-aarch64"
elif [ $1 = '--armv7' ]; then
    TARGET_TRIPLE="armv7-unknown-linux-gnueabihf"
    IMAGE_NAME="linux-rust-$RUST_VERSION-xc-armv7"
else
    echo "USAGE: $0 [<--armv7>|<--aarch64> (default: aarch64)]"
    exit 1
fi

CONTAINER_CMD="cargo build --bin refbox --target $TARGET_TRIPLE --release"
CONTAINER_WORKDIR="/workdir/uwh-refbox-rs"

CONTAINER_NAME="$(docker create -t -w "$CONTAINER_WORKDIR/" $IMAGE_NAME /bin/bash -c "$CONTAINER_CMD")"

BASE_DIR="$(dirname "$0")/../.."

for file in $(ls "$BASE_DIR" | grep -v target | grep -v xc); do
    docker cp "$BASE_DIR/$file" "$CONTAINER_NAME:$CONTAINER_WORKDIR/"
done

docker start -a "$CONTAINER_NAME"

docker cp "$CONTAINER_NAME:$CONTAINER_WORKDIR/target/$TARGET_TRIPLE/release/refbox" "$(dirname "$0")/output/$TARGET_TRIPLE/"

docker rm "$CONTAINER_NAME"
