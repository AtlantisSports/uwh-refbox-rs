#!/bin/bash

# Find all .toml files excluding ./Cargo.toml that do not contain 'rust-version'
missing_rust_version=$(grep -rL 'rust-version' --include \*.toml . | grep -v './Cargo.toml')

# Check if any files are found
if [ -n "$missing_rust_version" ]; then
  echo "The following .toml files are missing 'rust-version':"
  echo "$missing_rust_version"
  exit 1
else
  echo "All .toml files contain 'rust-version'."
fi