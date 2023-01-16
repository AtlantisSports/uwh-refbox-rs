# Introduction

The main software component here is the [`refbox`](refbox) crate. The other crates are support crates that are also used by other binaries, not included here.

# Running the Binary

On Windows and Mac the app can be run by downloading the latest relase from GitHub and following the bundled instructions.

## Logging

The app will log all events to a folder called `uwh-refbox-logs`, which will be placed in the appropriate system folder, selected by the [`directories` crate](https://crates.io/crates/directories)'s definition of [`data_local_dir`](https://docs.rs/directories/4.0.1/directories/struct.BaseDirs.html#method.data_local_dir). The locations will be:

| Platform | Value                                                | Example                                                  |
| -------- | ---------------------------------------------------- | -------------------------------------------------------- |
| Linux    | $XDG_DATA_HOME or $HOME/.local/share/uwh-refbox-logs | /home/alice/.local/share/uwh-refbox-logs                 |
| macOS    | $HOME/Library/Application Support/uwh-refbox-logs    | /Users/Alice/Library/Application Support/uwh-refbox-logs |
| Windows  | {FOLDERID_LocalAppData}\uwh-refbox-logs              | C:\Users\Alice\AppData\Local\uwh-refbox-logs             |

# Running From Source

1. You will need to [Install Rust](https://rustup.rs/)
2. Ensure that you have the following libraries installed: 
   - OpenSSL (`libssl-dev` package in `apt`)
   - pkg-config (`pkg-config` package in `apt`)
   - Alsa (`libasound2-dev` package in `apt`)
3. Go to the [`refbox`](refbox) folder and `cargo run`
4. Call the binary with the `-h` or `--help` flags to get the usage

# Packaging

There are provisions for cross compiling to windows and linux via Docker in the [xc](xc) folder. If you are building on a Mac, you can also bundle the build into a `.app` with `cargo bundle --release` (you will need to `cargo install cargo-bundle` first).

# Contributing

Contributions are welcome, just open a PR with your changes. All PRs must pass all tests, must have no `clippy` warnings, and must pass `cargo audit` before being merged.
