use std::borrow::Cow;

use chia::{clvm_utils::TreeHash, protocol::Bytes32};
use chia_puzzle_types::CoinProof;
use chia_wallet_sdk::types::Mod;
use clvm_traits::{FromClvm, ToClvm};
use hex_literal::hex;

pub const PARTIAL_PUZZLE: [u8; 395] = hex!(
    "
    ff02ffff01ff04ffff04ff14ffff04ffff0bff0bffff02ff1effff04ff02ffff
    04ffff04ff82027fffff04ffff04ff17ffff04ffff02ffff03ffff15ff2fff82
    02ff80ffff01ff0880ffff018202ff80ff0180ffff04ffff04ff17ff8080ff80
    808080ff808080ff8080808080ff808080ffff04ffff04ff08ffff04ffff30ff
    82027fffff02ff05ffff04ff82057fff8207ff8080ff820b7f80ff808080ffff
    04ffff02ff16ffff04ff02ffff04ff82057fffff04ffff11ff820b7fffff05ff
    ff14ffff12ff8202ffff82013f80ff8201bf808080ff8080808080ffff03ff82
    05ffffff04ffff04ff1cff8205ff80ff5f80ff5f80808080ffff04ffff01ffff
    46ff3f33ff01ffff03ffff15ff0bff8080ffff04ff1cffff04ff05ffff04ff0b
    ffff04ffff04ff05ff8080ff8080808080ffff04ff0aff808080ff02ffff03ff
    ff07ff0580ffff01ff0bffff0102ffff02ff1effff04ff02ffff04ff09ff8080
    8080ffff02ff1effff04ff02ffff04ff0dff8080808080ffff01ff0bffff0101
    ff058080ff0180ff018080
    "
);

pub const PARTIAL_PUZZLE_HASH: TreeHash = TreeHash::new(hex!(
    "
    3f99548f5c089cfb1f2b8f8b04a63787b46a0700e74fc926528807fc996d8432
    "
));

#[derive(ToClvm, FromClvm, Debug, Clone, Copy, PartialEq, Eq)]
#[clvm(list)]
pub struct PartialPriceData {
    pub price_precision: u64,
    #[clvm(rest)]
    pub precision: u64,
}

#[derive(ToClvm, FromClvm, Debug, Clone, Copy, PartialEq, Eq)]
#[clvm(curry)]
pub struct PartialPuzzleArgs<CM, IC> {
    pub cat_maker: CM,
    pub other_asset_offer_mod: Bytes32,
    pub receiver_puzzle_hash: Bytes32,
    pub minimum_other_asset_amount: u64,
    pub inner_conditions: IC,
    pub price_data: PartialPriceData,
}

#[derive(FromClvm, ToClvm, Debug, Clone, PartialEq, Eq)]
#[clvm(list)]
pub struct PartialSolution<CCR, CMS> {
    pub my_data: CoinProof,
    pub other_asset_amount: u64,
    pub create_coin_rest: Option<CCR>,
    #[clvm(rest)]
    pub cat_maker_solution: CMS,
}

impl<CM, IC> Mod for PartialPuzzleArgs<CM, IC> {
    fn mod_reveal() -> Cow<'static, [u8]> {
        Cow::Borrowed(&PARTIAL_PUZZLE)
    }

    fn mod_hash() -> TreeHash {
        PARTIAL_PUZZLE_HASH
    }
}
