use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::process::Command;
use zbus::export::futures_util::StreamExt;
use zbus::Connection;

// --- KONFIGURACJA HARDWARE (Z Twoich plików) ---
const EC_PATH: &str = "/sys/kernel/debug/ec/ec0/io";
const LED_OFFSET: u64 = 0x10;
const LED_BIT: u8 = 6;      // Bit 6
const LED_MASK: u8 = 1 << LED_BIT; // 0x40 (64)

// --- KONFIGURACJA LOGIKI ---
// Bit 1 (True) zazwyczaj wyłącza oddychanie (Static/Off w kontekście sleep)
// Bit 0 (False) włącza oddychanie (Default)
const VAL_SLEEP_MODE: bool = true;  // Ustaw bit na 1 przed snem (wyłącz miganie)
const VAL_WAKE_MODE: bool = false; // Ustaw bit na 0 po obudzeniu (powrót do systemu)

#[derive(Parser)]
#[command(name = "legion-led")]
#[command(about = "Legion Go S LED Controller & Daemon", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Uruchamia proces w tle (nasłuchuje uśpienia/wybudzenia)
    Daemon,
    /// Wymusza tryb "OFF" (przestaje migać/świecić statycznie)
    Off,
    /// Przywraca tryb "ON" (domyślne oddychanie/sterowanie systemowe)
    On,
}

/// Sprawdza i ładuje moduł jądra, jeśli nie jest obecny
fn ensure_ec_access() -> Result<()> {
    if !std::path::Path::new(EC_PATH).exists() {
        println!("Ładowanie modułu ec_sys...");
        let status = Command::new("modprobe")
            .arg("ec_sys")
            .arg("write_support=1")
            .status()
            .context("Nie udało się uruchomić modprobe")?;

        if !status.success() {
            return Err(anyhow::anyhow!("Błąd ładowania ec_sys. Uruchom jako root!"));
        }
    }
    Ok(())
}

/// Bezpieczna funkcja Read-Modify-Write dla EC
fn modify_ec_led(disable_breathing: bool) -> Result<()> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(EC_PATH)
        .context(format!("Nie można otworzyć {}. Brak uprawnień root?", EC_PATH))?;

    // 1. Odczyt
    file.seek(SeekFrom::Start(LED_OFFSET))?;
    let mut buf = [0u8; 1];
    file.read_exact(&mut buf)?;
    let original = buf[0];

    // 2. Modyfikacja
    let new_val = if disable_breathing {
        original | LED_MASK // Ustaw bit na 1
    } else {
        original & !LED_MASK // Wyzeruj bit (powrót do 0)
    };

    // 3. Zapis (tylko jeśli jest zmiana, żeby nie męczyć EC)
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

    // Każda operacja wymaga dostępu do EC
    ensure_ec_access()?;

    match args.command {
        Commands::Off => {
            modify_ec_led(true)?;
            println!("Dioda: Tryb statyczny/OFF wymuszony.");
        }
        Commands::On => {
            modify_ec_led(false)?;
            println!("Dioda: Tryb domyślny/ON przywrócony.");
        }
        Commands::Daemon => {
            println!("Start demona Legion LED...");
            println!("Nasłuchiwanie sygnałów D-Bus (PrepareForSleep)...");

            // Połączenie z systemową szyną D-Bus
            let conn = Connection::system().await?;
            
            // Proxy do menedżera logowania systemd
            let proxy = zbus::Proxy::new(
                &conn,
                "org.freedesktop.login1",
                "/org/freedesktop/login1",
                "org.freedesktop.login1.Manager",
            )
            .await?;

            // Subskrypcja sygnału
            let mut stream = proxy.receive_signal("PrepareForSleep").await?;

            // Pętla nasłuchująca (nic nie robi dopóki nie przyjdzie sygnał)
            while let Some(msg) = stream.next().await {
                let body: (bool,) = msg.body().deserialize()?;
                let is_going_to_sleep = body.0;

                if is_going_to_sleep {
                    // System IDZIE SPAĆ -> Wyłącz diodę
                    if let Err(e) = modify_ec_led(VAL_SLEEP_MODE) {
                        eprintln!("Błąd podczas usypiania: {}", e);
                    }
                } else {
                    // System WSTAŁ -> Przywróć diodę
                    if let Err(e) = modify_ec_led(VAL_WAKE_MODE) {
                        eprintln!("Błąd podczas wybudzania: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}