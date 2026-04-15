#!/bin/sh

mount -o remount,rw /

ln -sf /usr/share/zoneinfo/America/New_York /etc/localtime

mount -o remount,ro /

# Disable default PM3 UI
systemctl stop nx
systemctl mask nx
systemctl stop init-abtproxy
systemctl mask init-abtproxy

/usr/bin/NxExe watchdog 0
