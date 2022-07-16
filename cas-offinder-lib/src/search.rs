use opencl3::*;
use crate::chrom_chunk::*;
use crate::bit4ops::{cdiv,roundup};
use std::ops::Add;
use std::sync::mpsc;
use std::thread;
use crossbeam_channel;
use std::iter::Peekable;
use std::sync::Arc;
use std::slice;


const SEARCH_CHUNK_SIZE: usize = 1<<22; // must be less than 1<<32
const SEARCH_CHUNK_SIZE_BYTES: usize = SEARCH_CHUNK_SIZE/2;
const CHUNKS_PER_SEARCH:usize = SEARCH_CHUNK_SIZE/CHUNK_SIZE;

const CPU_BLOCK_SIZE: usize = 8;
const GPU_BLOCK_SIZE: usize = 4;
const PATTERN_CHUNK_SIZE:usize = 16;

struct SearchChunkMeta{
    pub chr_names: Vec<String>,
    // start and end of data within chromosome, by nucleotide
    pub chunk_starts: Vec<u64>,
    pub chunk_ends: Vec<u64>,
}

struct SearchChunkInfo{
    // fixed size data, divied into SEARCH_CHUNK_SIZE/CHUNK_SIZE chunks
    pub data: Box<[u8;SEARCH_CHUNK_SIZE_BYTES]>,
    pub meta: SearchChunkMeta,
}
struct SearchChunkResult{
    pub matches: Vec<SearchMatch>,
    pub meta: SearchChunkMeta,
}

struct SearchMatch{
    pub chunk_idx: u32,
    pub pattern_idx: u32,
    pub mismatches: u32,
}
// fn make_patternbuf(){}

fn search_device(recv: &crossbeam_channel::Receiver<SearchChunkInfo>, dest: &mpsc::SyncSender<SearchChunkResult>){

}
fn search_compute(patterns: &Vec<Vec<u8>>,recv: mpsc::Receiver<SearchChunkInfo>, dest: mpsc::SyncSender<SearchChunkResult>){
    /* divies off work to opencl devices */
}
fn checked_div(x:usize, y:usize)->usize{
    assert!(x % y == 0);
    x/y
}
fn pack(d:&[u8])->u64{
    assert!(d.len() == 8);
    unsafe{std::mem::transmute::<[u8;8],u64>([d[0],d[1],d[2],d[3],d[4],d[5],d[6],d[7]])}
}
fn block_data_cpu(data:&[u8])->Vec<u64>{
    data.chunks(8).map(|d|pack(d)).collect()
}
fn search_chunk_cpu(max_mismatches: u32, pattern_len: usize, packed_patterns:&[u8],data:&[u8;SEARCH_CHUNK_SIZE_BYTES])->Vec<SearchMatch>{
    let mut matches :Vec<SearchMatch> = Vec::new();
    assert!(data.as_ptr().align_offset(8) == 0, "data must be 8 byte aligned");
    let n_patterns = checked_div(packed_patterns.len(),roundup(pattern_len, 2*PATTERN_CHUNK_SIZE)/2);
    let pattern_ptr = block_data_cpu(packed_patterns);//packed_patterns.as_ptr() as *const u64;
    let genome_ptr = block_data_cpu(data);//data.as_ptr() as *const u64;
    let genome_blocks = checked_div(data.len(),CPU_BLOCK_SIZE);
    let pattern_blocks = cdiv(pattern_len,CPU_BLOCK_SIZE*2);
    let packed_pattern_size = roundup(pattern_blocks, PATTERN_CHUNK_SIZE/CPU_BLOCK_SIZE);
    // assert!(pattern_blocks == 2);
    // assert!(n_patterns == 2);
    const NUCL_PER_BLOCK:usize = 2*std::mem::size_of::<u64>();
    const BLOCKS_PER_EXEC: usize = 4;
    let mut shifted_data = vec![0 as u64; BLOCKS_PER_EXEC+pattern_blocks+1];
    for gen_block_idx in 0..checked_div(genome_blocks,BLOCKS_PER_EXEC){
        let gen_idx = gen_block_idx*BLOCKS_PER_EXEC;
        shifted_data.fill(0);
        let n_copy = std::cmp::min(BLOCKS_PER_EXEC+pattern_blocks+1, genome_blocks-gen_idx);
        shifted_data[..n_copy].copy_from_slice(&genome_ptr[gen_idx..][..n_copy]);
        for l in 0..NUCL_PER_BLOCK{
            for j in 0..n_patterns{
                let mut num_matches = [0 as u32; BLOCKS_PER_EXEC];
                for k in 0..pattern_blocks{
                    for o in 0..BLOCKS_PER_EXEC{
                        num_matches[o] += (
                            shifted_data[k+o] & 
                            pattern_ptr[j*packed_pattern_size+k]
                        ).count_ones()
                    }
                }
                for o in 0..BLOCKS_PER_EXEC{
                    let mismatches = pattern_len as u32 - num_matches[o];
                    if mismatches <= max_mismatches{
                        matches.push(SearchMatch{
                            chunk_idx: ((gen_idx+o)*NUCL_PER_BLOCK+l) as u32,
                            pattern_idx: j as u32,
                            mismatches: mismatches,
                        });
                    }
                }
            }
            for k in 0..(pattern_blocks+BLOCKS_PER_EXEC-1){
                shifted_data[k] >>= 4;
                shifted_data[k] |= shifted_data[k+1] << (4*(NUCL_PER_BLOCK - 1));
            }
            shifted_data[pattern_blocks+BLOCKS_PER_EXEC-1] >>= 4;
        }
    }
    println!("{}",matches.len());
    matches
}

