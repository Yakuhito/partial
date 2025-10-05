use std::borrow::Cow;

use chia::{clvm_utils::TreeHash, protocol::Bytes32};
use chia_puzzle_types::CoinProof;
use chia_wallet_sdk::types::Mod;
use clvm_traits::{FromClvm, ToClvm};
use hex_literal::hex;

pub const PARTIAL_PUZZLE: [u8; 429] = hex!(
    "
    ff02ffff01ff02ffff01ff04ffff04ffff013fffff04ffff0bff17ffff02ff05
    ffff04ff05ffff04ff8204ffffff04ffff04ff2fffff04ffff02ffff03ffff15
    ff8205ffff81bf80ffff018205ffffff01ff088080ff0180ffff04ffff04ff2f
    ff8080ff80808080ff808080808080ff808080ffff04ffff04ffff0146ffff04
    ffff30ff8204ffffff02ff0bffff04ff820affff8217ff8080ff8216ff80ff80
    8080ffff04ffff03ffff15ff04ff8080ffff04ffff0133ffff04ff820affffff
    04ff04ffff04ffff04ff820affff8080ff8080808080ffff04ffff0101ff8080
    80ffff03ffff02ffff03ff820bffffff01ff02ffff03ffff15ff822bffffff01
    81ff80ffff01ff0101ffff018080ff0180ffff018080ff0180ffff04ffff04ff
    ff0133ff820bff80ff0680ff0680808080ffff04ffff04ffff11ff820b7fffff
    13ffff12ff8202ffff82013f80ff8201bf8080ffff02ff2fff820fff8080ff01
    8080ffff04ffff01ff02ffff03ffff07ff0380ffff01ff0bffff0102ffff02ff
    02ffff04ff02ff058080ffff02ff02ffff04ff02ff07808080ffff01ff0bffff
    0101ff038080ff0180ff018080
    "
);

pub const PARTIAL_PUZZLE_HASH: TreeHash = TreeHash::new(hex!(
    "
    e8b38a9da89c523f3a1e4b2679d893e1f565d800ecd4e37b25ef644ae84fe868
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
pub struct PartialPuzzleArgs<CM, IP> {
    pub cat_maker: CM,
    pub other_asset_offer_mod: Bytes32,
    pub receiver_puzzle_hash: Bytes32,
    pub inner_puzzle: IP,
    pub min_other_asset_amount_minus_one: u64,
    pub price_data: PartialPriceData,
}

#[derive(FromClvm, ToClvm, Debug, Clone, PartialEq, Eq)]
#[clvm(list)]
pub struct PartialSolution<CCR, CMS, IPS> {
    pub my_data: CoinProof,
    pub other_asset_amount: u64,
    pub create_coin_rest: Option<CCR>,
    pub cat_maker_solution: CMS,
    #[clvm(rest)]
    pub inner_puzzle_solution: IPS,
}

impl<CM, IC> Mod for PartialPuzzleArgs<CM, IC> {
    fn mod_reveal() -> Cow<'static, [u8]> {
        Cow::Borrowed(&PARTIAL_PUZZLE)
    }

    fn mod_hash() -> TreeHash {
        PARTIAL_PUZZLE_HASH
    }
}
