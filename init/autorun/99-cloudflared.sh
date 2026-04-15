#!/bin/sh

mount -o remount,rw /

# Install and start board service
cp /init/cloudflared/cloudflared.service /etc/systemd/system/

systemctl daemon-reload
systemctl enable cloudflared.service

mount -o remount,ro /

systemctl start cloudflared.service
