#!/bin/sh

cargo fmt -- --check
cargo clippy --all-targets -- -W warnings -D warnings
cargo doc -p up-rust --no-deps
