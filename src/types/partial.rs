use std::borrow::Cow;

use chia::{clvm_utils::TreeHash, protocol::Bytes32};
use chia_puzzle_types::CoinProof;
use chia_wallet_sdk::types::Mod;
use clvm_traits::{FromClvm, ToClvm};
use hex_literal::hex;

pub const PARTIAL_PUZZLE: [u8; 420] = hex!(
    "
    ff02ffff01ff04ffff04ff14ffff04ffff0bff0bffff02ff1effff04ff02ffff
    04ffff04ff82013fffff04ffff04ff17ffff04ffff02ffff03ffff15ff82017f
    ff8080ffff0182017fffff01ff088080ff0180ffff04ffff04ff17ff8080ff80
    808080ff808080ff8080808080ff808080ffff04ffff04ff08ffff04ffff30ff
    82013fffff02ff05ffff04ff8202bfff8203ff8080ff8205bf80ff808080ffff
    04ffff02ff16ffff04ff02ffff04ff8202bfffff04ffff11ff8205bfffff05ff
    ff14ffff12ff82017fff819f80ff81df808080ff8080808080ffff03ffff02ff
    ff03ff8202ffffff01ff15ff820affffff0181ff80ff8080ff0180ffff04ffff
    04ff1cff8202ff80ff2f80ff2f80808080ffff04ffff01ffff46ff3f33ff01ff
    ff03ffff15ff0bff8080ffff04ff1cffff04ff05ffff04ff0bffff04ffff04ff
    05ff8080ff8080808080ffff04ff0aff808080ff02ffff03ffff07ff0580ffff
    01ff0bffff0102ffff02ff1effff04ff02ffff04ff09ff80808080ffff02ff1e
    ffff04ff02ffff04ff0dff8080808080ffff01ff0bffff0101ff058080ff0180
    ff018080
    "
);

pub const PARTIAL_PUZZLE_HASH: TreeHash = TreeHash::new(hex!(
    "
    83dc065cf7f261338123f8ec988d092aa577af1ab3d2ab9dd37edde13e709cd9
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
