use std::borrow::Cow;

use chia::{clvm_utils::TreeHash, protocol::Bytes32};
use chia_puzzle_types::CoinProof;
use chia_wallet_sdk::types::Mod;
use clvm_traits::{FromClvm, ToClvm};
use hex_literal::hex;

pub const PARTIAL_PUZZLE: [u8; 446] = hex!(
    "
    ff02ffff01ff04ffff04ffff013fffff04ffff0bff0bffff02ff06ffff04ff02
    ffff04ffff04ff82027fffff04ffff04ff17ffff04ffff02ffff03ffff15ff82
    02ffff5f80ffff018202ffffff01ff088080ff0180ffff04ffff04ff17ff8080
    ff80808080ff808080ff8080808080ff808080ffff04ffff04ffff0146ffff04
    ffff30ff82027fffff02ff05ffff04ff82057fff820bff8080ff820b7f80ff80
    8080ffff04ffff02ff04ffff04ff02ffff04ff82057fffff04ffff11ff820b7f
    ffff05ffff14ffff12ff8202ffff82013f80ff8201bf808080ff8080808080ff
    ff02ffff03ffff02ffff03ff8205ffffff01ff15ff8215ffffff0181ff80ff80
    80ff0180ffff01ff04ffff04ffff0133ff8205ff80ffff02ff2fff820fff8080
    ffff01ff02ff2fff820fff8080ff0180808080ffff04ffff01ffff03ffff15ff
    0bff8080ffff04ffff0133ffff04ff05ffff04ff0bffff04ffff04ff05ff8080
    ff8080808080ffff01ff018080ff02ffff03ffff07ff0580ffff01ff0bffff01
    02ffff02ff06ffff04ff02ffff04ff09ff80808080ffff02ff06ffff04ff02ff
    ff04ff0dff8080808080ffff01ff0bffff0101ff058080ff0180ff018080
    "
);

pub const PARTIAL_PUZZLE_HASH: TreeHash = TreeHash::new(hex!(
    "
    91fe76d50effb3b2e7079770f3c309d7c0aa8022b5034457121f61ba38df4b12
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
