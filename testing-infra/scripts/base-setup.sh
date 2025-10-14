#!/bin/bash
set -euo pipefail

# Create a test user
useradd -m -G wheel -s /bin/bash testuser
echo "testuser:password" | chpasswd
echo "testuser ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

# Install common dependencies
dnf install -y \
  git \
  openssh-server \
  qemu-guest-agent \
  wl-clipboard \
  ydotool \
  dbus-x11

# Enable SSH
systemctl enable sshd

# Setup SSH for the testuser
mkdir -p /home/testuser/.ssh
touch /home/testuser/.ssh/authorized_keys
# This is a placeholder for the CI SSH key. In a real scenario, we would
# inject the public key here. For this example, we'll rely on password auth.
# For a production setup, we would use Packer's ssh_authorized_key_file feature.
chown -R testuser:testuser /home/testuser/.ssh
chmod 700 /home/testuser/.ssh
chmod 600 /home/testuser/.ssh/authorized_keys

# Enable and start the qemu-guest-agent
systemctl enable qemu-guest-agent
systemctl start qemu-guest-agent