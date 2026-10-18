use std::net::{IpAddr, UdpSocket};
use std::path::PathBuf;

use anyhow::Context;
use oc_transport::DEFAULT_PORT;

/// `OC_CONFIG_DIR` verilirse onu kullanır (aynı makinede iki örnek denemek için).
pub fn default_dir() -> anyhow::Result<PathBuf> {
    if let Some(dir) = std::env::var_os("OC_CONFIG_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let dirs = directories::ProjectDirs::from("org", "OpenContinuity", "OpenContinuity")
        .context("kullanıcı config dizini bulunamadı")?;
    Ok(dirs.config_dir().to_path_buf())
}

/// `OC_FILES_DIR` verilirse onu kullanır; yoksa İndirilenler/OpenContinuity.
pub fn files_dir() -> anyhow::Result<PathBuf> {
    if let Some(dir) = std::env::var_os("OC_FILES_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let downloads = directories::UserDirs::new()
        .and_then(|dirs| dirs.download_dir().map(PathBuf::from))
        .context("İndirilenler klasörü bulunamadı")?;
    Ok(downloads.join("OpenContinuity"))
}

pub fn device_name() -> String {
    gethostname::gethostname().to_string_lossy().trim_end_matches(".local").to_owned()
}

/// `OC_PORT` verilirse onu kullanır (aynı makinede iki örnek için).
pub fn port() -> anyhow::Result<u16> {
    match std::env::var("OC_PORT") {
        Ok(port) => port.parse().context("OC_PORT geçerli bir port değil"),
        Err(_) => Ok(DEFAULT_PORT),
    }
}

/// Varsayılan rotanın yerel adresi; UDP `connect` paket göndermez.
pub fn local_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?;
    Some(socket.local_addr().ok()?.ip())
}
