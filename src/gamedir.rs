//! Locate the game directory via Steam or local Paradox files.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use steamlocate::SteamDir;

// Steam app IDs
#[cfg(feature = "vic3")]       const APP_ID: u32 = 529_340;
#[cfg(feature = "ck3")]        const APP_ID: u32 = 1158310;
#[cfg(feature = "imperator")]  const APP_ID: u32 = 859580;
#[cfg(feature = "hoi4")]       const APP_ID: u32 = 394360;
#[cfg(feature = "eu5")]        const APP_ID: u32 = 1623920;

// Paradox local data subdirectory name
#[cfg(feature = "vic3")]       const PDX_DIR: &str = "Victoria 3";
#[cfg(feature = "ck3")]        const PDX_DIR: &str = "Crusader Kings III";
#[cfg(feature = "imperator")]  const PDX_DIR: &str = "Imperator";
#[cfg(feature = "hoi4")]       const PDX_DIR: &str = "Hearts of Iron IV";
#[cfg(feature = "eu5")]        const PDX_DIR: &str = "Europa Universalis 5";

/// The conf file name that marks a mod root for this game.
#[cfg(feature = "vic3")]       pub const TIGER_CONF: &str = "vic3-tiger.conf";
#[cfg(feature = "ck3")]        pub const TIGER_CONF: &str = "ck3-tiger.conf";
#[cfg(feature = "imperator")]  pub const TIGER_CONF: &str = "imperator-tiger.conf";
#[cfg(feature = "hoi4")]       pub const TIGER_CONF: &str = "hoi4-tiger.conf";
#[cfg(feature = "eu5")]        pub const TIGER_CONF: &str = "eu5-tiger.conf";

const PDX_LINUX: &str   = ".local/share/Paradox Interactive";
const PDX_MAC: &str     = "Library/Application Support/Paradox Interactive";
const PDX_WINDOWS: &str = "Documents/Paradox Interactive";

pub fn find_game_directory_steam() -> Result<PathBuf> {
    let steamdir = SteamDir::locate()?;
    if let Some((app, library)) = steamdir.find_app(APP_ID)? {
        Ok(library.resolve_app_dir(&app))
    } else {
        bail!("Game not found in Steam library (app id {APP_ID})")
    }
}

pub fn find_workshop_directory_steam() -> Result<PathBuf> {
    let steamdir = SteamDir::locate()?;
    let mut path = steamdir.path().to_path_buf();
    path.push("steamapps/workshop/content");
    path.push(APP_ID.to_string());
    Ok(path)
}

pub fn find_paradox_directory() -> Option<PathBuf> {
    let home = home::home_dir()?;
    let target = Path::new(PDX_DIR);
    for try_dir in &[PDX_LINUX, PDX_MAC, PDX_WINDOWS] {
        let full = home.join(try_dir).join(target);
        if full.is_dir() {
            return Some(full);
        }
    }
    None
}
