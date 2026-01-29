#!/bin/bash

# --- CONFIGURATION ---
SOURCE_BINARY_NAME="legion-led"
FINAL_BINARY_NAME="legion-led"
INSTALL_PATH="/usr/local/bin/$FINAL_BINARY_NAME"
SERVICE_NAME="legion-led.service"
MODULE_CONF="/etc/modules-load.d/legion-led.conf"
MODPROBE_CONF="/etc/modprobe.d/legion-led.conf"

# 1. Root Check
if [ "$EUID" -ne 0 ]; then
    echo "ERROR: Please run with sudo!"
    exit 1
fi

# --- MODE: UNINSTALL ---
if [ "$1" == "uninstall" ]; then
    echo ">>> [UNINSTALL] Stopping and removing service..."
    systemctl stop $SERVICE_NAME 2>/dev/null
    systemctl disable $SERVICE_NAME 2>/dev/null
    rm -f /etc/systemd/system/$SERVICE_NAME
    systemctl daemon-reload

    echo ">>> [UNINSTALL] Removing module configs..."
    rm -f "$MODULE_CONF"
    rm -f "$MODPROBE_CONF"

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
if [ -f "./$SOURCE_BINARY_NAME" ]; then
    BINARY_TO_INSTALL="./$SOURCE_BINARY_NAME"
elif [ -f "./target/release/$SOURCE_BINARY_NAME" ]; then
    BINARY_TO_INSTALL="./target/release/$SOURCE_BINARY_NAME"
else
    echo "ERROR: Binary file '$SOURCE_BINARY_NAME' not found."
    echo "Did you run 'cargo build --release' or download the binary?"
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

echo ">>> [4/5] Configuring Kernel Modules & Service..."

# Ensure ec_sys module loads on boot with write support
echo "ec_sys" > "$MODULE_CONF"
echo "options ec_sys write_support=1" > "$MODPROBE_CONF"

# Load it immediately for this session
modprobe ec_sys write_support=1 2>/dev/null

# Create a clean, unrestricted service file
cat <<EOF > /etc/systemd/system/$SERVICE_NAME
[Unit]
Description=Legion Go S LED Sleep Controller
After=multi-user.target

[Service]
Type=simple
ExecStart=$INSTALL_PATH daemon
Restart=always
RestartSec=5
User=root
# No hardening - full hardware access required

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