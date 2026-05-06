mod cli_utils;

use crate::cli_utils::parse_and_validate_args;
use crate::cli_utils::SearchRunInfo;
use cas_offinder_lib::*;
use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;
use std::fs;

fn get_usage(device_strs: &[String]) -> String {
    const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
    // const PKG_EDITION: &'static str = env!("CARGO_PKG_DATETIME");
    const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
    const HOMEPAGE: &str = env!("CARGO_PKG_HOMEPAGE");
    let dev_info = device_strs.join("\n");
    format!(
        "
Cas-OFFinder 2 - v{}

Copyright (c) 2023 {}
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
        Err(err) => panic!("OpenCL runtime errored on load with error: {}", err),
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

    // Capture data needed later (log + result writer) before run_info fields get moved.
    let out_path_clone = run_info.out_path.clone();
    let search_filter_clone = run_info.search_filter.clone();
    let pattern_len_clone = run_info.pattern_len;
    let max_mismatches_clone = run_info.max_mismatches;
    let use_myers = run_info.max_dna_bulges > 0 || run_info.max_rna_bulges > 0;
    // PAM length = positions where the user's crRNA patterns hold N placeholders
    // (PAM is always at the end in the original crRNA orientation). Counted from
    // trailing N's of the first pattern; all patterns are padded identically.
    let pam_len_clone: u64 = {
        let first = &run_info.patterns[0];
        let mut count = 0u64;
        for &c in first.iter().rev() {
            if c == b'N' || c == b'n' {
                count += 1;
            } else {
                break;
            }
        }
        count
    };
    let log_out_path = run_info.out_path.clone();
    let log_genome_path = run_info.genome_path.clone();
    let log_n_patterns = run_info.patterns.len();
    // pattern_labels[user_guide_idx] — used for the per-row Id column.
    // Match cas-offinder C++: pattern_idx in [0, n_user) is the forward copy,
    // [n_user, 2*n_user) is the reverse-complement; both map to the same
    // user-supplied label via `pattern_idx % n_user`.
    let pattern_labels = run_info.pattern_labels.clone();
    let n_user_patterns = run_info.patterns.len();
    let log_search_filter = run_info.search_filter.clone();
    let log_max_mismatches = run_info.max_mismatches;
    let log_max_dna_bulges = run_info.max_dna_bulges;
    let log_max_rna_bulges = run_info.max_rna_bulges;
    let log_device_label = match run_info.dev_ty {
        OclDeviceType::CPU => "CPU".to_string(),
        OclDeviceType::GPU => "GPU".to_string(),
        OclDeviceType::ACCEL => "Accelerator".to_string(),
        OclDeviceType::ALL => "All".to_string(),
    };

    let send_thread = thread::spawn(move || {
        let genome_path = Path::new(&run_info.genome_path);
        let is_folder = fs::metadata(genome_path).unwrap().is_dir();
        if is_folder {
            read_fasta_folder(&src_sender, genome_path).unwrap();
        } else {
            let mut file = File::open(genome_path).unwrap();
            let mut first_byte = [0_u8; 1];
            file.read_exact(&mut first_byte).unwrap();
            if first_byte[0] == b'>' {
                read_fasta(&src_sender, genome_path).unwrap();
            } else {
                read_2bit(&src_sender, genome_path).unwrap();
            }
        }
    });
    let result_count = thread::spawn(move || {
        let out_writer = if out_path_clone != "-" {
            Box::new(File::create(out_path_clone).unwrap()) as Box<dyn Write>
        } else {
            Box::new(std::io::stdout()) as Box<dyn Write>
        };
        // 1 MB buffer (default 8 KB) — far fewer write() syscalls on
        // multi-GB outputs (db≥2 / db+rb mixed configs).
        let mut out_buf_writer = BufWriter::with_capacity(1 << 20, out_writer);

        // Unified cas-offinder-bulge header (always emit). The leading Id
        // column carries the user-supplied label from the input file (or an
        // auto-assigned 0-based index when none was given), matching
        // cas-offinder C++ output.
        out_buf_writer
            .write_all(b"#Id\tBulge type\tcrRNA\tDNA\tChromosome\tPosition\tDirection\tMismatches\tBulge Size\n")
            .unwrap();

        let mut marked_dna_buf: Vec<u8> = vec![0_u8; pattern_len_clone];
        let mut total_matches: u64 = 0;
        // Counter keyed by (mismatches, dna_bulges, rna_bulges). BTreeMap so
        // dump order is deterministic and sorted.
        let mut summary: BTreeMap<(u32, u32, u32), u64> = BTreeMap::new();
        // Reusable scratch buffer for one output line: assembled then
        // write_all'd once, avoiding per-field fmt::Write calls and per-match
        // heap allocations.
        let mut scratch: Vec<u8> = Vec::with_capacity(256);
        // itoa::Buffer is a stack-allocated 40-byte scratch — avoids
        // fmt::Display allocation per integer.
        let mut itoa_buf = itoa::Buffer::new();

        for chunk in dest_receiver.iter() {
            for mut m in chunk {
                let bulge_type: &[u8];
                let bulge_size: u32;
                let dir: u8 = if m.is_forward { b'+' } else { b'-' };
                let rna_bytes: &[u8];
                let dna_bytes: &[u8];

                if use_myers {
                    // Myers path: bulge info already classified, PAM already verified.
                    let total = m.dna_bulge_size + m.rna_bulge_size;
                    bulge_type = if total == 0 {
                        b"X"
                    } else if m.dna_bulge_size > 0 {
                        b"DNA"
                    } else {
                        b"RNA"
                    };
                    bulge_size = total;
                    rna_bytes = &m.rna_seq;
                    dna_bytes = &m.dna_seq;
                } else {
                    // Legacy popcount path (bulge=0): apply post-hoc PAM filter
                    // and mark mismatched DNA bases lowercase (old convention).
                    //
                    // C++ cas-offinder semantics for genome 'N':
                    //   * pattern N (wildcard) + genome anything (incl. N) → match
                    //   * pattern specific + genome N → mismatch, shown lowercase 'n'
                    //   * filter N  + genome anything (incl. N) → PAM filter passes
                    //   * filter specific + genome same → PAM filter passes
                    //   * filter specific + genome N → PAM filter FAILS (reject)
                    //
                    // The kernel's popcount also over-counts mismatches by 1 per
                    // (pattern N, genome N) cell because `0xF & 0 == 0`. We
                    // recompute mismatches here from the raw DNA string.
                    let mut pam_ok = true;
                    let mut actual_mm: u32 = 0;
                    for (pos, &dna_c) in m.dna_seq.iter().enumerate() {
                        let filter_c = search_filter_clone[pos];
                        if !(filter_c == b'N' || filter_c == b'n') {
                            // PAM position: genome must match filter base.
                            // dna_c == 0 (NUL = genome N) always fails here.
                            if !cmp_chars(dna_c, filter_c) {
                                pam_ok = false;
                                break;
                            }
                        }
                        let rna_c = m.rna_seq[pos];
                        if !(rna_c == b'N' || rna_c == b'n') && !cmp_chars(dna_c, rna_c) {
                            actual_mm += 1;
                        }
                    }
                    if !pam_ok {
                        continue;
                    }
                    if actual_mm > max_mismatches_clone {
                        continue;
                    }
                    // m.mismatches was the kernel's popcount-derived count.
                    // Replace it with the C++-consistent actual_mm.
                    m.mismatches = actual_mm;
                    marked_dna_buf.clone_from_slice(&m.dna_seq);
                    for (dnac, rnac) in marked_dna_buf.iter_mut().zip(m.rna_seq.iter()) {
                        let rna_is_n = *rnac == b'N' || *rnac == b'n';
                        let dna_is_n = *dnac == b'N' || *dnac == b'n';
                        if *dnac == 0 {
                            // Shouldn't happen with the new mixed_base=true
                            // genome encoding (padding never decodes here),
                            // but keep a safe fallback: show as pattern-aware
                            // 'N' / 'n'.
                            *dnac = if rna_is_n { b'N' } else { b'n' };
                        } else if rna_is_n {
                            // Pattern N is a wildcard; always a match — keep
                            // uppercase regardless of the genome base.
                        } else if dna_is_n {
                            // Pattern specific + genome N = mismatch; lowercase.
                            *dnac = b'n';
                        } else if !cmp_chars(*dnac, *rnac) {
                            *dnac |= !0xdf;
                        }
                    }
                    bulge_type = b"X";
                    bulge_size = 0;
                    rna_bytes = &m.rna_seq;
                    dna_bytes = &marked_dna_buf;
                }

                // Match original cas-offinder position convention:
                //   + strand: leftmost + strand coord (= Rust internal)
                //   - strand: shifted by (PAM_len - 1 - dna_bulge_size)
                let pos_out = if m.is_forward {
                    m.chrom_idx
                } else {
                    m.chrom_idx
                        + pam_len_clone
                            .saturating_sub(1)
                            .saturating_sub(m.dna_bulge_size as u64)
                };

                // Map back from the internal pattern index (forward + RC
                // copies stored consecutively) to the user-supplied guide
                // label.
                let user_idx = (m.pattern_idx as usize) % n_user_patterns;
                let label = pattern_labels[user_idx].as_bytes();

                // Assemble one full row in `scratch`, then a single write_all.
                scratch.clear();
                scratch.extend_from_slice(label);
                scratch.push(b'\t');
                scratch.extend_from_slice(bulge_type);
                scratch.push(b'\t');
                scratch.extend_from_slice(rna_bytes);
                scratch.push(b'\t');
                scratch.extend_from_slice(dna_bytes);
                scratch.push(b'\t');
                scratch.extend_from_slice(m.chr_name.as_bytes());
                scratch.push(b'\t');
                scratch.extend_from_slice(itoa_buf.format(pos_out).as_bytes());
                scratch.push(b'\t');
                scratch.push(dir);
                scratch.push(b'\t');
                scratch.extend_from_slice(itoa_buf.format(m.mismatches).as_bytes());
                scratch.push(b'\t');
                scratch.extend_from_slice(itoa_buf.format(bulge_size).as_bytes());
                scratch.push(b'\n');
                out_buf_writer.write_all(&scratch).unwrap();

                // Tally for the .summary.txt file.
                *summary
                    .entry((m.mismatches, m.dna_bulge_size, m.rna_bulge_size))
                    .or_insert(0) += 1;

                total_matches += 1;
            }
        }
        out_buf_writer.flush().unwrap();
        (total_matches, summary)
    });

    let run_config = match OclRunConfig::new(run_info.dev_ty) {
        Err(err) => panic!("OpenCL runtime errored on load with error: {}", err),
        Ok(cfg) => cfg,
    };
    let reversed_byte_patterns: Vec<Vec<u8>> = run_info
        .patterns
        .iter()
        .map(|v| reverse_compliment_char(v))
        .collect();
    let mut all_patterns: Vec<Vec<u8>> = run_info.patterns.clone();
    all_patterns.extend_from_slice(&reversed_byte_patterns);

    search(
        run_config,
        run_info.max_mismatches,
        run_info.max_dna_bulges,
        run_info.max_rna_bulges,
        &run_info.search_filter,
        run_info.pattern_len,
        &all_patterns,
        src_receiver,
        dest_sender,
    );
    send_thread.join().unwrap();
    let (total_matches, summary) = result_count.join().unwrap();
    let tot_time = start_time.elapsed();
    eprintln!("Completed in {}s", tot_time.as_secs_f64());

    // Write _summary.txt next to the output (skip when writing to stdout).
    // Naming: replace the output filename's extension with `_summary.txt`
    // (e.g. `out.txt` -> `out_summary.txt`, `path/run.tsv` ->
    // `path/run_summary.txt`, `output` (no ext) -> `output_summary.txt`).
    // Format: TSV with one row per (mm, db, rb) tuple plus a total comment.
    if log_out_path != "-" {
        let summary_path = {
            let path = std::path::Path::new(&log_out_path);
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            let summary_name = format!("{}_summary.txt", stem);
            match path.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => parent
                    .join(summary_name)
                    .to_string_lossy()
                    .into_owned(),
                _ => summary_name,
            }
        };
        if let Err(e) = write_summary_file(&summary_path, total_matches, &summary) {
            eprintln!(
                "warning: failed to write summary file '{}': {}",
                summary_path, e
            );
        }
    }

    // Write _log.txt file alongside the output (skip when writing to stdout).
    // Same naming rule as the summary file: strip the output filename's
    // extension and append `_log.txt`.
    if log_out_path != "-" {
        let log_path = {
            let path = std::path::Path::new(&log_out_path);
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            let log_name = format!("{}_log.txt", stem);
            match path.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => parent
                    .join(log_name)
                    .to_string_lossy()
                    .into_owned(),
                _ => log_name,
            }
        };
        let genome_size = fs::metadata(&log_genome_path).map(|m| m.len()).unwrap_or(0);
        let algorithm = if use_myers {
            "Myers bit-parallel".to_string()
        } else {
            "popcount (legacy)".to_string()
        };
        let log = RunLog {
            genome_path: log_genome_path,
            genome_size,
            n_patterns: log_n_patterns,
            search_filter: std::str::from_utf8(&log_search_filter)
                .unwrap_or("")
                .to_string(),
            max_mismatches: log_max_mismatches,
            max_dna_bulges: log_max_dna_bulges,
            max_rna_bulges: log_max_rna_bulges,
            device_label: log_device_label,
            algorithm,
            n_matches: total_matches,
            total_elapsed_secs: tot_time.as_secs_f64(),
        };
        if let Err(e) = write_log(&log_path, &log) {
            eprintln!("warning: failed to write log file '{}': {}", log_path, e);
        }
    }
}

/// Dump the per-(mismatch, dna_bulge, rna_bulge) match counter to a TSV
/// file alongside the main output. Header rows start with `#`.
fn write_summary_file(
    path: &str,
    total: u64,
    summary: &BTreeMap<(u32, u32, u32), u64>,
) -> std::io::Result<()> {
    let mut f = BufWriter::new(File::create(path)?);
    writeln!(
        f,
        "# Match counts by (mismatches, dna_bulges, rna_bulges)"
    )?;
    writeln!(f, "# total: {}", total)?;
    writeln!(f, "mm\tdb\trb\tcount")?;
    for ((mm, db, rb), count) in summary {
        writeln!(f, "{}\t{}\t{}\t{}", mm, db, rb, count)?;
    }
    f.flush()?;
    Ok(())
}
