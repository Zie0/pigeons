
systemctl stop pigeons.service
systemctl disable pigeons.service
rm /etc/systemd/system/pigeons.service
rm /usr/local/bin/pigeons
systemctl daemon-reload
