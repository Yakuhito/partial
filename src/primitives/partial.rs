use chia::{
    bls::Signature,
    protocol::{Bytes32, CoinSpend, SpendBundle},
};
use chia_puzzle_types::{
    CoinProof, LineageProof, Memos,
    cat::CatSolution,
    offer::{NotarizedPayment, Payment, SettlementPaymentsSolution},
};
use chia_puzzles::SETTLEMENT_PAYMENT_HASH;
use chia_wallet_sdk::{
    driver::{Cat, CatInfo, CatSpend, DriverError, Offer, Spend, SpendContext},
    prelude::{Coin, CreateCoin},
    types::{
        MerkleTree, Mod,
        puzzles::{
            P2OneOfManyArgs, P2OneOfManySolution, RevocationArgs, RevocationSolution,
            SettlementPayment,
        },
    },
};
use clvm_traits::{ToClvm, clvm_tuple};
use clvmr::{Allocator, NodePtr};

use crate::{PartialOfferInfo, PartialPriceData, PartialSolution};

#[derive(Debug, Clone)]
pub struct PartialOffer {
    pub coin: Coin,

    pub info: PartialOfferInfo,

    pub spend_bundle: SpendBundle,
}

impl PartialOffer {
    pub fn new(parent_coin_id: Bytes32, amount: u64, info: PartialOfferInfo) -> Self {
        Self {
            coin: Coin::new(parent_coin_id, info.puzzle_hash(), amount),
            info,
            spend_bundle: SpendBundle::new(Vec::new(), Signature::default()),
        }
    }

    pub fn take(self, spend_bundle: SpendBundle) -> SpendBundle {
        SpendBundle::new(
            [self.spend_bundle.coin_spends, spend_bundle.coin_spends].concat(),
            self.spend_bundle.aggregated_signature + &spend_bundle.aggregated_signature,
        )
    }

    pub fn from_spend_bundle(
        ctx: &mut SpendContext,
        spend_bundle: SpendBundle,
    ) -> Result<Self, DriverError> {
        let mut input_spend_bundle =
            SpendBundle::new(Vec::new(), spend_bundle.aggregated_signature);
        let mut special_coin_spend = None;

        for coin_spend in spend_bundle.coin_spends {
            if coin_spend.coin.puzzle_hash == Bytes32::new([1; 32]) {
                if special_coin_spend.is_none() {
                    special_coin_spend = Some(coin_spend);
                } else {
                    return Err(DriverError::ConflictingOfferInputs);
                }
            } else {
                input_spend_bundle.coin_spends.push(coin_spend);
            }
        }

        let Some(special_coin_spend) = special_coin_spend else {
            return Err(DriverError::Custom(
                "No hinting coin spend found in partial offer".to_string(),
            ));
        };

        let hint_ptr = ctx.alloc(&special_coin_spend.puzzle_reveal)?;
        let Some(info) = PartialOfferInfo::from_hint(&ctx.extract(hint_ptr)?) else {
            return Err(DriverError::Custom(
                "Partial offer has ambiguous inner conditions".to_string(),
            ));
        };

        let partial_coin = Coin::new(
            special_coin_spend.coin.parent_coin_info,
            info.puzzle_hash(),
            special_coin_spend.coin.amount,
        );

        Ok(PartialOffer {
            spend_bundle: input_spend_bundle,
            coin: partial_coin,
            info,
        })
    }

    pub fn to_spend_bundle(mut self, ctx: &mut SpendContext) -> Result<SpendBundle, DriverError> {
        let hint = ctx.alloc(&self.info.to_hint())?;

        self.spend_bundle.coin_spends.push(CoinSpend::new(
            Coin::new(
                self.coin.parent_coin_info,
                Bytes32::new([1; 32]),
                self.coin.amount,
            ),
            ctx.serialize(&hint)?,
            ctx.serialize(&NodePtr::NIL)?,
        ));

        Ok(self.spend_bundle)
    }

