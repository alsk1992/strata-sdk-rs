//! Read-only public CLI for humans and terminal agents.
//!
//! This binary deliberately has no wallet, keypair, RPC, admin, preparation,
//! or submission flags. Future write commands belong behind explicit public
//! capabilities and separate confirmation policy.

use clap::{Parser, Subcommand, ValueEnum};
use std::error::Error;
use std::time::Duration;
use strata_sdk::{QuoteRequest, QuoteSide, StrataClient, DEFAULT_API_BASE};

#[derive(Debug, Parser)]
#[command(
    name = "strata-agent",
    version,
    about = "Explore Strata and request validated Sonar quotes"
)]
struct Cli {
    #[arg(long, env = "STRATA_API_BASE", default_value = DEFAULT_API_BASE)]
    api_base: String,
    #[arg(long, default_value_t = 10)]
    timeout_secs: u64,
    /// Emit stable JSON for scripts and agents.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show the versioned public capability catalog.
    Capabilities,
    /// List available public markets.
    Markets {
        /// Include markets that are currently not quote-ready.
        #[arg(long)]
        all: bool,
    },
    /// Request a short-lived quote by market label or public market ID.
    Quote {
        #[arg(long)]
        market: String,
        #[arg(long, value_enum)]
        side: Side,
        /// Input amount in the input token's smallest atomic unit.
        #[arg(long)]
        amount_atoms: String,
        #[arg(long, default_value_t = 50)]
        slippage_bps: u16,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Side {
    Buy,
    Sell,
}

impl From<Side> for QuoteSide {
    fn from(value: Side) -> Self {
        match value {
            Side::Buy => QuoteSide::Buy,
            Side::Sell => QuoteSide::Sell,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let client = StrataClient::with_timeout(&cli.api_base, Duration::from_secs(cli.timeout_secs))?;

    match cli.command {
        Command::Capabilities => {
            let catalog = client.capabilities().await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&catalog)?);
            } else {
                println!("Strata public contract {}", catalog.contract_version);
                for capability in catalog.capabilities {
                    println!(
                        "{:<18} {:<8} {:<11} default={}",
                        capability.id,
                        format!("{:?}", capability.stability).to_lowercase(),
                        format!("{:?}", capability.risk).to_lowercase(),
                        if capability.default_enabled {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    );
                }
            }
        }
        Command::Markets { all } => {
            let mut response = client.markets().await?;
            if !all {
                response.markets.retain(|market| market.ready);
            }
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                println!("MARKET             STATUS   BASE/QUOTE DECIMALS");
                for market in response.markets {
                    println!(
                        "{:<18} {:<8} {}/{}",
                        market.label,
                        if market.ready { "ready" } else { "paused" },
                        market.base_decimals,
                        market.quote_decimals
                    );
                }
            }
        }
        Command::Quote {
            market,
            side,
            amount_atoms,
            slippage_bps,
        } => {
            let quote = client
                .quote(QuoteRequest {
                    market_id: market,
                    side: side.into(),
                    amount_in_atoms: amount_atoms,
                    slippage_bps,
                })
                .await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&quote)?);
            } else {
                println!("{} {} quote", quote.market_id, side_label(quote.side));
                println!("  input atoms:    {}", quote.amount_in_atoms);
                println!("  consumed atoms: {}", quote.amount_in_consumed_atoms);
                println!("  output atoms:   {}", quote.amount_out_atoms);
                println!("  minimum atoms:  {}", quote.minimum_output_atoms);
                println!("  input fee:      {}", quote.input_fee_atoms);
                println!("  output fee:     {}", quote.output_fee_atoms);
                println!("  reference:      {}", quote.reference_price);
                println!("  price impact:   {}%", quote.price_impact_pct);
                println!(
                    "  valid for:       {} ms",
                    quote.expires_at_ms.saturating_sub(quote.server_time_ms)
                );
                println!("  provider:        {}", quote.provider);
            }
        }
    }
    Ok(())
}

fn side_label(side: QuoteSide) -> &'static str {
    match side {
        QuoteSide::Buy => "buy",
        QuoteSide::Sell => "sell",
    }
}
