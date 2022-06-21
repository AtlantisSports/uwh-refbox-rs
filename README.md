# Introduction

The main software component here is the [`uwh-refbox`](uwh-refbox) crate. The other crates are support crates that are also used by other binaries, not included here.

# Running

1. You will need to [Install Rust](https://rustup.rs/)
2. Ensure that you have the following libraries installed: 
   - OpenSSL (`libssl-dev` package in `apt`)
   - pkg-config (`pkg-config` package in `apt`)
   - Alsa (`libasound2-dev` package in `apt`)
3. Go to the [`uwh-refbox`](uwh-refbox) folder and `cargo run`
4. Call the binary with the `-h` or `--help` flags to get the usage

# Packaging

There are provisions for cross compiling to windows and linux via Docker in the [xc](xc) folder. If you are building on a Mac, you can also bundle the build into a `.app` with `cargo bundle --release` (you will need to `cargo install cargo-bundle` first).

# Contributing

Contributions are welcome, just open a PR with your changes. All PRs must pass all tests, must have no `clippy` warnings, and must pass `cargo audit` before being merged.