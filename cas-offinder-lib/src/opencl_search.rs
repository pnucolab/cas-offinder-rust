use opencl3::*;
use crate::{ChromChunkInfo, ChromChunkResult, CHUNK_SIZE, CHUNK_SIZE_BYTES};
use std::sync::mpsc;
use std::thread;
use crossbeam_channel;


const SEARCH_CHUNK_SIZE: usize = 1<<22; // must be less than 1<<32
const SEARCH_CHUNK_SIZE_BYTES: usize = SEARCH_CHUNK_SIZE/2;
const CHUNKS_PER_SEARCH:usize = SEARCH_CHUNK_SIZE/CHUNK_SIZE;


struct SearchChunkInfo{
    pub chr_names: Vec<String>,
    // fixed size data, divied into SEARCH_CHUNK_SIZE/CHUNK_SIZE chunks
    pub data: Box<[u8;SEARCH_CHUNK_SIZE_BYTES]>,
    // start and end of data within chromosome, by nucleotide
    pub chunk_starts: Vec<u64>,
    pub chunk_ends: Vec<u64>,
}

struct SearchMatch{
    pub chunk_idx: u32,
    pub pattern_idx: u32,
    pub mismatches: u32,
}

fn search_device(recv: &crossbeam_channel::Receiver<SearchChunkInfo>, dest: &mpsc::SyncSender<Vec<SearchMatch>>){

}

fn search_largechunks(recv: mpsc::Receiver<SearchChunkInfo>, dest: mpsc::SyncSender<Vec<SearchMatch>>){
    /* divies off work to opencl devices */
}

pub fn search(recv: mpsc::Receiver<ChromChunkInfo>, dest: mpsc::SyncSender<ChromChunkResult>){
    /* public facing function, sends and receives data chunk by chunk */
    for item in recv.iter(){
        dest.send(ChromChunkResult{
            chr_name: item.chr_name,
            results: Vec::new(),
        });
    }
}



#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::CHUNK_SIZE_BYTES;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_opencl_compile(){
        let platforms = platform::get_platforms().unwrap();
        for plat in platforms.iter(){
            println!("{}",plat.name().unwrap());
        }
    }
    #[test]
    fn test_search_smoke() {
        let (src_sender, src_receiver): (mpsc::SyncSender<ChromChunkInfo>, mpsc::Receiver<ChromChunkInfo>) = mpsc::sync_channel(4);
        let (dest_sender, dest_receiver): (mpsc::SyncSender<ChromChunkResult>, mpsc::Receiver<ChromChunkResult>) = mpsc::sync_channel(4);
        const NUM_ITERS:usize = 1;
        let send_thread = thread::spawn(move|| {
            for i in 0..NUM_ITERS{
                src_sender.send(ChromChunkInfo{
                    chr_name: String::from_str("chr").unwrap(),
                    data: Box::new([0 as u8; CHUNK_SIZE_BYTES]),
                    chunk_start: 0,
                    chunk_end: CHUNK_SIZE as u64,
                }).unwrap();
            }
        });        
        let result_count = thread::spawn(move|| {
            let mut count:usize = 0;
            for chunk in dest_receiver.iter(){
                count += 1;
            }
            count
        });
        search(src_receiver, dest_sender);
        send_thread.join().unwrap();
        assert_eq!(result_count.join().unwrap(), NUM_ITERS);
    }
}