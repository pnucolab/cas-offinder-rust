
const T: u8 = 0x1;
const C: u8 = 0x2;
const A: u8 = 0x4;
const G: u8 = 0x8;

const NCHRS:usize = 1<<8;
const NSHRTS: usize = 1<<16;

pub fn cdiv(x:usize, y:usize)->usize{
    (x+y-1)/y
}

const fn makebit4map(mixed_base: bool) -> [u8;NCHRS]{
    let mut arr = [0 as u8;NCHRS];
    arr['G' as usize] = G;
    arr['C' as usize] = C;
    arr['A' as usize] = A;
    arr['T' as usize] = T;
    if mixed_base {
        arr['R' as usize] = A | G;
        arr['Y' as usize] = C | T;
        arr['S' as usize] = G | C;
        arr['W' as usize] = A | T;
        arr['K' as usize] = G | T;
        arr['M' as usize] = A | C;
        arr['B' as usize] = C | G | T;
        arr['D' as usize] = A | G | T;
        arr['H' as usize] = A | C | T;
        arr['V' as usize] = A | C | G;
        arr['N' as usize] = A | C | G | T;
    }
    return arr;
}
const fn apply_lower(inarr:[u8;NCHRS])->[u8;NCHRS]{
    let mut arr= [0 as u8;NCHRS];
    let mut i = 1;
    while i <= 26{
        arr[i + 96] = inarr[i + 64];
        arr[i + 64] = inarr[i + 64];
        i += 1;
    }
    arr
}
const fn invert_chrmap(inarr:[u8;NCHRS])->[u8;NCHRS]{
    let mut arr= [0 as u8;NCHRS];
    let mut i = 0;
    while i < NCHRS{
        if inarr[i] != 0{
            arr[inarr[i] as usize] = i as u8;
        }
        i += 1;
    }
    arr
}
const fn create_2bit_block_map()->[u16;NCHRS]{
    let mut arr= [0 as u16;NCHRS];
    let mut i = 0;
    while i < NCHRS{
        let mut val:u16 = 0;
        let mut j = 0;
        while j < 4{
            let bit2val = ((i >> ((4 - j - 1) * 2)) & 0x3);
            val |= (1<<bit2val) << (j*4);
            j += 1;
        }
        arr[i] = val;
        i += 1;
    }
    arr
}
const fn doubleup_patternmap(inarr:[u8;NCHRS])->[u8;NSHRTS]{
    let mut arr = [0 as u8;NSHRTS];
    let mut i = 0;
    let mut outidx = 0;
    while i < NCHRS{
        let mut j = 0;
        let shftval = inarr[i] << 4;
        while j < NCHRS{
            arr[outidx] = inarr[j] | shftval;
            outidx += 1;
            j += 1;
        }
        i += 1;
    }
    arr
}
const fn invert_double_patternmap(inarr:[u8;NSHRTS])->[u16;NCHRS]{
    let mut arr = [0 as u16; NCHRS];
    let mut i = 0;
    while i < NSHRTS{
        if inarr[i] != 0{
            arr[inarr[i]as usize] = i as u16;
        }
    }
    arr
}
pub fn bit4_to_string(out_data: &mut[u8], data:&[u8], read_offset: usize, n_chrs: usize){
    const bit4_map:[u8;NCHRS] = invert_chrmap(makebit4map(false));
    let src = &data[(read_offset/2)..];
    let r_offset = read_offset%2;
    if r_offset != 0 && n_chrs > 0{
        out_data[0] = bit4_map[((src[0] & 0xf0) >> 4) as usize];
        bit4_to_string(&mut out_data[1..], &src[1..], 0, n_chrs-1);
    }
    else{
        for i in 0..n_chrs/2{
            out_data[i*2] = bit4_map[(src[i]&0x0f) as usize];
            out_data[i*2+1] = bit4_map[((src[i]&0xf0) >> 4) as usize];
        }
        if n_chrs%2 == 1 {
            out_data[n_chrs-1] = bit4_map[(src[n_chrs/2]&0x0f) as usize];
        }
    }

}
pub fn string_to_bit4(out_data: &mut[u8], data:&[u8], write_offset: usize, mixed_base: bool){
    const strmaps: [[u8; NCHRS]; 2] = [
        apply_lower(makebit4map(false)),
        apply_lower(makebit4map(true))
    ];
    const dblstrmaps: [[u8; NSHRTS]; 2] = [
        doubleup_patternmap(strmaps[0]),
        doubleup_patternmap(strmaps[1]),
    ];
    let n_chrs = data.len();
    let dest = &mut out_data[write_offset/2..];
    if write_offset%2 != 0 && n_chrs > 0{
        dest[0] |= strmaps[mixed_base as usize][data[0] as usize] << 4;
        string_to_bit4(&mut dest[1..], &data[1..], 0, mixed_base);
    }
    else{
        assert!(dest.len() <= cdiv(n_chrs, 2));
        if n_chrs % 2 != 0{
            dest[n_chrs/2] |= strmaps[mixed_base as usize][data[n_chrs-1] as usize];
        }
        unsafe{
            let srcptr = data.as_ptr();
            let dsrcptr = srcptr as *const u16;
            for i in 0..(n_chrs/2){
                *dest.get_unchecked_mut(i) = dblstrmaps[mixed_base as usize][(*(dsrcptr.add(i))) as usize];
                // dest[i] = strmaps[mixed_base as usize][data[i*2] as usize] | (strmaps[mixed_base as usize][data[i*2+1] as usize] << 4);
            }
        }
    }
}
pub fn bit2_to_bit4(out_data: &mut[u8], data:&[u8], n_chrs: usize){
    const bit2map:[u16;256] = create_2bit_block_map();
    assert!(n_chrs % 4 == 0, "twobit2bit4 only support block of 4 writes");
    assert!(out_data.as_ptr().align_offset(2) == 0,
           "dest must be alligned to 2 byte boundaries");
    assert!(out_data.len() *2 == n_chrs);
    assert!(data.len() * 4 == n_chrs);
    let n_blks = n_chrs/4;
    unsafe{
        let blkdest = out_data.as_mut_ptr() as *mut u16; 
        for i in 0..n_blks{
            *blkdest.add(i) = bit2map[*data.get_unchecked(i) as usize];
        }
    }
}
pub fn memsetbit4(dest: &mut[u8], bit4val:u8, start:usize, end:usize){
    assert!(bit4val <= 0xf);
    if (start < end && start % 2 == 1) {
        dest[start / 2] &= 0xf;
        dest[start / 2] |= bit4val << 4;
        memsetbit4(dest, bit4val, start+1, end);
    }
    else if (start < end && end % 2 == 1) {
        dest[end / 2] &= 0xf0;
        dest[end / 2] |= bit4val;
        memsetbit4(dest, bit4val, start, end-1);
    }
    else if (start < end) {
        let bstart = start / 2;
        let bend = end / 2;
        let bval = bit4val | (bit4val << 4);
        for item in &mut dest[bstart..bend]{
            *item = bval;
        }
    }
}