fn pack_patterns(patterns: &Vec<Vec<u8>>)->Vec<u8>{
    patterns.iter()
        .map(|pattern|{
            let pattern_padding = cdiv(pattern.len(), PATTERN_CHUNK_SIZE)*PATTERN_CHUNK_SIZE - pattern.len();
            pattern.iter().copied()
            .chain((0..pattern_padding).map(|_|0 as u8))
        })
        .flatten()
        .collect()
}
fn search_device_cpu_thread(max_mismatches: u32, pattern_len: usize, packed_patterns: Arc<Vec<u8>>, recv: crossbeam_channel::Receiver<SearchChunkInfo>, dest: mpsc::SyncSender<SearchChunkResult>){
    for schunk in recv.iter(){
        dest.send(SearchChunkResult{
            matches:search_chunk_cpu(max_mismatches, pattern_len, &packed_patterns,&schunk.data),
            meta:schunk.meta,
        }).unwrap();
    }
}
fn search_compute_cpu(max_mismatches: u32, pattern_len: usize, patterns: &Vec<Vec<u8>>,recv: crossbeam_channel::Receiver<SearchChunkInfo>, dest: mpsc::SyncSender<SearchChunkResult>){
    /* divies off work to threads devices */
    let pattern_arc = Arc::new(pack_patterns(patterns));
    let n_threads = thread::available_parallelism().unwrap().into();
    let mut threads:Vec<thread::JoinHandle<()>> = Vec::new();
    for _ in 0..n_threads{
        let tpattners = pattern_arc.clone();
        let trecv = recv.clone();
        let tdest = dest.clone();
        threads.push(thread::spawn(move||search_device_cpu_thread(max_mismatches, pattern_len,tpattners, trecv, tdest)));
    }
    for t in threads{
        t.join().unwrap();
    }
}

