mod clip;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "oc", version, about = "OpenContinuity: cihazlar arası pano senkronizasyonu (PoC)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Sürüm, platform ve pano erişim bilgisini yazdırır.
    Info,
    /// Pano değişikliklerini izler; içerik değil yalnız uzunluk ve hash yazar.
    ClipWatch,
    /// Panoya metin yazar.
    ClipSet { text: String },
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Info => {
            println!(
                "oc {} ({}-{})",
                env!("CARGO_PKG_VERSION"),
                std::env::consts::OS,
                std::env::consts::ARCH
            );
            if let Some(behavior) = oc_clipboard::access_behavior() {
                println!("macOS pano erişim davranışı: {behavior}");
            }
        }
        Command::ClipWatch => clip::watch()?,
        Command::ClipSet { text } => clip::set(&text)?,
    }
    Ok(())
}
