use clap::{Parser, Subcommand};

use crate::{cli_cancel, cli_create, cli_take, cli_view};

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
    // Create a partial offer
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
        #[arg(long, default_value = "0.00042")]
        fee: String,

        /// Use testnet11
        #[arg(long, default_value = "false")]
        testnet11: bool,
    },
    // View details of a partial offer
    View {
        /// Offer
        #[arg(long)]
        offer: String,

        /// Use testnet11
        #[arg(long, default_value = "false")]
        testnet11: bool,
    },
    // Take a partial offer
    Take {
        /// Offer
        #[arg(long)]
        offer: String,

        /// Amount of requested asset (the one you give) to use
        #[arg(long)]
        amount: String,

        /// Fee to include in partial offer
        #[arg(long, default_value = "0.00042")]
        fee: String,

        /// Use testnet11
        #[arg(long, default_value = "false")]
        testnet11: bool,
    },
    // Cancel a partial offer
    Cancel {
        /// Offer
        #[arg(long)]
        offer: String,

        /// Fee to include in partial offer
        #[arg(long, default_value = "0.00042")]
        fee: String,

        /// Use testnet11
        #[arg(long, default_value = "false")]
        testnet11: bool,
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
            testnet11,
        } => {
            cli_create(
                offered_asset_id,
                offered_amount,
                asked_asset_id,
                asked_amount,
                expiration,
                fee,
                testnet11,
            )
            .await
        }
        Commands::View { offer, testnet11 } => cli_view(offer, testnet11).await,
        Commands::Take {
            offer,
            amount,
            fee,
            testnet11,
        } => cli_take(offer, amount, fee, testnet11).await,
        Commands::Cancel {
            offer,
            fee,
            testnet11,
        } => cli_cancel(offer, fee, testnet11).await,
    };

    if let Err(err) = res {
        eprintln!("Error: {err}");
    }
}
