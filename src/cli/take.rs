use chia_wallet_sdk::{
    coinset::ChiaRpcClient,
    driver::{Offer, SpendContext, decode_offer},
};
use slot_machine::{
    CliError, SageClient, assets_xch_only, get_coinset_client, parse_amount, wait_for_coin,
};

use crate::{PartialOffer, assets_cat_only, decode_partial_offer, encode_partial_offer};

pub async fn cli_take(
    offer: String,
    take_amount_str: String,
    fee_str: String,
    testnet11: bool,
) -> Result<(), CliError> {
    let fee = parse_amount(&fee_str, false)?;
    let mut ctx = SpendContext::new();

    let partial_offer = PartialOffer::from_spend_bundle(&mut ctx, decode_partial_offer(&offer)?)?;

    let mut take_amount = parse_amount(
        &take_amount_str,
        partial_offer.info.requested_asset_info.asset_id.is_some(),
    )?;
    let output_amount = PartialOffer::quote(take_amount, partial_offer.info.price_data);
    let min_take_amount = PartialOffer::reverse_quote(output_amount, partial_offer.info.price_data);

    if take_amount > min_take_amount {
        println!("Saving {} mojos :)", take_amount - min_take_amount);
        take_amount = min_take_amount;
    }

    if partial_offer.coin.amount > output_amount {
        println!(
            "New partial offer will be: {}",
            encode_partial_offer(
                &partial_offer
                    .child(partial_offer.coin.amount - output_amount)
                    .to_spend_bundle(&mut ctx)?
            )?
        );
    }

    let sage = SageClient::new()?;
    let offer_resp = sage
        .make_offer(
            if let Some(offered_asset_id) = partial_offer.info.offered_asset_info.asset_id {
                assets_cat_only(hex::encode(offered_asset_id), output_amount)
            } else {
                assets_xch_only(output_amount)
            },
            if let Some(requested_asset_id) = partial_offer.info.requested_asset_info.asset_id {
                assets_cat_only(hex::encode(requested_asset_id), take_amount)
            } else {
                assets_xch_only(take_amount)
            },
            fee,
            None,
            None,
            true,
        )
        .await?;

    println!("Offer {} created.", offer_resp.offer_id);

    let partial_offer_coin_id = partial_offer.coin.coin_id();
    let offer = Offer::from_spend_bundle(&mut ctx, &decode_offer(&offer_resp.offer)?)?;
    let sb = partial_offer.accept_offer(&mut ctx, offer)?;

    println!("Submitting transaction...");
    let client = get_coinset_client(testnet11);
    let resp = client.push_tx(sb).await?;

    println!("Transaction submitted; status='{}'", resp.status);
    wait_for_coin(&client, partial_offer_coin_id, true).await?;
    println!("Confirmed!");

    Ok(())
}
