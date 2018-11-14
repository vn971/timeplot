#!/bin/bash -euET
{
set -o pipefail

err_exit() {
	>&2 printf '%s\n' "$*"
	exit 1
}

# check code and update Cargo.lock:
cargo check --target i686-pc-windows-gnu
cargo check --target i686-unknown-linux-musl
cargo check --target i686-apple-darwin

if ! test -z "$(git status --porcelain)"; then # no uncommited local changes
  err_exit "error: uncommitted changes"
fi

cargo build --release
cargo build --target=i686-unknown-linux-musl --release
upx --ultra-brute target/i686-unknown-linux-musl/release/timeplot
cp target/i686-unknown-linux-musl/release/timeplot .vasya-personal/tpl/

cargo publish

tag=$(cat Cargo.toml | head | grep version | sed 's/.*"\(.*\)"/\1/')
git tag -m "release" "$tag"

exit
}
