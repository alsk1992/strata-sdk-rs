//! Capability-gated public CLI for humans and terminal agents.
//!
//! External agent owners control permission and signing. This binary accepts
//! public keys, detached signatures, and signed transactions, never private
//! keys, seed phrases, RPC credentials, or admin material.

use clap::{Parser, Subcommand, ValueEnum};
use std::error::Error;
use std::time::Duration;
use strata_sdk::{
    ExecutionChallengeRequest, ExecutionPrepareRequest, ExecutionSubmitRequest, QuoteRequest,
    QuoteSide, StrataClient, DEFAULT_API_BASE, DEFAULT_SLIPPAGE_BPS,
};

#[derive(Debug, Parser)]
#[command(
    name = "strata-agent",
    version,
    about = "Discover and traverse Strata's capability-gated action graph"
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
    /// Show the executable action topology and live node availability.
    ActionGraph,
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
        /// Optional maximum execution tolerance. The default requires exact output.
        #[arg(long, default_value_t = DEFAULT_SLIPPAGE_BPS)]
        slippage_bps: u16,
    },
    /// Request quote-bound authorization bytes for an external signer.
    ExecutionChallenge {
        #[arg(long)]
        market: String,
        #[arg(long)]
        quote_id: String,
        #[arg(long)]
        owner_wallet: String,
        #[arg(long)]
        session_public_key: String,
        #[arg(long)]
        account_sequence: String,
    },
    /// Exchange an external authorization signature for a prepared transaction.
    ExecutionPrepare {
        #[arg(long)]
        market: String,
        #[arg(long)]
        challenge_id: String,
        #[arg(long)]
        authorization_signature: String,
    },
    /// Submit an externally signed prepared transaction idempotently.
    ExecutionSubmit {
        #[arg(long)]
        market: String,
        #[arg(long)]
        execution_id: String,
        #[arg(long)]
        signed_transaction_base64: String,
        #[arg(long)]
        idempotency_key: String,
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
        Command::ActionGraph => {
            let graph = client.action_graph().await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&graph)?);
            } else {
                println!("Strata action graph {}", graph.graph_version);
                println!("  permission: {}", graph.authority.permission_source);
                println!("  signing:    {}", graph.authority.signing_location);
                for node in graph.nodes {
                    println!(
                        "{} {}: {}",
                        if node.available { "ready" } else { "off  " },
                        node.id,
                        node.summary
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
        Command::ExecutionChallenge {
            market,
            quote_id,
            owner_wallet,
            session_public_key,
            account_sequence,
        } => {
            let challenge = client
                .execution_challenge(
                    &market,
                    ExecutionChallengeRequest {
                        quote_id,
                        owner_wallet,
                        session_public_key,
                        account_sequence,
                    },
                )
                .await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&challenge)?);
            } else {
                println!("challenge:     {}", challenge.challenge_id);
                println!("authorization: {}", challenge.authorization_payload_base64);
                println!("expires:       {}", challenge.expires_at_ms);
            }
        }
        Command::ExecutionPrepare {
            market,
            challenge_id,
            authorization_signature,
        } => {
            let prepared = client
                .execution_prepare(
                    &market,
                    ExecutionPrepareRequest {
                        challenge_id,
                        authorization_signature,
                    },
                )
                .await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&prepared)?);
            } else {
                println!("execution:   {}", prepared.execution_id);
                println!("transaction: {}", prepared.transaction_base64);
                println!("expires:     {}", prepared.expires_at_ms);
            }
        }
        Command::ExecutionSubmit {
            market,
            execution_id,
            signed_transaction_base64,
            idempotency_key,
        } => {
            let receipt = client
                .execution_submit(
                    &market,
                    ExecutionSubmitRequest {
                        execution_id,
                        signed_transaction_base64,
                        idempotency_key,
                    },
                )
                .await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&receipt)?);
            } else {
                println!("execution: {}", receipt.execution_id);
                println!("signature: {}", receipt.signature);
                println!("status:    submitted");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_defaults_to_exact_output() {
        let cli = Cli::try_parse_from([
            "strata-agent",
            "quote",
            "--market",
            "SOL/USDC",
            "--side",
            "sell",
            "--amount-atoms",
            "10000000",
        ])
        .expect("valid quote command");

        let Command::Quote { slippage_bps, .. } = cli.command else {
            panic!("expected quote command");
        };
        assert_eq!(slippage_bps, DEFAULT_SLIPPAGE_BPS);
    }
}
