use super::errors::*;
use xdg;
use std::path::PathBuf;
use std::fs::{read_dir, DirEntry};

/// Automatically find an IRD file to use
///
/// Path is `$XDG_DATA_HOME/ps3bdtool/ird_files/`
///
/// so, probably, `~/.local/share/ps3bdtool/ird_files/`
///
pub fn find_ird_file(gameid: &str) -> Result<Option<PathBuf>> {
    let xdg_dirs: xdg::BaseDirectories = xdg::BaseDirectories::with_prefix("ps3bdtool").chain_err(|| "Failed to get base directories")?;
    let ird_dir = xdg_dirs.create_data_directory("ird_files").chain_err(|| "Failed to get ird files directory")?;
    for file in read_dir(ird_dir).chain_err(||"failed to read directory")? {
        let dir_entry: DirEntry = file.chain_err(||"failed to read file")?;
        let filename = dir_entry.file_name();
        let filename = filename.to_string_lossy();
        if filename.as_ref().contains(gameid) {
            return Ok(Some(dir_entry.path()))
        }
    }
    Ok(None)
}