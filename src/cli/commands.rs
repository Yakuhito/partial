use clap::{Parser, Subcommand};

use crate::cli_create;

#[derive(Parser)]
#[command(
    name = "Partial CLI",
    about = "A CLI for interacting with partial offers"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Create {
        /// Offered asset id
        #[arg(long)]
        offered_asset_id: Option<String>,

        /// Offered amount
        #[arg(long)]
        offered_amount: String,

        /// Asked asset id
        #[arg(long)]
        asked_asset_id: Option<String>,

        /// Asked amount
        #[arg(long)]
        asked_amount: String,

        /// Expiration slot
        #[arg(long)]
        expiration: Option<u64>,

        /// Fee to include in partial offer
        #[arg(long, default_value = "0.0025")]
        fee: u64,
    },
}

pub async fn run_cli() {
    let args = Cli::parse();

    let res = match args.command {
        Commands::Create {
            offered_asset_id,
            offered_amount,
            asked_asset_id,
            asked_amount,
            expiration,
            fee,
        } => {
            cli_create(
                offered_asset_id,
                offered_amount,
                asked_asset_id,
                asked_amount,
                expiration,
                fee,
            )
            .await
        }
    };

    if let Err(err) = res {
        eprintln!("Error: {err}");
    }
}
