use std::path::PathBuf;

use anyhow::Context;
use oc_pairing::Identity;

/// `OC_CONFIG_DIR` verilirse onu kullanır (aynı makinede iki örnek denemek için).
pub fn dir() -> anyhow::Result<PathBuf> {
    if let Some(dir) = std::env::var_os("OC_CONFIG_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let dirs = directories::ProjectDirs::from("org", "OpenContinuity", "OpenContinuity")
        .context("kullanıcı config dizini bulunamadı")?;
    Ok(dirs.config_dir().to_path_buf())
}

pub fn identity() -> anyhow::Result<Identity> {
    let name = gethostname::gethostname().to_string_lossy().trim_end_matches(".local").to_owned();
    Identity::load_or_create(&dir()?, &name)
}
