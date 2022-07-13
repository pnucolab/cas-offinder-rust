use std::io::{Read, BufReader};
use std::fs::File;
use std::path::Path;
use std::cmp::{min, max};
use std::sync::mpsc::{Sender,};
use crate::cdiv;
use crate::{bit2_to_bit4,memsetbit4};
use crate::chrom_chunk::{ChromChunkInfo, CHUNK_SIZE, CHUNK_SIZE_BYTES};
use crate::cli_err::{Result,CliError};

fn read_u32(reader: &mut BufReader<std::fs::File>)->Result<u32>{
    let mut buf = [0 as u8; 4];
    reader.read_exact(&mut buf)?;
    unsafe{
    return Ok(std::mem::transmute::<[u8;4],u32>(buf));
    }
}
fn read_u8(reader: &mut BufReader<std::fs::File>)->Result<u8>{
    let mut buf = [0 as u8; 1];
    reader.read_exact(&mut buf)?;
    return Ok(buf[0]);
}
fn read_str(reader: &mut BufReader<std::fs::File>, n_bytes: usize)->Result<String>{
    let mut str_buf = vec![0 as u8; n_bytes];
    reader.read_exact(&mut str_buf)?;
    Ok(String::from_utf8(str_buf)?)
}
fn read_intvec(reader: &mut BufReader<std::fs::File>, n_els: usize)->Result<Vec<u32>>{
    let mut int_buf = Vec::with_capacity(n_els);
    for _ in 0..n_els{
        int_buf.push(read_u32(reader)?);
    }
    Ok(int_buf)
}
pub fn read_2bit(dest:&Sender<ChromChunkInfo>, fname: &Path)->Result<()>{
    let file = File::open(fname)?;
    let buf_capacity = CHUNK_SIZE;
    let mut reader = BufReader::with_capacity(buf_capacity,file);
    let headerval = read_u32(&mut reader)?;
    if headerval != 0x1A412743{
        return Err(CliError::BadFileFormat(".2bit file badly formatted header"));
    }
    let version_num = read_u32(&mut reader)?;
    if version_num != 0{ // Version should be 0
        return Err(CliError::BadFileFormat("only supports version 0 of .2bit format"));
    }
    let chrcnt = read_u32(&mut reader)?;
    reader.seek_relative(4)?;// skip reserved bits

    let mut chrom_names:Vec<String> = Vec::with_capacity(chrcnt as usize);
    for _ in 0..chrcnt{
        let len_chrname = read_u8(&mut reader)?;
        let chromname = read_str(&mut reader,len_chrname as usize)?;
        chrom_names.push(chromname);
        reader.seek_relative(4)?;// Absolute position of each sequence
    }
    for chrname in chrom_names.iter(){
        let chrlen = read_u32(&mut reader)? as usize;
        let nblockcnt = read_u32(&mut reader)? as usize;

        let nblockstart = read_intvec(&mut reader, nblockcnt)?;
        let nblocksizes = read_intvec(&mut reader, nblockcnt)?;
        let mut nblocks:Vec<(u32,u32)> = nblockstart.iter().zip(nblocksizes.iter()).map(|(a1,a2)|(*a1,*a2)).collect();
        nblocks.sort_by_key(|(start,_size)|{*start});

        let maskblockcnt = read_u32(&mut reader)?;
        // skip mask infos
        reader.seek_relative((maskblockcnt*8+4) as i64)?;

        assert!(CHUNK_SIZE%4 == 0);
        const NUCL_PER_BYTE: usize = 4;
        const RAW_BUF_LEN: usize = CHUNK_SIZE/NUCL_PER_BYTE;
        let mut raw_buf = [0 as u8;RAW_BUF_LEN];
        let rawlen = cdiv(chrlen, NUCL_PER_BYTE);
        let mut read_pos = 0;
        let mut block_mask_idx: i64 = 0;

        while read_pos < rawlen{
            let read_size = min(rawlen - read_pos, RAW_BUF_LEN);
            let cur_data_size = read_size*NUCL_PER_BYTE;
            reader.read_exact(&mut raw_buf[..read_size])?;
            let mut chrdata = Box::new([0 as u8; CHUNK_SIZE_BYTES]);
            bit2_to_bit4(&mut chrdata[..],&raw_buf, read_size);
            memsetbit4(&mut chrdata[..], 0, chrlen - (read_pos+read_size)*NUCL_PER_BYTE, read_size*NUCL_PER_BYTE);
            let chunk_start = (read_pos*NUCL_PER_BYTE) as u64;
            let chunk_end = ((read_pos+read_size)*NUCL_PER_BYTE) as u64;
            //go back one in case previous zone overlaps with current block
            block_mask_idx = max(block_mask_idx-1, 0);
            loop{ 
                let (bstart, bsize) = nblocks[block_mask_idx as usize];
                let block_chunk_start = bstart as i64 - chunk_start as i64;
                let block_chunk_end = (bstart + bsize) as i64 - chunk_start as i64;
                if block_chunk_start > cur_data_size as i64{
                    break;
                }
                memsetbit4(&mut chrdata[..], 0, max(0,block_chunk_start) as usize, min(max(0,block_chunk_end) as usize, cur_data_size));
                block_mask_idx += 1;
            }
            let chrinfo = ChromChunkInfo{
                chr_name: chrname.clone(),
                chunk_start: chunk_start,
                chunk_end: chunk_end,
                data: chrdata,
            };
            dest.send(chrinfo)?;
            read_pos += read_size;
        }
    }

    Ok(())
}