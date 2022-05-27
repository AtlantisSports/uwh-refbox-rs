# Delete loose files in the debug directory
find ./target/debug -maxdepth 1 -type f -delete

# This is just more metadata
rm -f  ./target/.rustc_info.json