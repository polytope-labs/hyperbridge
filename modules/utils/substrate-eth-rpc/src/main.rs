use std::path::PathBuf;

use clap::{Parser, Subcommand};
use substrate_eth_rpc::Config;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser, Debug)]
#[command(name = "substrate-eth-rpc", about = "Ethereum JSON-RPC facade over Substrate chains")]
struct Cli {
	#[command(subcommand)]
	command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
	Serve {
		#[arg(long)]
		config: PathBuf,
	},
	EmitAbi {
		#[arg(long)]
		out: PathBuf,
	},
	RecordFixtures {
		#[arg(long)]
		config: PathBuf,
		#[arg(long)]
		chain: String,
		#[arg(long, value_delimiter = ',')]
		blocks: Vec<String>,
		#[arg(long)]
		out: PathBuf,
	},
}

fn setup_logging(default_filter: Option<&str>) {
	let filter = EnvFilter::try_from_default_env()
		.or_else(|_| EnvFilter::try_new(default_filter.unwrap_or("info")))
		.unwrap_or_else(|_| EnvFilter::new("info"));
	let use_ansi = std::env::var_os("NO_COLOR").is_none();
	tracing_subscriber::registry()
		.with(fmt::layer().with_ansi(use_ansi))
		.with(filter)
		.init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let cli = Cli::parse();
	match cli.command {
		Command::Serve { config } => {
			let config = Config::load(&config)?;
			setup_logging(config.log.as_deref());
			tracing::info!(listen = %config.listen, chains = config.chains.len(), "loaded config");
			substrate_eth_rpc::server::run(config).await
		},
		Command::EmitAbi { out } => substrate_eth_rpc::cli::run_emit_abi(&out),
		Command::RecordFixtures { config, chain, blocks, out } => {
			setup_logging(None);
			substrate_eth_rpc::cli::run_record_fixtures(&config, &chain, &blocks, &out).await
		},
	}
}
