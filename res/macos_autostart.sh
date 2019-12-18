#!/usr/bin/env bash

if [[ $EUID -ne 0 ]]; then
	echo "This script must be run as root"
	exit 1
fi

if [[ ":$PATH:" != *":$HOME/.cargo/bin:"* ]]; then
	echo "Cargo is not in your path, please fix this and try again"
	exit 1
fi

PLIST_PATH="/Library/LaunchAgents/timeplot.plist"
TIMEPLOT_PATH="$HOME/.cargo/bin/timeplot"

cat >"$PLIST_PATH" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>timeplot</string>
    <key>OnDemand</key>
    <false/>
    <key>Program</key>
    <string>${TIMEPLOT_PATH}</string>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
    <key>LaunchOnlyOnce</key>        
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/startup.stdout</string>
    <key>StandardErrorPath</key>
    <string>/tmp/startup.stderr</string>
</dict>
</plist>
EOF

launchctl load -w "$PLIST_PATH"

echo "Open Security & Privacy, click Accessibility, check Timeplot"
