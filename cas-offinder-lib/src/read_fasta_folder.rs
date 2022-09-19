use crate::chrom_chunk::ChromChunkInfo;
use crate::cli_err::CliError;
use crate::read_fasta::read_fasta;
use std::fs::read_dir;
use std::path::Path;
use std::sync::mpsc::SyncSender;

pub fn read_fasta_folder(dest: &SyncSender<ChromChunkInfo>, folder: &Path) -> Result<(), CliError> {
    for path_r in read_dir(folder)? {
        let path = path_r?;
        if path.file_type()?.is_file() {
            let path_p = path.path();
            let path_name = path_p.as_os_str().to_str().ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "paths must be utf-8 encodeable",
            ))?;
            if path_name.ends_with(".fa") {
                read_fasta(dest, &path_p)?;
            }
        }
    }
    Ok(())
}

/*
unit tests for this in integration tests.
*/
