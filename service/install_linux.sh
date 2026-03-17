if ! /usr/bin/nc -z 127.0.0.1 [SSHPORT] 2>/dev/null && ! nc -z 127.0.0.1 [SSHPORT] 2>/dev/null; then
    echo "Warning: no sshd detected on port [SSHPORT]. Pigeons won't be able to deliver connections."
    echo "  Make sure sshd is running before sending pigeons to this roost."
    echo ""
fi

echo "[Unit]
Description=pigeons

[Service]
Type=simple
WorkingDirectory=~
ExecStart=/bin/bash -c 'pigeons home -p --ssh-port [SSHPORT][RELAYARGS]'
Restart=on-failure
RestartSec=3s

[Install]
WantedBy=multi-user.target" > /etc/systemd/system/pigeons.service

cp [BINARYPATH] /usr/local/bin/pigeons

systemctl is-active pigeons.service
if [ $? -eq 0 ]; then
    exit 0
else
    systemctl enable pigeons.service
    systemctl start pigeons.service
fi
