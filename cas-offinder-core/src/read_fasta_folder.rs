use std::sync::mpsc::{Sender, };
use std::fs::read_dir;
use crate::chrom_chunk::{ChromChunkInfo, };
use crate::read_fasta::{read_fasta};
use crate::cli_err::CliError;

pub fn read_fasta_folder(dest:&Sender<ChromChunkInfo>, folder: &str)->Result<(),CliError>{
    for path_r in read_dir(folder)?{
        let path = path_r?;
        if path.file_type()?.is_file(){
            let path_p = path.path();
            let path_name = path_p.as_os_str().to_str().ok_or(std::io::Error::new(std::io::ErrorKind::InvalidData, "paths must be utf-8 encodeable"))?;
            if path_name.ends_with(".fa"){
                read_fasta(&dest, &path_p)?;
            }
        }
    }
    Ok(())
}