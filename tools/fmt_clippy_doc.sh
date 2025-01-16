#!/bin/sh

echo "Running cargo fmt --check"
cargo fmt --all --check

echo ""
echo "Running cargo clippy"
cargo clippy --all-targets --all-features --no-deps -- -W warnings -D warnings

echo ""
echo "Running cargo doc"
cargo doc --no-deps --all-features