    pub fn partial_coin_spend<T>(
        &self,
        ctx: &mut SpendContext,
        other_asset_amount: u64,
        create_coin: Option<CreateCoin<T>>,
    ) -> Result<(Spend, NotarizedPayment), DriverError>
    where
        T: ToClvm<Allocator>,
    {
        let args = self.info.to_args(ctx)?;
        let partial_puzzle = ctx.curry(&args)?;

        let partial_ph = self.info.partial_puzzle_hash().into();
        let merkle_tree = MerkleTree::new(&[partial_ph, self.info.maker_puzzle_hash]);
        let inner_puzzle = ctx.curry(P2OneOfManyArgs::new(merkle_tree.root()))?;
        let inner_solution = P2OneOfManySolution {
            merkle_proof: merkle_tree
                .proof(partial_ph)
                .ok_or(DriverError::InvalidMerkleProof)?,
            puzzle: partial_puzzle,
            solution: PartialSolution {
                my_data: CoinProof {
                    parent_coin_info: self.coin.parent_coin_info,
                    inner_puzzle_hash: self.info.inner_puzzle_hash().into(),
                    amount: self.coin.amount,
                },
                other_asset_amount,
                create_coin_rest: create_coin
                    .map(|cc| clvm_tuple!(cc.puzzle_hash, clvm_tuple!(cc.amount, cc.memos))),
                cat_maker_solution: (),
            },
        };
        let inner_solution = ctx.alloc(&inner_solution)?;

        let inner_solution = if self.info.offered_asset_info.hidden_puzzle_hash.is_some() {
            ctx.alloc(&RevocationSolution::new(
                false,
                inner_puzzle,
                inner_solution,
            ))?
        } else {
            inner_solution
        };

        let puzzle =
            PartialOfferInfo::full_puzzle(ctx, self.info.offered_asset_info, inner_puzzle)?;
        let solution = if self.info.offered_asset_info.asset_id.is_some() {
            ctx.alloc(&CatSolution {
                inner_puzzle_solution: inner_solution,
                lineage_proof: self.info.lineage_proof,
                prev_coin_id: self.coin.coin_id(),
                this_coin_info: self.coin,
                next_coin_proof: CoinProof {
                    parent_coin_info: self.coin.parent_coin_info,
                    inner_puzzle_hash: if let Some(hidden_puzzle_hash) =
                        self.info.offered_asset_info.hidden_puzzle_hash
                    {
                        RevocationArgs::new(
                            hidden_puzzle_hash,
                            self.info.inner_puzzle_hash().into(),
                        )
                        .curry_tree_hash()
                        .into()
                    } else {
                        self.info.inner_puzzle_hash().into()
                    },
                    amount: self.coin.amount,
                },
                prev_subtotal: 0,
                extra_delta: 0,
            })?
        } else {
            inner_solution
        };

        Ok((
            Spend::new(puzzle, solution),
            self.notatized_payment(ctx, other_asset_amount)?,
        ))
    }

