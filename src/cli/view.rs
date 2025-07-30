use chia_wallet_sdk::{coinset::ChiaRpcClient, driver::SpendContext, utils::Address};
use slot_machine::{CliError, get_coinset_client, get_prefix};

use crate::{PartialOffer, decode_partial_offer};

pub async fn cli_view(offer: String, testnet11: bool) -> Result<(), CliError> {
    let mut ctx = SpendContext::new();

    let offer = PartialOffer::from_spend_bundle(&mut ctx, decode_partial_offer(&offer)?)?;

    if let Some(requested_asset_id) = offer.info.requested_asset_info.asset_id {
        println!(
            "Remaining requested amount: {:.3} (asset id: {})",
            PartialOffer::reverse_quote(offer.coin.amount, offer.info.price_data) as f64 / 1000.0,
            hex::encode(requested_asset_id)
        );
    } else {
        println!(
            "Remaining requested amount: {:.12} XCH",
            PartialOffer::reverse_quote(offer.coin.amount, offer.info.price_data) as f64 / 1e12,
        );
    }

    if let Some(offered_asset_id) = offer.info.offered_asset_info.asset_id {
        println!(
            "Remaining offered amount: {:.3} (asset id: {})",
            offer.coin.amount as f64 / 1000.0,
            hex::encode(offered_asset_id)
        );
    } else {
        println!(
            "Remaining offered amount: {:.12} XCH",
            offer.coin.amount as f64 / 1e12,
        );
    }

    println!("Expiration: {:?}", offer.info.expiration);
    println!("Required fee: {:?}", offer.info.required_fee);
    println!("Pricing data: {:?}", offer.info.price_data);
    println!(
        "Maker address: {}",
        Address::new(offer.info.maker_puzzle_hash, get_prefix(testnet11)).encode()?
    );

    let client = get_coinset_client(testnet11);

    let mut coin_ids = offer
        .spend_bundle
        .coin_spends
        .iter()
        .map(|cs| cs.coin.coin_id())
        .collect::<Vec<_>>();
    coin_ids.push(offer.coin.coin_id());

    let resp = client
        .get_coin_records_by_names(coin_ids, None, None, Some(true))
        .await?;

    // Yes, this can be tricked - but works for 'normal' cancellations
    println!(
        "Active: {:?}",
        !resp.coin_records.unwrap().iter().any(|cr| cr.spent)
    );

    Ok(())
}