#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    
    pub fn string_to_bit4_fn(data:&[u8], offset: usize, mixed_base:bool)->Vec<u8>{
        let size = cdiv(data.len() + offset, 2);
        let mut outarr = vec![0 as u8;size];
        string_to_bit4(&mut outarr, data, offset, mixed_base);
        outarr        
    }
    
    pub fn bit4_to_string_fn(data:&[u8], offset: usize, n_chrs: usize)->Vec<u8>{
        let mut outarr = vec![0 as u8;n_chrs];
        bit4_to_string(&mut outarr, data, offset, n_chrs);
        outarr
    }

    #[test]
    fn test_str2bit4() {
        let input_data =  b"ACtGc";
        let expected_out:[u8;3] = [0x24, 0x81, 0x02];
        let mut actual_out = [0 as u8;3];
        let offset = 0;
        let mixed_base = true;
        string_to_bit4(&mut actual_out, input_data,offset,mixed_base);
        assert_eq!(actual_out, expected_out);
    }
    #[test]
    fn test_str2bit4_offset_1() {
        let input_data =  b"ACTGC";
        let expected_out:[u8;3] = [ 0x40, 0x12, 0x28];
        let mut actual_out = [0 as u8;3];
        let offset = 1;
        let mixed_base = true;
        string_to_bit4(&mut actual_out, input_data,offset,mixed_base);
        assert_eq!(actual_out, expected_out);
    }
    #[test]
    fn test_str2bit4_offset_3() {
        let input_data =  b"ACTGC";
        let expected_out:[u8;4] = [ 0x00, 0x40, 0x12, 0x28];
        let mut actual_out = [0 as u8;4];
        let offset = 3;
        let mixed_base = true;
        string_to_bit4(&mut actual_out, input_data,offset,mixed_base);
        assert_eq!(actual_out, expected_out);
    }
    #[test]
    fn test_str2bit4_large() {
        let input_data =  b"ACTGCAACTGCA";
        let expected_out:[u8;6] = [ 0x24, 0x81, 0x42, 0x24, 0x81, 0x42 ];
        let mut actual_out = [0 as u8;6];
        let offset = 0;
        let mixed_base = true;
        string_to_bit4(&mut actual_out, input_data,offset,mixed_base);
        assert_eq!(actual_out, expected_out);
    }
    #[test]
    fn test_str2bit4_mixedbase() {
        let input_data =  b"ARGN";
        let expected_out:[u8;2] = [0xc4, 0xf8];
        let mut actual_out = [0 as u8;2];
        let offset = 0;
        let mixed_base = true;
        string_to_bit4(&mut actual_out, input_data,offset,mixed_base);
        assert_eq!(actual_out, expected_out);
    }
    #[test]
    fn test_str2bit4_mixedbase_off() {
        let input_data =  b"ARGN";
        let expected_out:[u8;2] = [0x04, 0x08];
        let mut actual_out = [0 as u8;2];
        let offset = 0;
        let mixed_base = false;
        string_to_bit4(&mut actual_out, input_data,offset,mixed_base);
        assert_eq!(actual_out, expected_out);
    }
    #[test]
    fn test_bit42str() {
        let input = [0x24, 0x81, 0x02];
        let expected_out = b"ACTGC";
        let out_size = expected_out.len();
        let mut actual_out = vec![0 as u8;out_size];
        let read_offset = 0;
        bit4_to_string(&mut actual_out, &input, read_offset, out_size);
        assert_eq!(actual_out, expected_out);
    }
    #[test]
    fn test_bit42str_offset1() {
        let input = [0x40, 0x12, 0x28];
        let expected_out = b"ACTGC";
        let out_size = expected_out.len();
        let mut actual_out = vec![0 as u8;out_size];
        let read_offset = 1;
        bit4_to_string(&mut actual_out, &input, read_offset, out_size);
        assert_eq!(actual_out, expected_out);
    }
    #[test]
    fn test_bit42str_offset3() {
        let input = [0x00, 0x40, 0x12, 0x28];
        let expected_out = b"ACTGC";
        let out_size = expected_out.len();
        let mut actual_out = vec![0 as u8;out_size];
        let read_offset = 3;
        bit4_to_string(&mut actual_out, &input, read_offset, out_size);
        assert_eq!(actual_out, expected_out);
    }
    #[test]
    fn test_memsetbit4_middle() {
        let mut input = [0x24, 0x81, 0x42, 0x02];
        let expected = [0x04, 0x00, 0x40, 0x02];
        memsetbit4(&mut input, 0, 1, 5);
        assert_eq!(input, expected);
    }
    #[test]
    fn test_memsetbit4_edge() {
        let mut input = [0x24, 0x81, 0x42, 0x02];
        let expected = [0x04, 0x80, 0x42, 0x02 ];
        memsetbit4(&mut input, 0, 1, 3);
        assert_eq!(input, expected);
    }
    #[test]
    fn test_memsetbit4_chnk() {
        let mut input = [0x24, 0x81, 0x42, 0x02];
        let expected = [0x24, 0xff, 0x42, 0x02 ];
        memsetbit4(&mut input, 0xf, 2, 4);
        assert_eq!(input, expected);
    }
    #[test]
    fn test_twobit2bit4() {
        let expected: [u8;4] = [ 0x24, 0x81, 0x81, 0x24 ];
        let input:[u8;2] = [
            (2 << 6) | (1 << 4) | (0 << 2) | (3 << 0),
            (0 << 6) | (3 << 4) | (2 << 2) | (1 << 0)
        ];
        let mut result = vec![0 as u8;expected.len()];
        bit2_to_bit4(&mut result, &input, input.len()*4);
        assert_eq!(result, expected);
    }
}