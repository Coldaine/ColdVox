#!/bin/bash
set -euo pipefail

# This script is for XFCE specific setup.

# Configure auto-login for the testuser
# We will use lightdm, the default display manager for XFCE.
cat <<EOF > /etc/lightdm/lightdm.conf.d/10-autologin.conf
[Seat:*]
autologin-guest=false
autologin-user=testuser
autologin-user-timeout=0
EOF

# Ensure the correct session is used
echo "xfce" > /home/testuser/.dmrc
chown testuser:testuser /home/testuser/.dmrc