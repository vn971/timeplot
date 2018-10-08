#!/bin/bash -euET
{

err_exit() {
	>&2 printf '%s\n' "$*"
	exit 1
}

cargo check  # update Cargo.lock

if ! test -z "$(git status --porcelain)"; then # no uncommited local changes
  err_exit "error: uncommitted changes"
fi

cargo build --target=i686-unknown-linux-musl --release
upx target/i686-unknown-linux-musl/release/timeplot

version=$(cat Cargo.toml | head | grep version | sed 's/.*"\(.*\)"/\1/')
git tag -m "release" "$version"

exit
}