fn chunks_to_searchchunk(chunk_buf:&Vec<ChromChunkInfo>)->SearchChunkInfo
{
    let mut search_buf = Box::new([0 as u8;SEARCH_CHUNK_SIZE_BYTES]);
    let mut names:Vec<String> = Vec::with_capacity(CHUNKS_PER_SEARCH);
    let mut starts:Vec<u64> = Vec::with_capacity(CHUNKS_PER_SEARCH);
    let mut ends:Vec<u64> = Vec::with_capacity(CHUNKS_PER_SEARCH);
    // only takes  CHUNKS_PER_SEARCH-1 chunks because you don't want to leave any hanging data on the end
    for (idx,chunk) in chunk_buf.iter().enumerate(){
        assert!(idx == 0 || *ends.last().unwrap() == chunk.chunk_start || chunk.chunk_start == 0, 
                "search expects chromosome chunks to arrive in order");
        search_buf[idx*CHUNK_SIZE_BYTES..(idx+1)*CHUNK_SIZE_BYTES].copy_from_slice(&chunk.data[..]);
        names.push(chunk.chr_name.clone());
        starts.push(chunk.chunk_start);
        ends.push(chunk.chunk_end);
    }
    SearchChunkInfo{
        data: search_buf,
        meta: SearchChunkMeta{
            chr_names: names,
            chunk_starts: starts,
            chunk_ends: ends,
        }
    }
}
fn get_match_key(x:&SearchMatch)->u32{x.chunk_idx}
fn convert_matches(pattern_len:usize, search_res: SearchChunkResult)->Vec<Match>{
    let chr_names:Vec<Arc<String>> = search_res.meta.chr_names.iter().map(|name|Arc::new(name.clone())).collect();
    let mut results:Vec<Match> = Vec::new();
    for smatch in search_res.matches.iter(){
        let idx = smatch.chunk_idx as usize / CHUNK_SIZE;
        let offset = smatch.chunk_idx as usize % CHUNK_SIZE;
        let pos = search_res.meta.chunk_starts[idx] + offset as u64;
        //skip anything in the last chunk, it will be repeated again in the next search item
        let is_last_chunk = idx == CHUNKS_PER_SEARCH-1;
        let is_end_chrom = idx == chr_names.len()-1 || search_res.meta.chunk_starts[idx+1] == 0;
        let is_past_end = pos + pattern_len as u64 > search_res.meta.chunk_ends[idx];
        if !is_last_chunk && !(is_end_chrom && is_past_end){
            results.push(Match{
                chr_name: chr_names[idx].clone(),
                chrom_idx: pos,
                pattern_idx: smatch.pattern_idx,
                mismatches: smatch.mismatches,
            });
        }
    }
    results
}

pub fn search(max_mismatches: u32, pattern_len: usize, patterns: &Vec<Vec<u8>>,recv: mpsc::Receiver<ChromChunkInfo>, dest: mpsc::SyncSender<Vec<Match>>){
    /* public facing function, sends and receives data chunk by chunk */
    assert!(patterns.len() > 0);
    assert!(patterns[0].len()*2 >= pattern_len);
    let (compute_send_src, compute_recv_src): (crossbeam_channel::Sender<SearchChunkInfo>, crossbeam_channel::Receiver<SearchChunkInfo>) = crossbeam_channel::bounded(4);
    let (compute_send_dest, compute_recv_dest): (mpsc::SyncSender<SearchChunkResult>, mpsc::Receiver<SearchChunkResult>) = mpsc::sync_channel(4);
    assert!(patterns.iter().all(|p|p.len() == patterns[0].len()), "All patterns must have same length");
    let send_thread = thread::Builder::new().stack_size(SEARCH_CHUNK_SIZE_BYTES*2).spawn(move|| {
        let mut buf:Vec<ChromChunkInfo> = Vec::with_capacity(CHUNKS_PER_SEARCH);
        loop{
            let res = recv.recv();
            match res{
                Ok(chunk)=>{
                    buf.push(chunk);
                    if buf.len() == CHUNKS_PER_SEARCH{
                        compute_send_src.send(chunks_to_searchchunk(&buf)).unwrap();
                        let last_el = buf.pop().unwrap();
                        buf.clear();
                        //last element is now first element so that no patterns are cut off
                        buf.push(last_el);
                    }
                }
                Err(e)=>{
                    break;
                }
            }
        }
        if buf.len() > 0{
            compute_send_src.send(chunks_to_searchchunk(&buf)).unwrap();
        }
    }).unwrap();
    let recv_thread = thread::spawn(move|| {
        for search_chunk in compute_recv_dest.iter(){
            dest.send(convert_matches(pattern_len,search_chunk)).unwrap();
        }
    });        
    search_compute_cpu(max_mismatches, pattern_len, patterns,compute_recv_src, compute_send_dest);
    send_thread.join().unwrap();
    recv_thread.join().unwrap();
}



