use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "oc", version, about = "OpenContinuity: cihazlar arası pano senkronizasyonu (PoC)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Sürüm ve platform bilgisini yazdırır.
    Info,
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
        }
    }
    Ok(())
}
