RUST_VERSION="1.63"

docker build -f Dockerfile-aarch64 -t "linux-rust-$RUST_VERSION-xc-aarch64" .
docker build -f Dockerfile-armv7 -t "linux-rust-$RUST_VERSION-xc-armv7" .
