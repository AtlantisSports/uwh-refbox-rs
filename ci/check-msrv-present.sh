#!/bin/bash

# Find all .toml files excluding ./Cargo.toml that do not contain 'rust-version'
excluded_files=(
  './Cargo.toml'
  './Cross.toml'
  './refbox/i18n.toml'
  './wireless-remote/rust-toolchain.toml'
  './wireless-remote/.cargo/config.toml'
)

missing_rust_version=$(grep -rL 'rust-version' --include \*.toml .)
for file in "${excluded_files[@]}"; do
  missing_rust_version=$(echo "$missing_rust_version" | grep -v -F "$file")
done

# Check if any files are found
if [ -n "$missing_rust_version" ]; then
  echo "The following .toml files are missing 'rust-version':"
  echo "$missing_rust_version"
  exit 1
else
  echo "All .toml files contain 'rust-version'."
fi