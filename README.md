# Introduction

The main software component here is the [`uwh-refbox`](uwh-refbox) crate. The other crates are support crates that are also used by other binaries, not included here.

# Running the Binary

On Windows and Mac the app can be run by downloading the latest relase from GitHub and following the bundled instructions.

If you want to change the size of the simulated panels, you will need to run via the command line:

## Windows

1. Open `PowerShell` (to open `PowerShell`, start by typing "PowerShell" into the search bar by the windows icon, then clicking the app)
2. Drag the `.exe` from `File Explorer` into the `PowerShell` window (this will insert the location of the `.exe` into the command line)
3. Add the following to the command line: `-s N` (with a space before `-s`) where `N` is any positive decimal number (`4` and `4.0` are both acceptable). `N` sets the size of the panels, the default value is `4`
4. Confirm that the command line now looks something like this: `'<PATH TO EXE>' -s 5.5`
5. Press `enter` to start the program

# Running From Source

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