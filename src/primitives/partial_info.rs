use chia::{
    clvm_utils::{ToTreeHash, TreeHash},
    protocol::Bytes32,
};
use chia_puzzle_types::{LineageProof, cat::CatArgs};
use chia_puzzles::SETTLEMENT_PAYMENT_HASH;
use chia_wallet_sdk::{
    driver::{CatMaker, DriverError, SpendContext},
    types::{
        Condition, Conditions, MerkleTree, Mod,
        puzzles::{P2OneOfManyArgs, RevocationArgs},
    },
};
use clvm_traits::clvm_list;
use clvmr::NodePtr;

use crate::{PartialOfferAssetInfo, PartialOfferHint, PartialPriceData, PartialPuzzleArgs};

#[derive(Debug, Clone)]
pub struct PartialOfferInfo {
    pub lineage_proof: Option<LineageProof>,
    pub offered_asset_info: PartialOfferAssetInfo,
    pub requested_asset_info: PartialOfferAssetInfo,
    pub maker_puzzle_hash: Bytes32,
    pub expiration: Option<u64>,
    pub required_fee: Option<u64>,
    pub price_data: PartialPriceData,
}

impl PartialOfferInfo {
    pub fn new(
        lineage_proof: Option<LineageProof>,
        offered_asset_info: PartialOfferAssetInfo,
        requested_asset_info: PartialOfferAssetInfo,
        maker_puzzle_hash: Bytes32,
        expiration: Option<u64>,
        required_fee: Option<u64>,
        price_data: PartialPriceData,
    ) -> Self {
        Self {
            lineage_proof,
            offered_asset_info,
            requested_asset_info,
            maker_puzzle_hash,
            expiration,
            required_fee,
            price_data,
        }
    }

    pub fn with_lineage_proof(self, lineage_proof: Option<LineageProof>) -> Self {
        Self {
            lineage_proof,
            ..self
        }
    }

    pub fn to_cat_maker(asset_info: PartialOfferAssetInfo) -> CatMaker {
        if let Some(asset_id) = asset_info.asset_id {
            if let Some(hidden_puzzle_hash) = asset_info.hidden_puzzle_hash {
                CatMaker::Revocable {
                    tail_hash_hash: asset_id.tree_hash(),
                    hidden_puzzle_hash_hash: hidden_puzzle_hash.tree_hash(),
                }
            } else {
                CatMaker::Default {
                    tail_hash_hash: asset_id.tree_hash(),
                }
            }
        } else {
            CatMaker::Xch
        }
    }

    pub fn full_asset_puzzle_hash(
        asset_info: PartialOfferAssetInfo,
        inner_puzzle_hash: Bytes32,
    ) -> Bytes32 {
        if let Some(asset_id) = asset_info.asset_id {
            if let Some(hidden_puzzle_hash) = asset_info.hidden_puzzle_hash {
                CatArgs::curry_tree_hash(
                    asset_id,
                    RevocationArgs::new(hidden_puzzle_hash, inner_puzzle_hash).curry_tree_hash(),
                )
                .into()
            } else {
                CatArgs::curry_tree_hash(asset_id, inner_puzzle_hash.into()).into()
            }
        } else {
            inner_puzzle_hash
        }
    }

    pub fn full_puzzle(
        ctx: &mut SpendContext,
        asset_info: PartialOfferAssetInfo,
        inner_puzzle: NodePtr,
    ) -> Result<NodePtr, DriverError> {
        if let Some(asset_id) = asset_info.asset_id {
            if let Some(hidden_puzzle_hash) = asset_info.hidden_puzzle_hash {
                let inner_puzzle_hash = ctx.tree_hash(inner_puzzle);
                let inner_puzzle_w_revocation = ctx.curry(RevocationArgs::new(
                    hidden_puzzle_hash,
                    inner_puzzle_hash.into(),
                ))?;

                ctx.curry(CatArgs::new(asset_id, inner_puzzle_w_revocation))
            } else {
                ctx.curry(CatArgs::new(asset_id, inner_puzzle))
            }
        } else {
            Ok(inner_puzzle)
        }
    }

