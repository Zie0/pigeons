if ! /usr/bin/nc -z 127.0.0.1 [SSHPORT] 2>/dev/null; then
    echo "Warning: no sshd detected on port [SSHPORT]. Pigeons won't be able to deliver connections."
    echo "  Enable Remote Login in System Settings to start sshd."
    echo ""
fi

PLIST_PATH="/Library/LaunchDaemons/computer.pigeons.daemon.plist"

cat > "$PLIST_PATH" <<'PLIST_EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>computer.pigeons.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/bash</string>
        <string>-c</string>
        <string>pigeons home -p --ssh-port [SSHPORT][RELAYARGS]</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>WorkingDirectory</key>
    <string>/var/root</string>
    <key>StandardOutPath</key>
    <string>/var/log/pigeons.log</string>
    <key>StandardErrorPath</key>
    <string>/var/log/pigeons.log</string>
</dict>
</plist>
PLIST_EOF

if [ "$(realpath '[BINARYPATH]')" != "$(realpath /usr/local/bin/pigeons 2>/dev/null)" ]; then
    cp [BINARYPATH] /usr/local/bin/pigeons
fi

launchctl list | grep -q computer.pigeons.daemon
if [ $? -eq 0 ]; then
    # Already running; bootout and re-bootstrap to pick up any config changes
    launchctl bootout system/computer.pigeons.daemon 2>/dev/null || launchctl unload "$PLIST_PATH" 2>/dev/null
fi

# bootstrap registers AND starts the service (modern launchctl)
launchctl bootstrap system "$PLIST_PATH" 2>/dev/null || launchctl load "$PLIST_PATH"
