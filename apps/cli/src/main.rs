mod clip;
mod pair;
mod run;

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
    /// Panoya PNG dosyası yazar.
    ClipSetImage { path: std::path::PathBuf },
    /// Panoya dosya listesi yazar (Finder/Gezgin'de yapıştırılabilir).
    ClipSetFiles { paths: Vec<std::path::PathBuf> },
    /// Cihaz kimliğini ve eşleşmiş eşi gösterir.
    Id,
    /// Eşleştirir. `--addr` yoksa aynı ağda `oc pair` çalıştıran cihazı kendisi bulur.
    Pair {
        /// Karşı cihazın adresi (IP veya IP:PORT).
        #[arg(long)]
        addr: Option<String>,
    },
    /// Eşleşmeyi kaldırır.
    Unpair,
    /// Eşle bağlantıyı ayakta tutar ve panoyu iki yönde senkronlar.
    Run,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
        Command::ClipSetImage { path } => clip::set_image(&path)?,
        Command::ClipSetFiles { paths } => clip::set_files(&paths)?,
        Command::Id => pair::show_identity()?,
        Command::Pair { addr } => pair::pair(addr).await?,
        Command::Unpair => pair::unpair()?,
        Command::Run => run::run().await?,
    }
    Ok(())
}
