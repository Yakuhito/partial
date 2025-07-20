use slot_machine::CliError;

pub async fn cli_create(
    offered_asset_id: Option<String>,
    offered_amount: u64,
    asked_asset_id: Option<String>,
    asked_amount: u64,
    expiration: Option<u64>,
    fee: u64,
) -> Result<(), CliError> {
    Ok(())
}
