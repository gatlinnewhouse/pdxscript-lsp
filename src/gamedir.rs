//! Locate the Victoria 3 game directory via Steam or local Paradox files.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use steamlocate::SteamDir;

const VIC3_APP_ID: u32 = 529_340;

const PDX_LINUX: &str = ".local/share/Paradox Interactive";
const PDX_MAC: &str = "Library/Application Support/Paradox Interactive";
const PDX_WINDOWS: &str = "Documents/Paradox Interactive";

/// Try to find the Victoria 3 game directory via Steam.
pub fn find_game_directory_steam() -> Result<PathBuf> {
    let steamdir = SteamDir::locate()?;
    if let Some((app, library)) = steamdir.find_app(VIC3_APP_ID)? {
        Ok(library.resolve_app_dir(&app))
    } else {
        bail!("Victoria 3 not found in Steam library")
    }
}

/// Try to find the Steam workshop directory for Victoria 3.
pub fn find_workshop_directory_steam() -> Result<PathBuf> {
    let steamdir = SteamDir::locate()?;
    let mut path = steamdir.path().to_path_buf();
    path.push("steamapps/workshop/content");
    path.push(VIC3_APP_ID.to_string());
    Ok(path)
}

/// Try to find the local Paradox Interactive directory for Victoria 3.
pub fn find_paradox_directory() -> Option<PathBuf> {
    let home = home::home_dir()?;
    let target = Path::new("Victoria 3");
    for try_dir in &[PDX_LINUX, PDX_MAC, PDX_WINDOWS] {
        let full = home.join(try_dir).join(target);
        if full.is_dir() {
            return Some(full);
        }
    }
    None
}
