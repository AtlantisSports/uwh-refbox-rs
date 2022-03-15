# vim:set ft=dockerfile:

FROM cimg/base:2022.01

ENV RUST_VERSION=1.59.0 \
	PATH=/home/circleci/.cargo/bin:$PATH

RUN curl -O https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init && \
	chmod +x rustup-init && \
	./rustup-init -y --no-modify-path --default-toolchain $RUST_VERSION && \
	rm rustup-init && \
	rustc --version && \
	cargo --version && \
        rustup component add rustfmt && \
        rustup component add clippy && \
        cargo install cargo-audit && \
        sudo apt update && \
        sudo apt install libgtk-3-dev libsdl2-dev
