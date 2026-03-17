PLIST_PATH="/Library/LaunchDaemons/com.pigeons.daemon.plist"

launchctl unload "$PLIST_PATH"
rm "$PLIST_PATH"
rm /usr/local/bin/pigeons
