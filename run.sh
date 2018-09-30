#!/bin/bash -euET
{

export DISPLAY="${DISPLAY:-:0}"
export PATH='/home/vasya/bin:/home/vasya/bin:/usr/local/bin:/usr/bin:/bin:/usr/local/sbin:/usr/lib/jvm/default/bin'
export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=/run/user/$(id -u)/bus}"

cd "$(dirname -- "$(realpath -- "$0")")"
RUST_BACKTRACE=1 cargo run -- "$@"
#exec "$(dirname -- "$(realpath -- "$0")")"/target/x86_64-unknown-linux-musl/release/screenshooter-rs

exit
}
