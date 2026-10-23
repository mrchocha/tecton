use std::fs;

use crate::{error::WalError, file::WalFile, manager::WalOptions};

pub fn get_first_file(
    options: &WalOptions,
    after: Option<u32>,
) -> Result<Option<WalFile>, WalError> {
    let mut first_wal_file: Option<WalFile> = None;
    let wal_files: Vec<WalFile> = list_files(options)?;

    for wal_file in wal_files {
        if first_wal_file.as_ref().is_none_or(|latest| {
            wal_file.seq_no < latest.seq_no && wal_file.seq_no >= after.unwrap_or(0)
        }) {
            first_wal_file = Some(wal_file);
        }
    }

    Ok(first_wal_file)
}

fn list_files(options: &WalOptions) -> Result<Vec<WalFile>, WalError> {
    let mut wal_files = Vec::new();
    let path = options.file_path.clone();

    for entry in fs::read_dir(path)? {
        let path = entry?.path();

        if path.extension().is_none_or(|ext| ext != "wal") {
            continue;
        }

        let Some(path_str) = path.to_str() else {
            continue;
        };

        let wal_file = WalFile::from_path(path_str)?;
        wal_files.push(wal_file);
    }

    Ok(wal_files)
}
