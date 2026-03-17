if ! /usr/bin/nc -z 127.0.0.1 [SSHPORT] 2>/dev/null; then
    echo "Warning: no sshd detected on port [SSHPORT]. Pigeons won't be able to deliver connections."
    echo "  Enable Remote Login in System Settings to start sshd."
    echo ""
fi

PLIST_PATH="/Library/LaunchDaemons/com.pigeons.daemon.plist"

cat > "$PLIST_PATH" <<'PLIST_EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.pigeons.daemon</string>
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

cp [BINARYPATH] /usr/local/bin/pigeons

launchctl list | grep -q com.pigeons.daemon
if [ $? -eq 0 ]; then
    exit 0
else
    launchctl load "$PLIST_PATH"
fi
