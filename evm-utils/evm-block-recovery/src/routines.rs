/// Module containing routines for recovering the EVM chain state. Includes functions for checking, comparing, finding, repeating, and restoring blocks.
pub(crate) mod check_evm;
pub(crate) mod check_native;
pub(crate) mod compare;
pub(crate) mod find;
pub(crate) mod repeat;
pub(crate) mod restore_chain;
pub(crate) mod upload;

pub use check_evm::check_evm;
pub use check_native::check_native;
pub use compare::compare_native;
pub use find::{find_evm, find_native};
pub use repeat::{repeat_evm, repeat_native};
pub use restore_chain::restore_chain;
pub use upload::upload;

use anyhow::*;
/// Finds ranges of missing blocks between the provided confirmed blocks 
/// and the given start/end block range. Returns a vector of BlockRange 
/// structs representing the missing ranges.
///
/// The BlockRange struct represents either a single missing block or 
/// an inclusive range of missing blocks.
///
/// The find_uncommitted_ranges function is useful for identifying gaps  
/// in chain data that need to be recovered.

async fn write_blocks_collection(
    ledger: &nexis_storage_bigtable::LedgerStorage,
    blocks: Vec<evm_state::Block>,
) -> Result<()> {
    for block in blocks {
        log::info!(
            "Writing block {} with hash {} to the Ledger...",
            block.header.block_number,
            block.header.hash()
        );

        let block_num = block.header.block_number;

        // TODO: informative message if early-return
        ledger
            .upload_evm_block(block_num, block)
            .await
            .context(format!("Unable to write block {block_num} to bigtable"))?;
    }

    Ok(())
}

fn find_uncommitted_ranges(blocks: Vec<u64>, start_block: u64, end_block: u64) -> Vec<BlockRange> {
    let mut result = Vec::new();
    let first_block = if let Some(b) = blocks.first() {
        *b
    } else {
        return vec![BlockRange::new(start_block, end_block)];
    };
    let last_block = *blocks.last().unwrap();
    if start_block < first_block {
        let missing_range = BlockRange::new(start_block, first_block);
        result.push(missing_range);
    }
    for blocks in blocks.windows(2) {
        let previous = blocks[0];
        let current = blocks[1];

        if current - previous != 1 {
            let first = previous + 1;
            let last = current - 1;
            let missing_range = BlockRange::new(first, last);
            // log::info!("Found missing {missing_range}");
            result.push(missing_range);
        }
    }
    if end_block > last_block {
        let missing_range = BlockRange::new(last_block, end_block);
        result.push(missing_range);
    }

    result
}

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize)]
pub enum BlockRange {
    SingleBlock(u64),
    InclusiveRange(u64, u64),
}

impl BlockRange {
    pub fn new(first: u64, last: u64) -> Self {
        if first > last {
            panic!("The last block ID should be greater or equal to the first block ID")
        }
        if first == last {
            return Self::SingleBlock(first);
        }
        Self::InclusiveRange(first, last)
    }

    pub fn first(&self) -> u64 {
        match self {
            BlockRange::SingleBlock(single) => *single,
            BlockRange::InclusiveRange(first, _) => *first,
        }
    }

    pub fn last(&self) -> u64 {
        match self {
            BlockRange::SingleBlock(single) => *single,
            BlockRange::InclusiveRange(_, last) => *last,
        }
    }
}

impl std::fmt::Display for BlockRange {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockRange::SingleBlock(s) => write!(fmt, "single block: {s}"),
            BlockRange::InclusiveRange(f, l) => write!(fmt, "inclusive range: [{f}; {l}]"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_missing_blocks() {
        let confirmed_blocks = vec![1, 2, 3, 8, 9, 10];
        assert_eq!(
            find_uncommitted_ranges(confirmed_blocks, 1, 10),
            vec![BlockRange::InclusiveRange(4, 7)]
        )
    }

    #[test]
    fn test_find_missing_blocks_multirange() {
        let confirmed_blocks = vec![1, 2, 5, 6, 10, 11, 13];
        assert_eq!(
            find_uncommitted_ranges(confirmed_blocks, 1, 13),
            vec![
                BlockRange::InclusiveRange(3, 4),
                BlockRange::InclusiveRange(7, 9),
                BlockRange::SingleBlock(12)
            ]
        );
    }
}
