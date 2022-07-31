mod cli_utils;

use cas_offinder_lib::*;
use std::path::Path;
use std::thread;
use std::env;
use std::sync::mpsc;
use crate::cli_utils::SearchRunInfo;
use crate::cli_utils::parse_and_validate_args;


fn get_usage(device_strs: &[String])->String{
    const PKG_VERSION: &'static str = env!("CARGO_PKG_VERSION");
    // const PKG_EDITION: &'static str = env!("CARGO_PKG_DATETIME");
    const AUTHORS: &'static str = env!("CARGO_PKG_AUTHORS");
    const HOMEPAGE: &'static str = env!("CARGO_PKG_HOMEPAGE");
    let dev_info = device_strs.join("\n");
    format!("
Cas-OFFinder-Rust v{}

Copyright (c) 2022 {}
Website: {}

Usage: cas-offinder [options] {{input_filename|-}} {{C|G|A}}[device_id(s)] {{output_filename|-}}
(C: using CPUs, G: using GPUs, A: using accelerators)

Example input file:
/var/chromosomes/human_hg19
NNNNNNNNNNNNNNNNNNNNNRG
GGCCGACCTGTCGCTGACGCNNN 5
CGCCAGCGTCAGCGACAGGTNNN 5
ACGGCGCCAGCGTCAGCGACNNN 5
GTCGCTGACGCTGGCGCCGTNNN 5

Available device list:
{}
",
PKG_VERSION,
AUTHORS,
HOMEPAGE,
dev_info)
}
fn get_usage_with_devices()->String{
    let run_config = match OclRunConfig::new(OclDeviceType:: ALL){
        Err(err)=>panic!("OpenCL runtime errored on load with error: {}",err.to_string()),
        Ok(cfg)=>cfg
    };
    get_usage(&run_config.get_device_strs())
}
fn main() {
    let args: Vec<String> = env::args().collect();
    let run_info:SearchRunInfo = parse_and_validate_args(&args).unwrap();
    
    let (src_sender, src_receiver): (mpsc::SyncSender<ChromChunkInfo>, mpsc::Receiver<ChromChunkInfo>) = mpsc::sync_channel(4);
    let (dest_sender, dest_receiver): (mpsc::SyncSender<Vec<Match>>, mpsc::Receiver<Vec<Match>>) = mpsc::sync_channel(4);
    const NUM_ITERS:usize = 2;
    let send_thread = thread::spawn(move|| {
        read_2bit(&src_sender, &Path::new(&run_info.genome_path)).unwrap();
    });
    let result_count = thread::spawn(move|| {
        let mut count:usize = 0;
        for chunk in dest_receiver.iter(){
            count += chunk.len();
        }
        count
    });
    
    let run_config = match OclRunConfig::new(run_info.dev_ty){
        Err(err)=>panic!("OpenCL runtime errored on load with error: {}",err.to_string()),
        Ok(cfg)=>cfg
    };
    let reversed_byte_patterns:Vec<Vec<u8>> = run_info.patterns.iter().map(|v|reverse_compliment_char(&v)).collect();
    let mut all_patterns:Vec<Vec<u8>> = run_info.patterns.clone();
    all_patterns.extend_from_slice(&reversed_byte_patterns);

    let all_patterns_4bit:Vec<Vec<u8>> = all_patterns.iter().map(|pat|{
        let mut buf = vec![0 as u8; cdiv(pat.len(),2)];
        string_to_bit4(&mut buf, pat, 0, true);
        buf
    })
    .collect();
    
    search(run_config,run_info.max_mismatches, run_info.pattern_len, &all_patterns_4bit,src_receiver, dest_sender);
    send_thread.join().unwrap();
    let out = result_count.join().unwrap();

    println!("{}",out);
    // assert_eq!(result_count.join().unwrap(), expected_results);
}

