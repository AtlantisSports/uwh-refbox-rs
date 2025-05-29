# vim:set ft=dockerfile:

FROM cimg/rust:1.87

RUN rustup component add clippy && \
    cargo install cargo-audit && \
    sudo apt update && \
    sudo apt install libsdl2-dev libfreetype-dev libexpat1-dev
