#!/bin/sh

mount -o remount,rw /

# Install and start board service
cp /init/proxbeacon/proxbeacon.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable proxbeacon.service

mount -o remount,ro /

systemctl start proxbeacon.service
