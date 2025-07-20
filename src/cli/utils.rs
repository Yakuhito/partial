use sage_api::{Amount, Assets, CatAmount};

pub fn assets_cat_only(asset_id: String, cat_amount: u64) -> Assets {
    Assets {
        xch: Amount::u64(0),
        cats: vec![CatAmount {
            asset_id,
            amount: Amount::u64(cat_amount),
        }],
        nfts: vec![],
    }
}

pub fn assets_cats_only(cats: Vec<(String, u64)>) -> Assets {
    Assets {
        xch: Amount::u64(0),
        cats: cats
            .into_iter()
            .map(|(asset_id, amount)| CatAmount {
                asset_id,
                amount: Amount::u64(amount),
            })
            .collect(),
        nfts: vec![],
    }
}
