#!/bin/sh

mount -o remount,rw /

echo 'Enabling SSH!'
echo "Port 22" >/etc/ssh/sshd_config
echo "PermitRootLogin yes" >>/etc/ssh/sshd_config
echo "PubkeyAuthentication yes" >>/etc/ssh/sshd_config
echo "AuthorizedKeysFile .ssh/authorized_keys" >>/etc/ssh/sshd_config

mkdir -p /root/.ssh
echo "__SSH_PUBLIC_KEY__" >/root/.ssh/authorized_keys
chmod 0700 /root/.ssh

systemctl enable --now sshd.socket

mount -o remount,ro /
