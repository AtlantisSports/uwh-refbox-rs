# We'll just use the official Rust image rather than build our own from scratch
FROM docker.io/library/rust:1.61.0-slim-bullseye

ENV KEYRINGS /usr/local/share/keyrings

RUN set -eux; \
    # Skipping all of the "recommended" cruft reduces total images size by ~300MiB
    apt-get update && apt-get install --no-install-recommends -y \
    libssl-dev \
    pkg-config \
    libasound2-dev

# Retrieve the std lib for the target
RUN rustup target add aarch64-unknown-linux-gnu

WORKDIR /workdir
