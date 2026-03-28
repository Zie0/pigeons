
systemctl stop pigeons.service
systemctl disable pigeons.service
rm /etc/systemd/system/pigeons.service
rm -rf /etc/pigeons
systemctl daemon-reload
