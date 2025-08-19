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

1. If you are building on Windows, ensure that you have the [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) installed, with the "Desktop development with C++" workload selected.
2. [Install Rust](https://rustup.rs/)
3. Ensure that you have the following libraries installed: 
   - OpenSSL (`libssl-dev` package in `apt`)
   - pkg-config (`pkg-config` package in `apt`)
   - Alsa (`libasound2-dev` package in `apt`)
4. Go to the [`refbox`](refbox) folder and `cargo run`
5. Call the binary with the `-h` or `--help` flags to get the usage

# Cross Compiling

Builds for cargo targets other than the host target can be compiled using the `cross` tool:

1. Install [Docker](https://www.docker.com/products/docker-desktop/) and ensure it is running
2. Install `cross` using `cargo install cross`
3. Compile the binary using `cross build --all --release --target <target>`, where `<target>` is the target you want to compile for:
   - `aarch64-unknown-linux-gnu` for the Raspberry Pi 4 or 5
   - `x86_64-pc-windows-gnu` for Windows
   - `aarch64-apple-darwin` for newer Arm based Macs (M series chips)
   - `x86_64-apple-darwin` for Intel based Macs
4. The resulting binaries will be in the `target/<target>/release` folder.

# Documentation

Comprehensive design documentation is available in the [`docs/`](docs) directory:

- **Design Documents**: [`docs/design/`](docs/design) contains detailed design specifications
- **Documentation Scripts**: [`docs/scripts/`](docs/scripts) contains tools for generating HTML documentation

To view the formatted documentation, open [`docs/design/Atlantis UWH-REFBOX-RS Detailed Design.html`](docs/design/Atlantis%20UWH-REFBOX-RS%20Detailed%20Design.html) in your web browser.

# Testing

## Integration Tests

Integration tests and test scripts are located in [`integration-tests/`](integration-tests):

- **Test Scripts**: [`integration-tests/scripts/`](integration-tests/scripts) contains scripts for running font sizing tests and demonstrations
- **Test Suites**: [`integration-tests/src/`](integration-tests/src) contains the test implementation code

Run test scripts from the project root:
```bash
# Font sizing demonstration
integration-tests\scripts\run_font_demo.bat

# Font sizing tests
integration-tests\scripts\run_font_tests.bat
```

# Contributing

Contributions are welcome, just open a PR with your changes. All PRs must pass all tests, must have no `clippy` warnings, and must pass `cargo audit` before being merged.
