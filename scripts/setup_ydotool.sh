#!/usr/bin/env bash
# Dedicated ydotool setup helper for ColdVox
# Configures a non-root ydotoold user service, socket directory, and environment.

set -euo pipefail

log() {
    echo "[$(date +%H:%M:%S)] $*"
}

err() {
    echo "[$(date +%H:%M:%S)] ERROR: $*" >&2
}

# Detect package manager (reuse ordering from setup_text_injection.sh)
detect_pkg_mgr() {
    if command -v dnf >/dev/null 2>&1; then
        PKG_MANAGER="dnf"
        PKG_INSTALL_CMD=(sudo dnf install -y)
    elif command -v pacman >/dev/null 2>&1; then
        PKG_MANAGER="pacman"
        PKG_INSTALL_CMD=(sudo pacman -S --noconfirm)
    elif command -v apt >/dev/null 2>&1; then
        PKG_MANAGER="apt"
        PKG_INSTALL_CMD=(sudo apt install -y)
    else
        err "Unsupported package manager. Install ydotool manually."
        exit 1
    fi
}

ensure_package() {
    local binary="$1"
    local package="${2:-$1}"

    if command -v "$binary" >/dev/null 2>&1; then
        log "✓ $binary already installed"
        return
    fi

    if [[ -z "${PKG_MANAGER:-}" ]]; then
        detect_pkg_mgr
        log "Detected package manager: $PKG_MANAGER"
    fi

    log "Installing $package..."
    if ! "${PKG_INSTALL_CMD[@]}" "$package"; then
        err "Failed to install $package"
        exit 1
    fi
}

ensure_socket_dir() {
    local dir="$HOME/.ydotool"
    if [[ ! -d "$dir" ]]; then
        mkdir -p "$dir"
        chmod 700 "$dir"
        log "✓ Created $dir"
    else
        chmod 700 "$dir" || true
        log "✓ Ensured $dir exists with 700 permissions"
    fi
}

write_user_service() {
    local ydotoold_path="$1"
    local systemd_dir="$HOME/.config/systemd/user"
    local service_path="$systemd_dir/ydotool.service"

    mkdir -p "$systemd_dir"

    local desired_unit="[Unit]
Description=ydotoold input daemon (ColdVox recommended setup)
After=graphical-session.target

[Service]
Type=simple
ExecStart=${ydotoold_path} --socket-path %h/.ydotool/socket
Restart=on-failure

[Install]
WantedBy=default.target
"

    if [[ -f "$service_path" ]]; then
        if diff -q <(printf "%s" "$desired_unit") "$service_path" >/dev/null 2>&1; then
            log "✓ Existing ydotool user service matches desired configuration"
            return
        fi
        log "Updating existing ydotool user service to match desired configuration"
    else
        log "Creating ydotool user service at $service_path"
    fi

    printf "%s" "$desired_unit" >"$service_path"
}

enable_user_service() {
    if ! systemctl --user daemon-reload >/dev/null 2>&1; then
        err "systemctl --user not available. Ensure you have a user systemd session (e.g. log into a graphical session) and rerun."
        return 1
    fi

    systemctl --user daemon-reload || true

    if ! systemctl --user is-enabled ydotool.service >/dev/null 2>&1; then
        if systemctl --user enable ydotool.service >/dev/null 2>&1; then
            log "✓ Enabled ydotool user service"
        else
            err "Failed to enable ydotool user service"
            return 1
        fi
    else
        log "✓ ydotool user service already enabled"
    fi

    if ! systemctl --user is-active ydotool.service >/dev/null 2>&1; then
        if systemctl --user start ydotool.service >/dev/null 2>&1; then
            log "✓ Started ydotool user service"
        else
            err "Failed to start ydotool user service; inspect with: systemctl --user status ydotool.service"
            return 1
        fi
    else
        log "✓ ydotool user service already running"
    fi
}

write_environment_dropin() {
    local env_dir="$HOME/.config/environment.d"
    local env_file="$env_dir/coldvox-ydotool.conf"
    local desired_line="YDOTOOL_SOCKET=$HOME/.ydotool/socket"

    mkdir -p "$env_dir"

    if [[ -f "$env_file" ]]; then
        if grep -q "^YDOTOOL_SOCKET=$HOME/.ydotool/socket$" "$env_file"; then
            log "✓ YDOTOOL_SOCKET already configured in $env_file"
            return
        fi
        if grep -q "^YDOTOOL_SOCKET=" "$env_file"; then
            sed -i "s|^YDOTOOL_SOCKET=.*|$desired_line|" "$env_file"
            log "✓ Updated YDOTOOL_SOCKET in $env_file"
            return
        fi
    fi

    printf "%s\n" "$desired_line" >>"$env_file"
    log "✓ Added YDOTOOL_SOCKET to $env_file"
}

summarise_status() {
    echo
    log "Summary:"

    if command -v ydotool >/dev/null 2>&1; then
        log "  - ydotool binary: present"
    else
        log "  - ydotool binary: MISSING"
    fi

    if command -v ydotoold >/dev/null 2>&1; then
        log "  - ydotoold binary: present"
    else
        log "  - ydotoold binary: MISSING"
    fi

    if [[ -S "$HOME/.ydotool/socket" ]]; then
        log "  - socket: $HOME/.ydotool/socket available"
    else
        log "  - socket: not detected yet (service may need restart or login)"
    fi

    if systemctl --user is-active ydotool.service >/dev/null 2>&1; then
        log "  - user service: active"
    else
        log "  - user service: inactive"
    fi

    if groups | grep -q '\binput\b'; then
        log "  - input group: user in group"
    else
        log "  - input group: user NOT in group (run: sudo usermod -a -G input \"$USER\")"
    fi
}

main() {
    log "== ColdVox ydotool setup =="

    ensure_package ydotool
    ensure_package ydotoold ydotool

    ensure_socket_dir

    local ydotoold_path
    ydotoold_path=$(command -v ydotoold || true)
    if [[ -z "$ydotoold_path" ]]; then
        err "ydotoold not found in PATH even after install. Aborting."
        exit 1
    fi

    write_user_service "$ydotoold_path"

    if ! enable_user_service; then
        log "ℹ️  Could not manage user service automatically. Ensure user systemd is active and rerun."
    fi

    write_environment_dropin

    if [[ -z "${YDOTOOL_SOCKET:-}" ]]; then
        export YDOTOOL_SOCKET="$HOME/.ydotool/socket"
        log "✓ Exported YDOTOOL_SOCKET for current shell"
    fi

    summarise_status
}

main "$@"
