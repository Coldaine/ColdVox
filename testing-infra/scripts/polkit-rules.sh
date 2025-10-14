#!/bin/bash
set -euo pipefail

# This script installs the Polkit rules required for unattended testing.

cat <<EOF > /etc/polkit-1/rules.d/99-coldvox-testing.rules
polkit.addRule(function(action, subject) {
    if (subject.user === "testuser") {
        // Grant all permissions to the dedicated test user.
        // This is safe as the VMs are isolated and ephemeral.
        return polkit.Result.YES;
    }
});
EOF