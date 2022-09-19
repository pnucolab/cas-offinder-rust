use cas_offinder_lib::*;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
fn get_dev_ty(arg: &str) -> Result<OclDeviceType> {
    let dev_parse_err = CliError::ArgumentError("2nd argument must be one of {{C|G|A}}");
    if arg.len() != 1 {
        return Err(dev_parse_err);
    }
    let firstc = arg.chars().next().unwrap();
    match firstc {
        'C' => Ok(OclDeviceType::CPU),
        'G' => Ok(OclDeviceType::GPU),
        'A' => Ok(OclDeviceType::ACCEL),
        _ => Err(dev_parse_err),
    }
}
pub struct SearchRunInfo {
    pub genome_path: String,
    pub out_path: String,
    pub dev_ty: OclDeviceType,
    pub search_filter: Vec<u8>,
    pub patterns: Vec<Vec<u8>>,
    pub pattern_infos: Vec<String>,
    pub pattern_len: usize,
    pub max_mismatches: u32,
}
struct InFileInfo {
    genome_path: String,
    search_filter: Vec<u8>,
    patterns: Vec<Vec<u8>>,
    pattern_infos: Vec<String>,
    pattern_len: usize,
    max_mismatches: u32,
}
fn parse_and_validate_input(in_path: &String) -> Result<InFileInfo> {
    let file = if in_path != "-" {
        Box::new(File::open(in_path)?) as Box<dyn Read>
    } else {
        Box::new(std::io::stdin()) as Box<dyn Read>
    };
    let file_too_short_err = "Input file must contain at least 3 lines";
    let mixed_base_error =
        CliError::ArgumentError("Pattern in input file needs to be a mixed base string");

    let reader = BufReader::new(file);
    let mut line_iter = reader.lines();
    let genome_path = line_iter
        .next()
        .ok_or(CliError::ArgumentError(file_too_short_err))??;
    let searcher_line = line_iter
        .next()
        .ok_or(CliError::ArgumentError(file_too_short_err))??;
    let search_filter = searcher_line.trim_end().as_bytes().to_vec();
    if !is_mixedbase_str(&search_filter) {
        return Err(mixed_base_error);
    }
    let pattern_len = search_filter.len();

    let mut patterns: Vec<Vec<u8>> = Vec::new();
    let mut pattern_infos: Vec<String> = Vec::new();
    let mut is_using_info_opt: Option<bool> = None;
    let mut mismatches_opt: Option<u32> = None;
    for line_r in line_iter {
        let line = line_r?;
        let lineparts: Vec<&str> = line.split_ascii_whitespace().collect();
        if lineparts.len() != 2 && lineparts.len() != 3 {
            return Err(CliError::ArgumentError(
                "Pattern line must have following elements: <patterns> <mismatches> [<label>]",
            ));
        }
        let pattern_buf = lineparts[0].as_bytes().to_vec();
        if !is_mixedbase_str(&pattern_buf) {
            return Err(mixed_base_error);
        }
        let cur_mismatches: u32 = lineparts[1].parse::<u32>().map_err(|_| {
            CliError::ArgumentError(
                "2nd element of each pattern in mismatches must be an unsigned integer",
            )
        })?;
        mismatches_opt = mismatches_opt.or(Some(cur_mismatches));
        is_using_info_opt = is_using_info_opt.or(Some(lineparts.len() == 3));
        let mismatches = mismatches_opt.unwrap();
        let is_using_info = is_using_info_opt.unwrap();
        if mismatches != cur_mismatches {
            return Err(CliError::ArgumentError("In this version of cas-offinder, all mismatches on every line of input file must be the same"));
        }
        let cur_info = if is_using_info {
            lineparts.get(2).ok_or(CliError::ArgumentError("Pattern lines in input file must be consistently have either 2 or 3 elements, no mixing and matching allowed"))?
        } else {
            lineparts[0]
        };

        if pattern_buf.len() != pattern_len {
            return Err(CliError::ArgumentError(
                "All patters in input file must be same length",
            ));
        }
        patterns.push(pattern_buf);
        pattern_infos.push(cur_info.to_string());
    }
    match mismatches_opt {
        None => Err(CliError::ArgumentError(
            "Input file must contain at least 1 pattern line",
        )),
        Some(max_mismatches) => Ok(InFileInfo {
            genome_path,
            search_filter,
            patterns,
            pattern_infos,
            pattern_len,
            max_mismatches,
        }),
    }
}
pub fn parse_and_validate_args(args: &Vec<String>) -> Result<SearchRunInfo> {
    //Usage: cas-offinder {{input_file/directory}} {{mismatches}} {{C|G|A}}[device_id(s)] {{output_file}} {{pattern1}} [{{optional patterns}}...]
    if args.len() < 4 {
        return Err(CliError::ArgumentError(
            "Too few arguments, expected 3 arguments",
        ));
    }
    let in_filename = &args[1];
    let device_ty_str = &args[2];
    let out_filename = &args[3];
    let parsed_in_file = parse_and_validate_input(in_filename)?;
    Ok(SearchRunInfo {
        genome_path: parsed_in_file.genome_path,
        search_filter: parsed_in_file.search_filter,
        patterns: parsed_in_file.patterns,
        pattern_infos: parsed_in_file.pattern_infos,
        pattern_len: parsed_in_file.pattern_len,
        max_mismatches: parsed_in_file.max_mismatches,
        out_path: out_filename.clone(),
        dev_ty: get_dev_ty(device_ty_str)?,
    })
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_str2bit4() {
        let input_data = b"ACtGc";
        let expected_out: [u8; 3] = [0x24, 0x81, 0x02];
        let mut actual_out = [0_u8; 3];
        let offset = 0;
        let mixed_base = true;
        string_to_bit4(&mut actual_out, input_data, offset, mixed_base);
        assert_eq!(actual_out, expected_out);
    }
}
