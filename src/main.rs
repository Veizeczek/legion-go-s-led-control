use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::process::Command;
use zbus::export::futures_util::StreamExt;
use zbus::Connection;

// --- HARDWARE CONFIGURATION ---
const EC_PATH: &str = "/sys/kernel/debug/ec/ec0/io";
const LED_OFFSET: u64 = 0x10;
const LED_BIT: u8 = 6;      // Bit 6
const LED_MASK: u8 = 1 << LED_BIT; // 0x40 (64)

// --- LOGIC CONFIGURATION ---
// Bit 1 (True) usually disables breathing (Static/Off in sleep context)
// Bit 0 (False) enables breathing (Default)
const VAL_SLEEP_MODE: bool = true;  // Set bit to 1 before sleep (disable breathing)
const VAL_WAKE_MODE: bool = false; // Set bit to 0 after wake (return to system control)

#[derive(Parser)]
#[command(name = "legion-led")]
#[command(about = "Legion Go S LED Controller & Daemon", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Starts the background process (listens for sleep/wake events)
    Daemon,
    /// Forces "OFF" mode (stops breathing/static light)
    Off,
    /// Restores "ON" mode (default breathing/system control)
    On,
}

/// Checks and loads the kernel module if not present
fn ensure_ec_access() -> Result<()> {
    if !std::path::Path::new(EC_PATH).exists() {
        println!("Loading ec_sys module...");
        let status = Command::new("modprobe")
            .arg("ec_sys")
            .arg("write_support=1")
            .status()
            .context("Failed to run modprobe")?;

        if !status.success() {
            return Err(anyhow::anyhow!("Error loading ec_sys. Run as root!"));
        }
    }
    Ok(())
}

/// Safe Read-Modify-Write function for EC
fn modify_ec_led(disable_breathing: bool) -> Result<()> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(EC_PATH)
        .context(format!("Cannot open {}. Missing root permissions?", EC_PATH))?;

    // 1. Read
    file.seek(SeekFrom::Start(LED_OFFSET))?;
    let mut buf = [0u8; 1];
    file.read_exact(&mut buf)?;
    let original = buf[0];

    // 2. Modify
    let new_val = if disable_breathing {
        original | LED_MASK // Set bit to 1
    } else {
        original & !LED_MASK // Clear bit (return to 0)
    };

    // 3. Write (only if changed, to avoid wearing out EC)
    if original != new_val {
        file.seek(SeekFrom::Start(LED_OFFSET))?;
        file.write_all(&[new_val])?;
        // println!("EC: 0x{:02X} -> 0x{:02X} (Bit {}={})", original, new_val, LED_BIT, disable_breathing);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    // Every operation requires EC access
    ensure_ec_access()?;

    match args.command {
        Commands::Off => {
            modify_ec_led(true)?;
            println!("LED: Static/OFF mode forced.");
        }
        Commands::On => {
            modify_ec_led(false)?;
            println!("LED: Default/ON mode restored.");
        }
        Commands::Daemon => {
            println!("Starting Legion LED Daemon...");
            println!("Listening for D-Bus signals (PrepareForSleep)...");

            // Connection to system D-Bus
            let conn = Connection::system().await?;
            
            // Proxy to systemd login manager
            let proxy = zbus::Proxy::new(
                &conn,
                "org.freedesktop.login1",
                "/org/freedesktop/login1",
                "org.freedesktop.login1.Manager",
            )
            .await?;

            // Signal subscription
            let mut stream = proxy.receive_signal("PrepareForSleep").await?;

            // Listening loop (idle until signal received)
            while let Some(msg) = stream.next().await {
                let body: (bool,) = msg.body().deserialize()?;
                let is_going_to_sleep = body.0;

                if is_going_to_sleep {
                    // System GOING TO SLEEP -> Disable LED
                    if let Err(e) = modify_ec_led(VAL_SLEEP_MODE) {
                        eprintln!("Error during sleep: {}", e);
                    }
                } else {
                    // System WOKE UP -> Restore LED
                    if let Err(e) = modify_ec_led(VAL_WAKE_MODE) {
                        eprintln!("Error during wake: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}
