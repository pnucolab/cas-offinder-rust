use cas_offinder_lib::*;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;

// Note this useful idiom: importing names from outer (for mod tests) scope.
fn gather_chrom_results(rec: &Receiver<ChromChunkInfo>) -> Vec<ChromChunkInfo> {
    let mut res: Vec<ChromChunkInfo> = Vec::new();
    for data in rec.iter() {
        res.push(data);
    }
    res
}
fn concat_results_as_str(chunks: &[ChromChunkInfo]) -> Vec<u8> {
    let mut res: Vec<u8> = Vec::new();
    for chunk in chunks.iter() {
        let mut chrchunk = [0_u8; CHUNK_SIZE];
        bit4_to_string(&mut chrchunk, &chunk.data[..], 0, chunk.size());
        res.extend(chrchunk.iter().take(chunk.size()));
    }
    res
}
fn get_expected_output() -> Vec<u8> {
    let expected_path = Path::new("./tests/test_data/expected.txt");
    let mut file = File::open(expected_path).unwrap();
    let mut res: Vec<u8> = Vec::new();
    file.read_to_end(&mut res).unwrap();
    res
}

#[test]
fn test_read_2bit() {
    let input_path = Path::new("./tests/test_data/upstream1000.2bit");
    let (sender, receiver): (SyncSender<ChromChunkInfo>, Receiver<ChromChunkInfo>) =
        mpsc::sync_channel(1);
    thread::spawn(move || {
        read_2bit(&sender, input_path).unwrap();
    });
    let results = gather_chrom_results(&receiver);
    let result_str = concat_results_as_str(&results);
    let expected_results = get_expected_output();
    assert_eq!(result_str, expected_results);
}
#[test]
fn test_read_fasta_folder() {
    let input_path = Path::new("./tests/test_data/");
    let (sender, receiver): (SyncSender<ChromChunkInfo>, Receiver<ChromChunkInfo>) =
        mpsc::sync_channel(1);
    thread::spawn(move || {
        read_fasta_folder(&sender, input_path).unwrap();
    });
    let results = gather_chrom_results(&receiver);
    let result_str = concat_results_as_str(&results);
    let expected_results = get_expected_output();
    assert_eq!(result_str, expected_results);
}

#[test]
fn test_read_fasta() {
    let input_path = Path::new("./tests/test_data/upstream1000.fa");
    let (sender, receiver): (SyncSender<ChromChunkInfo>, Receiver<ChromChunkInfo>) =
        mpsc::sync_channel(1);
    thread::spawn(move || {
        read_fasta(&sender, input_path).unwrap();
    });
    let results = gather_chrom_results(&receiver);
    let result_str = concat_results_as_str(&results);
    let expected_results = get_expected_output();
    assert_eq!(result_str, expected_results);
}
