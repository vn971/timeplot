#!/bin/bash -euET
{

# check code and update Cargo.lock:
cargo check --target i686-pc-windows-gnu
cargo check --target i686-unknown-linux-musl
cargo check --target i686-apple-darwin

exit
}
