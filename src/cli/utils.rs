use sage_api::{Amount, OfferAmount};

pub fn assets_cat_only(asset_id: String, cat_amount: u64) -> Vec<OfferAmount> {
    vec![OfferAmount {
        asset_id: Some(asset_id),
        hidden_puzzle_hash: None,
        amount: Amount::u64(cat_amount),
    }]
}

pub fn assets_cats_only(cats: Vec<(String, u64)>) -> Vec<OfferAmount> {
    cats.into_iter()
        .map(|(asset_id, amount)| OfferAmount {
            asset_id: Some(asset_id),
            hidden_puzzle_hash: None,
            amount: Amount::u64(amount),
        })
        .collect()
}
