use chia_protocol::SpendBundle;
use chia_wallet_sdk::{
    driver::{DriverError, compress_offer, decompress_offer},
    utils::Bech32,
};

pub fn encode_partial_offer(spend_bundle: &SpendBundle) -> Result<String, DriverError> {
    encode_partial_offer_data(&compress_offer(spend_bundle)?)
}

pub fn decode_partial_offer(text: &str) -> Result<SpendBundle, DriverError> {
    decompress_offer(&decode_partial_offer_data(text)?)
}

pub fn encode_partial_offer_data(offer: &[u8]) -> Result<String, DriverError> {
    Ok(Bech32::new(offer.to_vec().into(), "partial".to_string()).encode()?)
}

pub fn decode_partial_offer_data(offer: &str) -> Result<Vec<u8>, DriverError> {
    Ok(Bech32::decode(offer)?.expect_prefix("partial")?.to_vec())
}
