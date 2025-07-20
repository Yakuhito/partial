use bech32::{Variant, u5};
use chia::protocol::SpendBundle;
use chia_wallet_sdk::driver::{DriverError, compress_offer, decompress_offer};

pub fn encode_partial_offer(spend_bundle: &SpendBundle) -> Result<String, DriverError> {
    encode_partial_offer_data(&compress_offer(spend_bundle)?)
}

pub fn decode_partial_offer(text: &str) -> Result<SpendBundle, DriverError> {
    decompress_offer(&decode_partial_offer_data(text)?)
}

pub fn encode_partial_offer_data(offer: &[u8]) -> Result<String, DriverError> {
    let data = bech32::convert_bits(offer, 8, 5, true)?
        .into_iter()
        .map(u5::try_from_u8)
        .collect::<Result<Vec<_>, bech32::Error>>()?;
    Ok(bech32::encode("partial", data, Variant::Bech32m)?)
}

pub fn decode_partial_offer_data(offer: &str) -> Result<Vec<u8>, DriverError> {
    let (hrp, data, variant) = bech32::decode(offer)?;

    if variant != Variant::Bech32m {
        return Err(DriverError::InvalidFormat);
    }

    if hrp.as_str() != "partial" {
        return Err(DriverError::InvalidPrefix(hrp));
    }

    Ok(bech32::convert_bits(&data, 5, 8, false)?)
}
