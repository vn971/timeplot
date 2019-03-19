#!/bin/bash -euET
{
set -o pipefail

cargo upgrade
cargo update

# check code and update Cargo.lock:
cargo check --target i686-pc-windows-gnu
cargo check --target i686-unknown-linux-musl
cargo check --target i686-apple-darwin

if ! test -z "$(git status --porcelain)"; then
	>&2 printf '%s\n' "error: uncommitted changes"
	exit 1
fi

cargo build --release
cargo build --target=i686-unknown-linux-musl --release
upx --ultra-brute target/i686-unknown-linux-musl/release/timeplot
cp target/i686-unknown-linux-musl/release/timeplot .vasya-personal/tpl/

cargo publish

tag=$(cat Cargo.toml | grep -m1 version | sed 's/.*"\(.*\)"/\1/')
git tag -m "release" "$tag"

exit
}