    pub fn claw_back(&self, ctx: &mut SpendContext, inner_spend: Spend) -> Result<(), DriverError> {
        let partial_ph = self.info.partial_puzzle_hash().into();
        let merkle_tree = MerkleTree::new(&[partial_ph, self.info.maker_puzzle_hash]);
        let inner_puzzle = ctx.curry(P2OneOfManyArgs::new(merkle_tree.root()))?;
        let inner_solution = P2OneOfManySolution {
            merkle_proof: merkle_tree
                .proof(self.info.maker_puzzle_hash)
                .ok_or(DriverError::InvalidMerkleProof)?,
            puzzle: inner_spend.puzzle,
            solution: inner_spend.solution,
        };
        let inner_solution = ctx.alloc(&inner_solution)?;

        let inner_solution = if self.info.offered_asset_info.hidden_puzzle_hash.is_some() {
            ctx.alloc(&RevocationSolution::new(
                false,
                inner_puzzle,
                inner_solution,
            ))?
        } else {
            inner_solution
        };

        let puzzle =
            PartialOfferInfo::full_puzzle(ctx, self.info.offered_asset_info, inner_puzzle)?;
        let solution = if self.info.offered_asset_info.asset_id.is_some() {
            ctx.alloc(&CatSolution {
                inner_puzzle_solution: inner_solution,
                lineage_proof: self.info.lineage_proof,
                prev_coin_id: self.coin.coin_id(),
                this_coin_info: self.coin,
                next_coin_proof: CoinProof {
                    parent_coin_info: self.coin.parent_coin_info,
                    inner_puzzle_hash: if let Some(hidden_puzzle_hash) =
                        self.info.offered_asset_info.hidden_puzzle_hash
                    {
                        RevocationArgs::new(
                            hidden_puzzle_hash,
                            self.info.inner_puzzle_hash().into(),
                        )
                        .curry_tree_hash()
                        .into()
                    } else {
                        self.info.inner_puzzle_hash().into()
                    },
                    amount: self.coin.amount,
                },
                prev_subtotal: 0,
                extra_delta: 0,
            })?
        } else {
            inner_solution
        };

        ctx.spend(self.coin, Spend::new(puzzle, solution))?;
        Ok(())
    }

    pub fn notatized_payment(
        &self,
        ctx: &mut SpendContext,
        amount: u64,
    ) -> Result<NotarizedPayment, DriverError> {
        Ok(NotarizedPayment {
            nonce: self.coin.parent_coin_info,
            payments: vec![Payment::new(
                self.info.maker_puzzle_hash,
                amount,
                ctx.hint(self.info.maker_puzzle_hash)?,
            )],
        })
    }

    pub fn quote(asked_asset_amount: u64, price_data: PartialPriceData) -> u64 {
        asked_asset_amount * price_data.price_precision / price_data.precision
    }

    pub fn reverse_quote(offered_asset_amount: u64, price_data: PartialPriceData) -> u64 {
        offered_asset_amount * price_data.precision / price_data.price_precision
    }

