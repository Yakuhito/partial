use clap::{Parser, Subcommand};

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
enum Commands {}

pub async fn run_cli() {
    let _args = Cli::parse();

    // let res = match args.command {};

    // if let Err(err) = res {
    //     eprintln!("Error: {err}");
    // }
}
