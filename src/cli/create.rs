use chia::protocol::{Bytes32, SpendBundle};
use chia_puzzle_types::{
    Memos,
    offer::{NotarizedPayment, Payment, SettlementPaymentsSolution},
    standard::StandardArgs,
};
use chia_wallet_sdk::{
    driver::{
        Cat, CatSpend, Offer, Spend, SpendContext, create_security_coin, decode_offer,
        spend_security_coin,
    },
    types::{Conditions, puzzles::SettlementPayment},
    utils::Address,
};
use clvmr::NodePtr;
use slot_machine::{
    CliError, SageClient, assets_xch_and_cat, assets_xch_only, get_constants,
    hex_string_to_bytes32, hex_string_to_pubkey, no_assets, parse_amount,
};

use crate::{
    PartialOffer, PartialOfferAssetInfo, PartialOfferInfo, PartialPriceData, encode_partial_offer,
};

pub async fn cli_create(
    offered_asset_id_str: Option<String>,
    offered_amount_str: String,
    asked_asset_id_str: Option<String>,
    asked_amount_str: String,
    expiration: Option<u64>,
    fee_str: String,
    testnet11: bool,
) -> Result<(), CliError> {
    let offered_asset_id = if let Some(offered_asset_id_str) = &offered_asset_id_str {
        Some(hex_string_to_bytes32(offered_asset_id_str)?)
    } else {
        None
    };

    let asked_asset_id = if let Some(asked_asset_id_str) = &asked_asset_id_str {
        Some(hex_string_to_bytes32(asked_asset_id_str)?)
    } else {
        None
    };

    if offered_asset_id == asked_asset_id {
        return Err(CliError::Custom(
            "Do you actually want to ask and offer the same asset?".to_string(),
        ));
    }

    let offered_amount = parse_amount(&offered_amount_str, offered_asset_id.is_some())?;
    let asked_amount = parse_amount(&asked_amount_str, asked_asset_id.is_some())?;
    let fee = parse_amount(&fee_str, false)?;

    let sage = SageClient::new()?;

    let one_sided_offer = sage
        .make_offer(
            no_assets(),
            if let Some(offered_asset_id_str) = offered_asset_id_str {
                assets_xch_and_cat(1, offered_asset_id_str, offered_amount)
            } else {
                assets_xch_only(offered_amount + 1)
            },
            fee,
            None,
            None,
            true,
        )
        .await?;
    println!("One-sided offer {} created.", one_sided_offer.offer_id);

    let data = &sage.get_derivations(false, 0, 1).await?.derivations[0];
    println!(
        "Will use the following address for clawback: {}",
        data.address
    );

    let maker_puzzle_hash = Address::decode(&data.address)?.puzzle_hash;
    let maker_pk = hex_string_to_pubkey(&data.public_key)?;

    if StandardArgs::curry_tree_hash(maker_pk) != maker_puzzle_hash.into() {
        return Err(CliError::Custom(
            "Maker uses non-standard puzzle".to_string(),
        ));
    }

    let mut ctx = SpendContext::new();

    let offer = Offer::from_spend_bundle(&mut ctx, &decode_offer(&one_sided_offer.offer)?)?;

    let price_data = PartialPriceData {
        price_precision: offered_amount,
        precision: asked_amount,
    };

    let requested_asset_info = if let Some(asked_asset_id) = asked_asset_id {
        PartialOfferAssetInfo::cat(asked_asset_id, None)
    } else {
        PartialOfferAssetInfo::xch()
    };

    let offered_asset_info = if let Some(offered_asset_id) = offered_asset_id {
        PartialOfferAssetInfo::cat(offered_asset_id, None)
    } else {
        PartialOfferAssetInfo::xch()
    };

    let lineage_proof = offered_asset_id.map(|offered_asset_id| {
        offer.offered_coins().cats.get(&offered_asset_id).unwrap()[0].child_lineage_proof()
    });

    let partial_offer_info = PartialOfferInfo::new(
        lineage_proof,
        offered_asset_info,
        requested_asset_info,
        1,
        maker_puzzle_hash,
        expiration,
        price_data,
    );

    let (security_sk, security_coin) =
        create_security_coin(&mut ctx, offer.offered_coins().xch[0])?;
    let _ = ctx.take(); // ignore coin spend inserted by func above

    let offer_mod = ctx.alloc_mod::<SettlementPayment>()?;
    let partial_offer_payment = Payment::new(
        partial_offer_info.inner_puzzle_hash().into(),
        offered_amount,
        Memos::None,
    );
    let security_coin_payment =
        Payment::new(security_coin.puzzle_hash, security_coin.amount, Memos::None);
    let xch_offer_solution = ctx.alloc(&SettlementPaymentsSolution {
        notarized_payments: vec![NotarizedPayment::new(
            Bytes32::default(),
            if offered_asset_id.is_none() {
                vec![partial_offer_payment.clone(), security_coin_payment]
            } else {
                vec![security_coin_payment]
            },
        )],
    })?;
    ctx.spend(
        offer.offered_coins().xch[0],
        Spend::new(offer_mod, xch_offer_solution),
    )?;

    if let Some(offered_asset_id) = offered_asset_id {
        let cat = offer.offered_coins().cats.get(&offered_asset_id).unwrap()[0];
        let cat_offer_solution = ctx.alloc(&SettlementPaymentsSolution {
            notarized_payments: vec![NotarizedPayment::new(
                Bytes32::default(),
                vec![partial_offer_payment],
            )],
        })?;
        let _ = Cat::spend_all(
            &mut ctx,
            &[CatSpend::new(
                cat,
                Spend::new(offer_mod, cat_offer_solution),
            )],
        )?;
    }

    let security_sig = spend_security_coin(
        &mut ctx,
        security_coin,
        Conditions::new().remark(NodePtr::NIL),
        &security_sk,
        get_constants(testnet11),
    )?;

    let parent_coin_id = if let Some(offered_asset_id) = offered_asset_id {
        offer.offered_coins().cats.get(&offered_asset_id).unwrap()[0]
            .coin
            .coin_id()
    } else {
        offer.offered_coins().xch[0].coin_id()
    };
    let partial_offer = PartialOffer::new(parent_coin_id, offered_amount, partial_offer_info);

    let mut coin_spends = partial_offer.to_spend_bundle(&mut ctx)?.coin_spends;
    coin_spends.extend(ctx.take());

    let sig = security_sig + &offer.spend_bundle().aggregated_signature;
    coin_spends.extend(offer.spend_bundle().coin_spends.to_vec());
    let sb = SpendBundle::new(coin_spends, sig);

    println!("Partial offer: {:}", encode_partial_offer(&sb)?);

    Ok(())
}
