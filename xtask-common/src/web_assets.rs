use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use bityzba::requires;

pub const WEB_ASSET_SYNC_TEMP_DIR_NAME: &str = ".jbotci-asset-sync";

#[requires(true)]
#[bityzba::ensures(true)]
pub fn remove_web_asset_sync_temp_dir(temp_dir: &Path) {
    // Asset replacement has already succeeded when this runs; a locked stale
    // temp directory should not fail the build, and later sync/prune passes try
    // the same cleanup path again.
    match fs::remove_dir_all(temp_dir) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(_) => {}
    }
}
