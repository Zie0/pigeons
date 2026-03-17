PLIST_PATH="/Library/LaunchDaemons/computer.pigeons.daemon.plist"

# bootout stops and deregisters the service (modern launchctl)
launchctl bootout system/computer.pigeons.daemon 2>/dev/null || launchctl unload "$PLIST_PATH"
rm "$PLIST_PATH"
rm /usr/local/bin/pigeons