    pub fn accept_offer(
        self,
        ctx: &mut SpendContext,
        offer: Offer,
    ) -> Result<SpendBundle, DriverError> {
        // assumes ask/give amounts were calculated correctly
        let offer_puzzle = ctx.alloc_mod::<SettlementPayment>()?;
        if let Some(requested_asset_id) = self.info.requested_asset_info.asset_id {
            // we're requesting a CAT
            let Some(cats) = offer.offered_coins().cats.get(&requested_asset_id) else {
                return Err(DriverError::IncompatibleAssetInfo);
            };

            let cat = cats[0];
            let (my_spend, notarized_payment) = self.partial_coin_spend(
                ctx,
                cat.coin.amount,
                Some(CreateCoin::<Memos> {
                    puzzle_hash: SETTLEMENT_PAYMENT_HASH.into(),
                    amount: Self::quote(cat.coin.amount, self.info.price_data),
                    memos: Memos::None,
                }),
            )?;
            ctx.spend(self.coin, my_spend)?;

            let inner_spend = Spend::new(
                offer_puzzle,
                ctx.alloc(&SettlementPaymentsSolution {
                    notarized_payments: vec![notarized_payment],
                })?,
            );
            let _ = Cat::spend_all(ctx, &[CatSpend::new(cat, inner_spend)])?;

            if self.info.required_fee.unwrap_or(0) > 0 {
                // offer also gives XCH to pay the required fee
                let Some(given_xch_coin) = offer.offered_coins().xch.first() else {
                    return Err(DriverError::IncompatibleAssetInfo);
                };

                let spend = Spend::new(
                    offer_puzzle,
                    ctx.alloc(&SettlementPaymentsSolution::<NodePtr> {
                        notarized_payments: vec![],
                    })?,
                );
                ctx.spend(*given_xch_coin, spend)?;
            }
        } else {
            // we're requesting XCH
            let Some(given_coin) = offer.offered_coins().xch.first() else {
                return Err(DriverError::IncompatibleAssetInfo);
            };

            let other_asset_amount = given_coin.amount - self.info.required_fee.unwrap_or(0);

            let (my_spend, notarized_payment) = self.partial_coin_spend(
                ctx,
                other_asset_amount,
                Some(CreateCoin::<Memos> {
                    puzzle_hash: SETTLEMENT_PAYMENT_HASH.into(),
                    amount: Self::quote(other_asset_amount, self.info.price_data),
                    memos: Memos::None,
                }),
            )?;
            ctx.spend(self.coin, my_spend)?;

            let spend = Spend::new(
                offer_puzzle,
                ctx.alloc(&SettlementPaymentsSolution {
                    notarized_payments: vec![notarized_payment],
                })?,
            );
            ctx.spend(*given_coin, spend)?;
        };

        if let Some(offered_asset_id) = self.info.offered_asset_info.asset_id {
            // we're offering a CAT
            let Some(notarized_payments) = offer.requested_payments().cats.get(&offered_asset_id)
            else {
                return Err(DriverError::IncompatibleAssetInfo);
            };
            let inner_spend = Spend::new(
                offer_puzzle,
                ctx.alloc(&SettlementPaymentsSolution {
                    notarized_payments: notarized_payments.clone(),
                })?,
            );

            let cat = Cat::new(
                Coin::new(
                    self.coin.coin_id(),
                    PartialOfferInfo::full_asset_puzzle_hash(
                        self.info.offered_asset_info,
                        SETTLEMENT_PAYMENT_HASH.into(),
                    ),
                    notarized_payments[0].payments[0].amount,
                ),
                Some(LineageProof {
                    parent_parent_coin_info: self.coin.parent_coin_info,
                    parent_inner_puzzle_hash: if let Some(hidden_puzzle_hash) =
                        self.info.offered_asset_info.hidden_puzzle_hash
                    {
                        RevocationArgs::new(
                            hidden_puzzle_hash,
                            self.info.inner_puzzle_hash().into(),
                        )
                        .curry_tree_hash()
                        .into()
                    } else {
                        self.info.inner_puzzle_hash().into()
                    },
                    parent_amount: self.coin.amount,
                }),
                CatInfo::new(
                    offered_asset_id,
                    self.info.offered_asset_info.hidden_puzzle_hash,
                    SETTLEMENT_PAYMENT_HASH.into(),
                ),
            );

            let _ = Cat::spend_all(ctx, &[CatSpend::new(cat, inner_spend)])?;
        } else {
            // we're offering XCH
            let coin_amount: u64 = offer
                .requested_payments()
                .xch
                .iter()
                .map(|p| p.payments.iter().map(|p| p.amount).sum::<u64>())
                .sum();
            let spend = Spend::new(
                offer_puzzle,
                ctx.alloc(&SettlementPaymentsSolution {
                    notarized_payments: offer.requested_payments().xch.clone(),
                })?,
            );
            ctx.spend(
                Coin::new(
                    self.coin.coin_id(),
                    SETTLEMENT_PAYMENT_HASH.into(),
                    coin_amount,
                ),
                spend,
            )?;
        }

        let spend_bundle = SpendBundle::new(ctx.take(), Signature::default());
        Ok(self.take(offer.take(spend_bundle)))
    }

