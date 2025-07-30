use chia::protocol::Bytes32;
use chia_puzzle_types::LineageProof;
use clvm_traits::{FromClvm, ToClvm};

use crate::PartialPriceData;

#[derive(FromClvm, ToClvm, Debug, Clone, Copy, PartialEq, Eq)]
#[clvm(list)]
pub struct PartialOfferAssetInfo {
    pub asset_id: Option<Bytes32>,
    pub hidden_puzzle_hash: Option<Bytes32>,
    // Last element - list terminator - allows for future extension
    // Since #[clvm(rest)] was not used
}

impl PartialOfferAssetInfo {
    pub fn new(asset_id: Option<Bytes32>, hidden_puzzle_hash: Option<Bytes32>) -> Self {
        Self {
            asset_id,
            hidden_puzzle_hash,
        }
    }

    pub fn xch() -> Self {
        Self {
            asset_id: None,
            hidden_puzzle_hash: None,
        }
    }

    pub fn cat(asset_id: Bytes32, hidden_puzzle_hash: Option<Bytes32>) -> Self {
        Self {
            asset_id: Some(asset_id),
            hidden_puzzle_hash,
        }
    }
}

// Partial coin parent & amount found in the hinted coin info
//   (puzzle hash = 0101..01)
#[derive(FromClvm, ToClvm, Debug, Clone, PartialEq, Eq)]
#[clvm(list)]
pub struct PartialOfferHint<IC> {
    pub lineage_proof: Option<LineageProof>,
    pub offered_asset_info: PartialOfferAssetInfo,
    pub requested_asset_info: PartialOfferAssetInfo,
    pub price_data: PartialPriceData,
    pub maker_puzzle_hash: Bytes32,
    pub inner_conditions: IC,
    // No #[clvm(rest)] here either
}
