# AlphaGen

AlphaGen is a small Rust application used to generate greyscale mask images based on the alpha channel of an image. It takes input image files and an output directory as command-line arguments and processes the images in parallel to generate greyscale mask images.

## Usage

```shell
alphagen input_file(s) output_directory
```

### Installation

To use AlphaGen, you need to have Rust and Cargo installed. If you don't have them installed, you can follow the official Rust installation guide: https://www.rust-lang.org/tools/install

Once Rust is installed, you can clone the project repository and navigate to the project directory:

```shell
git clone <repository-url>
cd alphagen
cargo install --path .
```
