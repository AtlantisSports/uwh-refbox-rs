# vim:set ft=dockerfile:

#FROM cimg/rust:1.66

# Begin workaround to use rust 1.66 because circleci hasn't released docker images yet
FROM cimg/base:2022.11

ENV RUST_VERSION=1.66.1 \
    PATH=/home/circleci/.cargo/bin:$PATH

RUN curl -O https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init && \
    chmod +x rustup-init && \
    ./rustup-init -y --no-modify-path --default-toolchain $RUST_VERSION && \
    rm rustup-init && \
    rustc --version && \
    cargo --version

RUN rustup component add rustfmt
# End Workaround

RUN rustup component add clippy && \
    cargo install cargo-audit && \
    sudo apt update && \
    sudo apt install libsdl2-dev libfreetype-dev libexpat1-dev
