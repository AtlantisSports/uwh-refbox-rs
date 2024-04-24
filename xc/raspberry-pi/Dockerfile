FROM debian:bullseye AS base

RUN apt update && apt install --no-install-recommends -y libssl-dev pkg-config libasound2-dev gcc-aarch64-linux-gnu curl ca-certificates libc6-dev-arm64-cross

WORKDIR /workdir

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y -t 'aarch64-unknown-linux-gnu' && \
    . $HOME/.cargo/env && \
    cargo install cargo-chef


FROM base AS planner
COPY . .
RUN . $HOME/.cargo/env && cargo chef prepare --recipe-path recipe.json


FROM base AS builder
COPY --from=planner /workdir/recipe.json recipe.json
RUN . $HOME/.cargo/env && cargo chef cook --release --recipe-path recipe.json --target aarch64-unknown-linux-gnu
