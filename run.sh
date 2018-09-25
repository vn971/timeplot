#!/bin/bash -euET
{

export DISPLAY=":0"
export DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/1000/bus"

"$(dirname -- "$(realpath -- "$0")")"/target/release/screenshooter-rs

exit
}
