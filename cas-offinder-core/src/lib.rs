// #![const_eval_limit = "100000000"]  

mod bit4ops;
mod chrom_chunk;
mod read_fasta;
mod read_bit2;
mod read_fasta_folder;
mod cli_err;

pub use crate::bit4ops::*;
pub use crate::read_bit2::*;
pub use crate::read_fasta::*;
pub use crate::read_fasta_folder::*;
pub use crate::cli_err::*;

pub fn addone(x:i64) -> i64{
    x+1
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
 
    #[test]
    fn test_add_one() {
        assert_eq!(addone(42), 43);
    }
}