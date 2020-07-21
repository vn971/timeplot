#!/bin/bash -euET
{
# script to release the project, probably has no use to anybody else except for reference.

set -o pipefail

rustup update
cargo upgrade
cargo update
cargo fmt --all --
if ! test -z "$(git status --porcelain)"; then
  >&2 printf '%s\n' "error: uncommitted changes"
  exit 1
fi

cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo check --target i686-pc-windows-gnu
cargo check --target x86_64-apple-darwin
cargo build --release
cargo build --target=i686-unknown-linux-musl --release

upx --ultra-brute target/i686-unknown-linux-musl/release/timeplot
cp target/i686-unknown-linux-musl/release/timeplot .vasya-personal/tpl/

cargo publish

tag=$(cat Cargo.toml | grep -m1 version | sed 's/.*"\(.*\)"/\1/')
git tag -m "release" "$tag"

exit
}
