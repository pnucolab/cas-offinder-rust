mod cli_utils;

use crate::cli_utils::parse_and_validate_args;
use crate::cli_utils::SearchRunInfo;
use cas_offinder_lib::*;
use std::env;
use std::fs::File;
use std::io::BufWriter;
use std::io::{LineWriter, Write};
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

fn get_usage(device_strs: &[String]) -> String {
    const PKG_VERSION: &'static str = env!("CARGO_PKG_VERSION");
    // const PKG_EDITION: &'static str = env!("CARGO_PKG_DATETIME");
    const AUTHORS: &'static str = env!("CARGO_PKG_AUTHORS");
    const HOMEPAGE: &'static str = env!("CARGO_PKG_HOMEPAGE");
    let dev_info = device_strs.join("\n");
    format!(
        "
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
        PKG_VERSION, AUTHORS, HOMEPAGE, dev_info
    )
}
fn get_usage_with_devices() -> String {
    let run_config = match OclRunConfig::new(OclDeviceType::ALL) {
        Err(err) => panic!(
            "OpenCL runtime errored on load with error: {}",
            err.to_string()
        ),
        Ok(cfg) => cfg,
    };
    get_usage(&run_config.get_device_strs())
}
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("{}", get_usage_with_devices());
        return;
    }
    let start_time = Instant::now();
    let run_info: SearchRunInfo = parse_and_validate_args(&args).unwrap();

    let (src_sender, src_receiver): (
        mpsc::SyncSender<ChromChunkInfo>,
        mpsc::Receiver<ChromChunkInfo>,
    ) = mpsc::sync_channel(4);
    let (dest_sender, dest_receiver): (mpsc::SyncSender<Vec<Match>>, mpsc::Receiver<Vec<Match>>) =
        mpsc::sync_channel(4);
    let send_thread = thread::spawn(move || {
        read_2bit(&src_sender, &Path::new(&run_info.genome_path)).unwrap();
    });
    let result_count = thread::spawn(move || {
        let out_writer = if run_info.out_path != "-" {
            Box::new(File::create(run_info.out_path).unwrap()) as Box<dyn Write>
        } else {
            Box::new(std::io::stdout()) as Box<dyn Write>
        };
        let mut out_buf_writer = BufWriter::new(out_writer);
        let mut search_filter_buf = vec![0 as u8; cdiv(run_info.pattern_len, 2)];
        string_to_bit4(&mut search_filter_buf, &run_info.search_filter, 0, true);
        let mut dna_buf = vec![0 as u8; cdiv(run_info.pattern_len, 2)];
        let mut marked_dna_buf: Vec<u8> = vec![0 as u8; run_info.pattern_len];
        for chunk in dest_receiver.iter() {
            for m in chunk {
                dna_buf.fill(0);
                string_to_bit4(&mut dna_buf, &m.dna_seq, 0, false);
                let n_search_matches: u32 = dna_buf
                    .iter()
                    .zip(search_filter_buf.iter())
                    .map(|(x1, x2)| (*x1 & *x2).count_ones())
                    .sum();
                if n_search_matches as usize == run_info.pattern_len {
                    let dir = if m.is_forward { '+' } else { '-' };
                    marked_dna_buf.clone_from_slice(&m.dna_seq);
                    for (dnac, rnac) in marked_dna_buf.iter_mut().zip(m.rna_seq.iter()) {
                        if !cmp_chars(*dnac, *rnac) {
                            *dnac |= !0xdf;
                        }
                    }
                    let rna_str = std::str::from_utf8(&m.rna_seq).unwrap();
                    let dna_str = std::str::from_utf8(&marked_dna_buf).unwrap();
                    write!(
                        out_buf_writer,
                        "{}\t{}\t{}\t{}\t{}\t{}\r\n",
                        rna_str, m.chr_name, m.chrom_idx, dna_str, dir, m.mismatches
                    )
                    .unwrap();
                    // .write_fmt(
                    //     format!().as_bytes()
                    // ).unwrap();
                }
            }
        }
    });

    let run_config = match OclRunConfig::new(run_info.dev_ty) {
        Err(err) => panic!(
            "OpenCL runtime errored on load with error: {}",
            err.to_string()
        ),
        Ok(cfg) => cfg,
    };
    let reversed_byte_patterns: Vec<Vec<u8>> = run_info
        .patterns
        .iter()
        .map(|v| reverse_compliment_char(&v))
        .collect();
    let mut all_patterns: Vec<Vec<u8>> = run_info.patterns.clone();
    all_patterns.extend_from_slice(&reversed_byte_patterns);

    let all_patterns_4bit: Vec<Vec<u8>> = all_patterns
        .iter()
        .map(|pat| {
            let mut buf = vec![0 as u8; cdiv(pat.len(), 2)];
            string_to_bit4(&mut buf, pat, 0, true);
            buf
        })
        .collect();

    search(
        run_config,
        run_info.max_mismatches,
        run_info.pattern_len,
        &all_patterns_4bit,
        src_receiver,
        dest_sender,
    );
    send_thread.join().unwrap();
    result_count.join().unwrap();
    let tot_time = start_time.elapsed();
    eprintln!("Completed in {}s", tot_time.as_secs_f64());
    // assert_eq!(result_count.join().unwrap(), expected_results);
}