#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use crate::string_to_bit4;
    use crate::read_2bit;
    use std::path::Path;

    use crate::CHUNK_SIZE_BYTES;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_opencl_runtime(){
        let platforms = platform::get_platforms().unwrap();
        let num_devices:usize = platforms.iter().map(|plat| plat.get_devices(device::CL_DEVICE_TYPE_ALL).unwrap().len()).sum();

        assert!(num_devices>0, "Needs at least one opencl device to run tests!");
    }
    #[test]
    fn test_search_smoke() {
        let (src_sender, src_receiver): (mpsc::SyncSender<ChromChunkInfo>, mpsc::Receiver<ChromChunkInfo>) = mpsc::sync_channel(4);
        let (dest_sender, dest_receiver): (mpsc::SyncSender<Vec<Match>>, mpsc::Receiver<Vec<Match>>) = mpsc::sync_channel(4);
        const NUM_ITERS:usize = 2;
        let send_thread = thread::spawn(move|| {
            for i in 0..NUM_ITERS{
                read_2bit(&src_sender, Path::new("tests/test_data/upstream1000.2bit"));
            }
            // for i in 0..NUM_ITERS{
            //     src_sender.send(ChromChunkInfo{
            //         chr_name: String::from_str("chr").unwrap(),
            //         data: Box::new([1 as u8; CHUNK_SIZE_BYTES]),
            //         chunk_start: 0,
            //         chunk_end: CHUNK_SIZE as u64,
            //     }).unwrap();
            // }
        });        
        let result_count = thread::spawn(move|| {
            let mut count:usize = 0;
            for chunk in dest_receiver.iter(){
                count += chunk.len();
            }
            count
        });
        let pattern1 = b"CCGTGGTTCAACATTTGCTTAGCA";
        let pattern2 = b"GATGTTGGTAAGTGGGATATGGCA";
        let mut pattern3 = pattern1.clone();
        let mut pattern4 =pattern2.clone();
        pattern3.reverse();
        pattern4.reverse();
        let mut pattern1_bit4 = vec![0 as u8; 12];
        let mut pattern2_bit4 =vec![0 as u8; 12];
        let mut pattern3_bit4 = vec![0 as u8; 12];
        let mut pattern4_bit4 =vec![0 as u8; 12];
        string_to_bit4(&mut pattern1_bit4, pattern1, 0, true);
        string_to_bit4(&mut pattern2_bit4, pattern2, 0, true);
        string_to_bit4(&mut pattern3_bit4, &pattern3, 0, true);
        string_to_bit4(&mut pattern4_bit4, &pattern4, 0, true);
        let patterns = vec![
            pattern1_bit4,
            pattern2_bit4,
            pattern3_bit4,
            pattern4_bit4,
        ];
        let max_mismatches = 11;
        let expected_results_per_file = 117;
        let expected_results = expected_results_per_file * NUM_ITERS;
        let pattern_len = pattern2.len();
        search(max_mismatches, pattern_len, &patterns,src_receiver, dest_sender);
        send_thread.join().unwrap();
        assert_eq!(result_count.join().unwrap(), expected_results);
    }
}