set -ex

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
        <string>[BINARYPATH] roost --ssh-port [SSHPORT][RELAYARGS]</string>
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

if launchctl list 2>/dev/null | grep -q computer.pigeons.daemon; then
    # Already running; bootout and re-bootstrap to pick up any config changes
    launchctl bootout system/computer.pigeons.daemon 2>/dev/null || launchctl unload "$PLIST_PATH" 2>/dev/null
fi

# bootstrap registers AND starts the service (modern launchctl)
launchctl bootstrap system "$PLIST_PATH" 2>/dev/null || launchctl load "$PLIST_PATH"

# Wait for the service to generate keys, then copy the public key
# to a world-readable location so unprivileged users can run 'pigeons status'
sleep 2
mkdir -p /etc/pigeons
cp /var/root/.ssh/pigeons_ed25519.pub /etc/pigeons/endpoint_id 2>/dev/null || true
chmod 644 /etc/pigeons/endpoint_id 2>/dev/null || true