    pub fn to_args(
        &self,
        ctx: &mut SpendContext,
    ) -> Result<PartialPuzzleArgs<NodePtr, Conditions>, DriverError> {
        let offered_cat_maker = Self::to_cat_maker(self.offered_asset_info);

        let other_asset_offer_mod =
            Self::full_asset_puzzle_hash(self.requested_asset_info, SETTLEMENT_PAYMENT_HASH.into());

        Ok(PartialPuzzleArgs {
            cat_maker: offered_cat_maker.get_puzzle(ctx)?,
            other_asset_offer_mod,
            receiver_puzzle_hash: self.maker_puzzle_hash,
            inner_conditions: self.inner_conditions(),
            price_data: self.price_data,
        })
    }

    pub fn inner_conditions(&self) -> Conditions {
        let mut inner_conditions = Conditions::new();
        if let Some(expiration) = self.expiration {
            inner_conditions = inner_conditions.assert_before_seconds_absolute(expiration);
        }
        if let Some(required_fee) = self.required_fee {
            inner_conditions = inner_conditions.reserve_fee(required_fee);
        }

        inner_conditions
    }

    pub fn partial_puzzle_hash(&self) -> TreeHash {
        PartialPuzzleArgs {
            cat_maker: Self::to_cat_maker(self.offered_asset_info).curry_tree_hash(),
            other_asset_offer_mod: Self::full_asset_puzzle_hash(
                self.requested_asset_info,
                SETTLEMENT_PAYMENT_HASH.into(),
            ),
            receiver_puzzle_hash: self.maker_puzzle_hash,
            // forbidden hack
            inner_conditions: match (self.expiration, self.required_fee) {
                (Some(expiration), Some(required_fee)) => {
                    clvm_list!(clvm_list!(85, expiration), clvm_list!(52, required_fee)).tree_hash()
                }
                (Some(expiration), None) => clvm_list!(clvm_list!(85, expiration)).tree_hash(),
                (None, Some(required_fee)) => clvm_list!(clvm_list!(52, required_fee)).tree_hash(),
                (None, None) => ().tree_hash(),
            },
            price_data: self.price_data,
        }
        .curry_tree_hash()
    }

    pub fn inner_puzzle_hash(&self) -> TreeHash {
        P2OneOfManyArgs::new(
            MerkleTree::new(&[self.partial_puzzle_hash().into(), self.maker_puzzle_hash]).root(),
        )
        .curry_tree_hash()
    }

    pub fn puzzle_hash(&self) -> Bytes32 {
        Self::full_asset_puzzle_hash(self.offered_asset_info, self.inner_puzzle_hash().into())
    }

    pub fn to_hint(&self) -> PartialOfferHint<Conditions> {
        PartialOfferHint {
            lineage_proof: self.lineage_proof,
            offered_asset_info: self.offered_asset_info,
            requested_asset_info: self.requested_asset_info,
            maker_puzzle_hash: self.maker_puzzle_hash,
            inner_conditions: self.inner_conditions(),
            price_data: self.price_data,
        }
    }

    pub fn from_hint(hint: &PartialOfferHint<Conditions>) -> Option<Self> {
        let expiration = hint
            .inner_conditions
            .iter()
            .find_map(Condition::as_assert_before_seconds_absolute)
            .map(|cond| cond.seconds);
        let required_fee = hint
            .inner_conditions
            .iter()
            .find_map(Condition::as_reserve_fee)
            .map(|cond| cond.amount);

        Some(Self {
            lineage_proof: hint.lineage_proof,
            offered_asset_info: hint.offered_asset_info,
            requested_asset_info: hint.requested_asset_info,
            maker_puzzle_hash: hint.maker_puzzle_hash,
            expiration,
            required_fee,
            price_data: hint.price_data,
        })
    }
}
