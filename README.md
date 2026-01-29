# Legion Go S LED Control
![Legion Go S](https://img.shields.io/badge/Device-Legion_Go_S-blue)
![Language](https://img.shields.io/badge/Written_in-Rust-orange)
![Platform](https://img.shields.io/badge/Platform-Linux%20%2F%20SteamOS-green)

A lightweight, efficient daemon written in **Rust** designed for the **Lenovo Legion Go S** running Linux (SteamOS).

It solves the common issue of the **Power LED breathing (pulsing)** while the device is in sleep mode, which can be distracting at night.

## Features

* **Automatic Sleep Management:** Automatically disables the LED breathing effect (sets it to static/off) instantly before the device sleeps.
* **Automatic Wake Management:** Restores the default LED behavior (breathing/system controlled) as soon as the device wakes up.
* **Zero Polling:** Uses D-Bus signals (`PrepareForSleep`) instead of constantly checking system status, ensuring **0% CPU usage** and no battery drain.
* **Safe EC Modification:** Uses a "Read-Modify-Write" strategy to flip only the specific bit responsible for the LED mode, leaving other EC settings (thermal, fans) untouched.
* **Manual Control:** Includes a CLI tool to manually toggle the LED mode.

>[!WARNING]  
>This software modifies hardware registers on the Embedded Controller. While it has been tested on the Legion Go S and uses safe bitwise operations, use it at your own risk. The author is not responsible for any potential damage or instability.

## Quick Install (One-Liner)

You can install the latest release directly via terminal. This script will download the binary, install the systemd service, and set up permissions.

~~~bash
mkdir -p /tmp/lgs-install && cd /tmp/lgs-install && curl -L -o legion-led https://github.com/Veizeczek/legion-go-s-led-control/releases/download/v1.0/legion-led && curl -L -o install.sh https://raw.githubusercontent.com/Veizeczek/legion-go-s-led-control/main/install.sh && chmod +x install.sh && sudo ./install.sh && cd ~ && rm -rf /tmp/lgs-install
~~~

## Manual Usage
Once installed, the service runs automatically in the background. However, you can control the LED manually if needed:

Turn the LED OFF:

~~~bash
sudo legion-led off
~~~

Turn the LED ON (Default):

~~~bash
sudo legion-led on
~~~

Check service status:
~~~bash
systemctl status legion-led
~~~

## Technical Details
This daemon interacts with the Lenovo Embedded Controller (EC) via the Linux ec_sys kernel module.

EC Register: 0x10

Control Bit: 6 (0x40)

Logic:

Bit 6 = 1: Disables breathing (Static/Off) - Used for Sleep.

Bit 6 = 0: Enables breathing (Default) - Used for Wake.