    pub fn child(&self, child_amount: u64) -> Self {
        Self {
            coin: Coin::new(self.coin.coin_id(), self.coin.puzzle_hash, child_amount),
            info: self
                .info
                .clone()
                .with_lineage_proof(self.info.lineage_proof.map(|_| LineageProof {
                    parent_parent_coin_info: self.coin.parent_coin_info,
                    parent_inner_puzzle_hash: if let Some(hidden_puzzle_hash) =
                        self.info.offered_asset_info.hidden_puzzle_hash
                    {
                        RevocationArgs::new(
                            hidden_puzzle_hash,
                            self.info.inner_puzzle_hash().into(),
                        )
                        .curry_tree_hash()
                        .into()
                    } else {
                        self.info.inner_puzzle_hash().into()
                    },
                    parent_amount: self.coin.amount,
                })),
            spend_bundle: SpendBundle::new(Vec::new(), Signature::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use chia_wallet_sdk::{
        driver::{
            AssetInfo, CatAssetInfo, OfferCoins, RequestedPayments, SpendWithConditions,
            StandardLayer,
        },
        test::{Benchmark, Simulator},
        types::{Conditions, announcement_id},
    };
    use clvm_traits::clvm_quote;
    use rstest::*;

    use crate::PartialOfferAssetInfo;

    use super::*;

    pub fn ensure_conditions_met(
        ctx: &mut SpendContext,
        sim: &mut Simulator,
        conditions: Conditions<NodePtr>,
        amount_to_mint: u64,
    ) -> Result<(), DriverError> {
        let checker_puzzle_ptr = clvm_quote!(conditions).to_clvm(ctx)?;
        let checker_coin = sim.new_coin(ctx.tree_hash(checker_puzzle_ptr).into(), amount_to_mint);
        ctx.spend(checker_coin, Spend::new(checker_puzzle_ptr, NodePtr::NIL))?;

        Ok(())
    }

    #[rstest]
    #[case("XCH for CAT", false, false, true, false)]
    #[case("XCH for rCAT", false, false, true, true)]
    #[case("CAT for XCH", true, false, false, false)]
    #[case("CAT for CAT", true, false, true, false)]
    #[case("CAT for rCAT", true, false, true, true)]
    #[case("rCAT for XCH", true, true, false, false)]
    #[case("rCAT for CAT", true, true, true, false)]
    #[case("rCAT for rCAT", true, true, true, true)]
    fn test_partial_offers(
        #[case] comment: &str,
        #[case] offered_is_cat: bool,
        #[case] offered_is_revocable: bool,
        #[case] asked_is_cat: bool,
        #[case] asked_is_revocable: bool,
    ) -> anyhow::Result<()> {
        let ctx = &mut SpendContext::new();
        let mut sim = Simulator::new();
        let mut benchmark = Benchmark::new(format!("Partial Offer ({})", comment));

        let offered_amount = 100_000;
        let asked_amount = 20_000;
        let price_data = PartialPriceData {
            price_precision: offered_amount,
            precision: asked_amount,
        };

        for expiration in [None, Some(100)] {
            for required_fee in [None, Some(4200000)] {
                let taker_bls = sim.bls(asked_amount);
                let maker_bls = sim.bls(offered_amount);

                let inner_conds = Conditions::new()
                    .create_coin(
                        SETTLEMENT_PAYMENT_HASH.into(),
                        asked_amount / 4,
                        Memos::None,
                    )
                    .create_coin(
                        SETTLEMENT_PAYMENT_HASH.into(),
                        asked_amount * 3 / 4,
                        Memos::None,
                    );
                #[allow(clippy::type_complexity)]
                let (create_conds, taker_cats, taker_fee_coins, taker_xch_coins): (
                    Conditions,
                    Option<Vec<Cat>>,
                    Option<Vec<Coin>>,
                    Option<Vec<Coin>>,
                ) = if asked_is_cat {
                    let taker_fee_coins = if let Some(required_fee) = required_fee {
                        let first_coin = sim.new_coin(SETTLEMENT_PAYMENT_HASH.into(), required_fee);
                        let second_coin =
                            sim.new_coin(SETTLEMENT_PAYMENT_HASH.into(), required_fee);
                        Some(vec![first_coin, second_coin])
                    } else {
                        None
                    };

                    if asked_is_revocable {
                        let (create_conds, cats) = Cat::issue_revocable_with_coin(
                            ctx,
                            taker_bls.coin.coin_id(),
                            Bytes32::default(), // hidden puzzle hash
                            asked_amount,
                            inner_conds,
                        )?;
                        (create_conds, Some(cats), taker_fee_coins, None)
                    } else {
                        let (create_conds, cats) = Cat::issue_with_coin(
                            ctx,
                            taker_bls.coin.coin_id(),
                            asked_amount,
                            inner_conds,
                        )?;
                        (create_conds, Some(cats), taker_fee_coins, None)
                    }
                } else {
                    (
                        inner_conds,
                        None,
                        None,
                        Some(vec![
                            Coin::new(
                                taker_bls.coin.coin_id(),
                                SETTLEMENT_PAYMENT_HASH.into(),
                                asked_amount / 4 + required_fee.unwrap_or(0),
                            ),
                            Coin::new(
                                taker_bls.coin.coin_id(),
                                SETTLEMENT_PAYMENT_HASH.into(),
                                asked_amount * 3 / 4 + required_fee.unwrap_or(0),
                            ),
                        ]),
                    )
                };
                StandardLayer::new(taker_bls.pk).spend(ctx, taker_bls.coin, create_conds)?;

                let (offered_asset_info, source_cat, source_xch_coin) = if offered_is_cat {
                    let inner_conds = Conditions::new().create_coin(
                        maker_bls.puzzle_hash,
                        maker_bls.coin.amount,
                        Memos::None,
                    );

                    let (create_conds, cats) = if offered_is_revocable {
                        Cat::issue_revocable_with_coin(
                            ctx,
                            maker_bls.coin.coin_id(),
                            Bytes32::default(), // hidden puzzle hash
                            maker_bls.coin.amount,
                            inner_conds,
                        )?
                    } else {
                        Cat::issue_with_coin(
                            ctx,
                            maker_bls.coin.coin_id(),
                            maker_bls.coin.amount,
                            inner_conds,
                        )?
                    };
                    StandardLayer::new(maker_bls.pk).spend(ctx, maker_bls.coin, create_conds)?;

                    (
                        PartialOfferAssetInfo::cat(
                            cats[0].info.asset_id,
                            cats[0].info.hidden_puzzle_hash,
                        ),
                        Some(cats[0]),
                        None,
                    )
                } else {
                    (PartialOfferAssetInfo::xch(), None, Some(maker_bls.coin))
                };

                let requested_asset_info = if let Some(ref taker_cats) = taker_cats {
                    PartialOfferAssetInfo::cat(
                        taker_cats[0].info.asset_id,
                        taker_cats[0].info.hidden_puzzle_hash,
                    )
                } else {
                    PartialOfferAssetInfo::xch()
                };

                let partial_offer_info = PartialOfferInfo::new(
                    source_cat.map(|cat| cat.child_lineage_proof()),
                    offered_asset_info,
                    requested_asset_info,
                    maker_bls.puzzle_hash,
                    expiration,
                    required_fee,
                    price_data,
                );
                let partial_creation_conds = Conditions::new().create_coin(
                    partial_offer_info.inner_puzzle_hash().into(),
                    offered_amount,
                    Memos::None,
                );

                let partial_offer_parent_id = if let Some(source_xch_coin) = source_xch_coin {
                    StandardLayer::new(maker_bls.pk).spend(
                        ctx,
                        source_xch_coin,
                        partial_creation_conds,
                    )?;

                    source_xch_coin.coin_id()
                } else {
                    let source_cat = source_cat.unwrap();

                    let inner_spend = StandardLayer::new(maker_bls.pk)
                        .spend_with_conditions(ctx, partial_creation_conds)?;

                    let _ = Cat::spend_all(ctx, &[CatSpend::new(source_cat, inner_spend)])?;

                    source_cat.coin.coin_id()
                };

                let mut partial_offer =
                    PartialOffer::new(partial_offer_parent_id, offered_amount, partial_offer_info);
                sim.spend_coins(ctx.take(), &[taker_bls.sk.clone(), maker_bls.sk.clone()])?;

                // Accept partial offer
                for partial_fill_only in [true, false] {
                    let given_amount = if partial_fill_only {
                        asked_amount / 4
                    } else {
                        // fill_no = 1 -> we're filling the rest of the offer
                        asked_amount * 3 / 4
                    };
                    let expected_amount = PartialOffer::quote(given_amount, price_data);
                    if partial_fill_only {
                        assert_eq!(expected_amount, offered_amount / 4);
                    } else {
                        assert_eq!(expected_amount, offered_amount * 3 / 4);
                    }

                    let fill_no = if partial_fill_only { 0 } else { 1 };

                    let mut asset_info = AssetInfo::new();
                    if let Some(ref taker_cats) = taker_cats {
                        asset_info.insert_cat(
                            taker_cats[fill_no].info.asset_id,
                            CatAssetInfo::new(taker_cats[fill_no].info.hidden_puzzle_hash),
                        )?;
                    }
                    if let Some(source_cat) = source_cat {
                        asset_info.insert_cat(
                            source_cat.info.asset_id,
                            CatAssetInfo::new(source_cat.info.hidden_puzzle_hash),
                        )?;
                    }

                    let mut offered_coins = OfferCoins::new();
                    if let Some(ref taker_cats) = taker_cats {
                        offered_coins
                            .cats
                            .insert(taker_cats[fill_no].info.asset_id, vec![taker_cats[fill_no]]);
                    } else if let Some(ref taker_xch_coins) = taker_xch_coins {
                        offered_coins.xch.push(taker_xch_coins[fill_no]);
                    }
                    if let Some(ref taker_fee_coins) = taker_fee_coins {
                        offered_coins.xch.push(taker_fee_coins[fill_no]);
                    }

                    let notarized_payment =
                        partial_offer.notatized_payment(ctx, expected_amount)?;
                    let notarized_payment_ptr = ctx.alloc(&notarized_payment)?;

                    let mut requested_payments = RequestedPayments::new();
                    if let Some(offered_asset_id) = offered_asset_info.asset_id {
                        requested_payments
                            .cats
                            .insert(offered_asset_id, vec![notarized_payment]);
                    } else {
                        requested_payments.xch.push(notarized_payment);
                    };

                    ensure_conditions_met(
                        ctx,
                        &mut sim,
                        Conditions::new().assert_puzzle_announcement(announcement_id(
                            PartialOfferInfo::full_asset_puzzle_hash(
                                offered_asset_info,
                                SETTLEMENT_PAYMENT_HASH.into(),
                            ),
                            ctx.tree_hash(notarized_payment_ptr).to_vec(),
                        )),
                        0,
                    )?;

                    let offer = Offer::new(
                        SpendBundle::new(vec![], Signature::default()),
                        offered_coins,
                        requested_payments,
                        asset_info,
                    );

                    let new_partial_offer = partial_offer.child(offered_amount - expected_amount);
                    let spend_bundle = partial_offer.accept_offer(ctx, offer)?;
                    benchmark.add_spends(
                        ctx,
                        &mut sim,
                        spend_bundle.coin_spends,
                        if partial_fill_only {
                            "partial_fill"
                        } else {
                            "full_fill"
                        },
                        &[taker_bls.sk.clone()],
                    )?;

                    if partial_fill_only {
                        assert!(sim.coin_state(new_partial_offer.coin.coin_id()).is_some());
                    }

                    partial_offer = new_partial_offer;
                }
            }
        }

        println!(" ");
        benchmark.print_summary(Some(&format!(
            "partial-offer-{}.costs",
            comment.replace(" ", "-")
        )));

        Ok(())
    }
}
