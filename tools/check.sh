#!/bin/bash
{
set -euETo pipefail
set -x

cargo test
cargo clippy --all-targets --all-features -- -D warnings

cargo check --target i686-pc-windows-gnu
cargo check --target i686-unknown-linux-musl
cargo check --target x86_64-apple-darwin

exit
}
