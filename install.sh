#!/bin/bash

# --- CONFIGURATION ---
# Nazwa pliku wygenerowanego przez cargo (w target/release/)
SOURCE_BINARY_NAME="legion-led"
# Nazwa pod jaką zainstalujemy program w systemie (krótsza)
FINAL_BINARY_NAME="legion-led"
INSTALL_PATH="/usr/local/bin/$FINAL_BINARY_NAME"
SERVICE_NAME="legion-led.service"

# 1. Root Check
if [ "$EUID" -ne 0 ]; then
    echo "ERROR: Please run with sudo!"
    exit 1
fi

# --- MODE: UNINSTALL ---
# If the user runs "./install.sh uninstall", we go here
if [ "$1" == "uninstall" ]; then
    echo ">>> [UNINSTALL] Stopping and removing service..."
    systemctl stop $SERVICE_NAME 2>/dev/null
    systemctl disable $SERVICE_NAME 2>/dev/null
    rm -f /etc/systemd/system/$SERVICE_NAME
    systemctl daemon-reload

    echo ">>> [UNINSTALL] Unlocking filesystem..."
    steamos-readonly disable 2>/dev/null

    echo ">>> [UNINSTALL] Removing binary..."
    rm -f "$INSTALL_PATH"

    echo ">>> [UNINSTALL] Securing filesystem..."
    steamos-readonly enable 2>/dev/null

    echo ">>> SUCCESS. Uninstalled completely."
    exit 0
fi

# --- MODE: INSTALL (Default) ---

# 2. Find the binary
# Check current directory first, then target/release
if [ -f "./$SOURCE_BINARY_NAME" ]; then
    BINARY_TO_INSTALL="./$SOURCE_BINARY_NAME"
elif [ -f "./target/release/$SOURCE_BINARY_NAME" ]; then
    BINARY_TO_INSTALL="./target/release/$SOURCE_BINARY_NAME"
else
    echo "ERROR: Binary file '$SOURCE_BINARY_NAME' not found."
    echo "Did you run 'cargo build --release'?"
    exit 1
fi

echo "Found binary at: $BINARY_TO_INSTALL"

echo ">>> [1/5] Stopping running service..."
systemctl stop $SERVICE_NAME 2>/dev/null

echo ">>> [2/5] Unlocking filesystem (SteamOS)..."
steamos-readonly disable 2>/dev/null

echo ">>> [3/5] Installing binary to $INSTALL_PATH..."
cp -f "$BINARY_TO_INSTALL" "$INSTALL_PATH"
chmod +x "$INSTALL_PATH"
chown root:root "$INSTALL_PATH"

echo ">>> [4/5] Creating Hardened Systemd Service..."
cat <<EOF > /etc/systemd/system/$SERVICE_NAME
[Unit]
Description=Legion Go S LED Sleep Controller
After=multi-user.target

[Service]
Type=simple
# Load the kernel module before starting the daemon
ExecStartPre=/usr/sbin/modprobe ec_sys write_support=1
# Run the binary with the 'daemon' subcommand
ExecStart=$INSTALL_PATH daemon
Restart=always
RestartSec=5
User=root

# --- PERMISSIONS ---
# Crucial: Allow writing to EC debugfs
ReadWritePaths=/sys/kernel/debug/ec

# --- SECURITY HARDENING ---
ProtectHome=true
ProtectSystem=full
PrivateTmp=true
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
EOF

echo ">>> [5/5] Activating service..."
systemctl daemon-reload
systemctl enable $SERVICE_NAME
systemctl restart $SERVICE_NAME

echo ">>> Securing filesystem..."
steamos-readonly enable 2>/dev/null

echo ">>> SUCCESS. Legion LED service installed."
echo "    Status check: systemctl status $SERVICE_NAME"
echo "    Manual control: sudo legion-led on / off"