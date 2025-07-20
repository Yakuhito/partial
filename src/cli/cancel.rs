use chia::protocol::SpendBundle;
use chia_wallet_sdk::{
    coinset::ChiaRpcClient,
    driver::{
        Offer, Spend, SpendContext, StandardLayer, create_security_coin, decode_offer,
        spend_security_coin,
    },
    types::Conditions,
    utils::Address,
};
use clvm_traits::clvm_quote;
use clvmr::NodePtr;
use slot_machine::{
    CliError, SageClient, assets_xch_only, get_coinset_client, get_constants, hex_string_to_pubkey,
    hex_string_to_signature, no_assets, parse_amount, wait_for_coin,
};

use crate::{PartialOffer, decode_partial_offer};

pub async fn cli_cancel(offer: String, fee_str: String, testnet11: bool) -> Result<(), CliError> {
    let fee = parse_amount(&fee_str, false)?;
    let mut ctx = SpendContext::new();

    let partial_offer = PartialOffer::from_spend_bundle(&mut ctx, decode_partial_offer(&offer)?)?;

    let sage = SageClient::new()?;
    let derivation_resp = &sage.get_derivations(false, 0, 1).await?.derivations[0];
    if Address::decode(&derivation_resp.address)?.puzzle_hash
        != partial_offer.info.maker_puzzle_hash
    {
        return Err(CliError::Custom(
            "You are not the maker of this offer".to_string(),
        ));
    }

    let offer_resp = sage
        .make_offer(no_assets(), assets_xch_only(1), fee, None, None, false)
        .await?;

    println!("Offer {} created.", offer_resp.offer_id);

    let partial_offer_coin_id = partial_offer.coin.coin_id();
    let offer = Offer::from_spend_bundle(&mut ctx, &decode_offer(&offer_resp.offer)?)?;

    let (security_sk, security_coin) =
        create_security_coin(&mut ctx, offer.offered_coins().xch[0])?;
    let security_sig = spend_security_coin(
        &mut ctx,
        security_coin,
        Conditions::new().assert_concurrent_spend(partial_offer_coin_id),
        &security_sk,
        get_constants(testnet11),
    )?;

    let quoted_conds = clvm_quote!(Conditions::new().create_coin(
        partial_offer.info.maker_puzzle_hash,
        partial_offer.coin.amount,
        ctx.hint(partial_offer.info.maker_puzzle_hash)?
    ));
    let inner_spend = Spend::new(ctx.alloc(&quoted_conds)?, NodePtr::NIL);
    let inner_spend = StandardLayer::new(hex_string_to_pubkey(&derivation_resp.public_key)?)
        .delegated_inner_spend(&mut ctx, inner_spend)?;
    partial_offer.claw_back(&mut ctx, inner_spend)?;

    let spends = ctx.take();
    let resp = sage.sign_coin_spends(spends.clone(), false, true).await?;

    let sig_from_signing = hex_string_to_signature(&resp.spend_bundle.aggregated_signature)?;
    let sb = offer.take(SpendBundle::new(spends, security_sig + &sig_from_signing));

    println!("Submitting transaction...");
    let client = get_coinset_client(testnet11);
    let resp = client.push_tx(sb).await?;

    println!("Transaction submitted; status='{}'", resp.status);
    wait_for_coin(&client, partial_offer_coin_id, true).await?;
    println!("Confirmed!");

    Ok(())
}
