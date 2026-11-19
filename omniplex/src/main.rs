use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "omniplex", about = "Indexes Hyperbridge into the tables omnihedron serves")]
struct Cli {
	/** Chain config, e.g. config/mainnet.toml. Every command but `explorer-api` needs one. */
	#[arg(long, env = "INDEXER_CONFIG")]
	config: Option<PathBuf>,
	/** Where the generated rindexer manifest is written, in a folder per set of chains. */
	#[arg(long, env = "INDEXER_RUN_DIR", default_value = "/tmp/omniplex")]
	run_dir: PathBuf,
	/// Chains to index, comma-separated; all of the config's when unset. Run one process per
	/// chain so a slow or stuck chain can't hold back the others.
	#[arg(long, env = "INDEXER_CHAINS", value_delimiter = ',')]
	chains: Vec<String>,
	/// Stops chains at these blocks instead of going live, as `chain=block` pairs separated by
	/// commas. For checking parity with SubQuery at a copy's heights.
	#[arg(long, env = "INDEXER_END_BLOCKS", value_delimiter = ',', value_parser = parse_end_block)]
	end_blocks: Vec<(String, u64)>,
	#[command(subcommand)]
	command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
	/** Turns a copy of SubQuery's database, at DATABASE_URL, into this indexer's. */
	ImportSubquery {
		/** Chains SubQuery never indexed, comma-separated. They start at their start block. */
		#[arg(long, value_delimiter = ',')]
		new_chains: Vec<String>,
	},
	/** Serves the block explorer's tables to the Statescan site. */
	ExplorerApi {
		#[arg(long, env = "EXPLORER_API_LISTEN", default_value = "0.0.0.0:7011")]
		listen: std::net::SocketAddr,
	},
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	// rindexer and reqwest compile in different rustls backends, so pick one for the process.
	rustls::crypto::ring::default_provider()
		.install_default()
		.map_err(|_| anyhow::anyhow!("a rustls crypto provider was already installed"))?;
	rindexer::setup_info_logger();
	let cli = Cli::parse();
	let config = || cli.config.as_deref().context("--config or INDEXER_CONFIG is required");
	match cli.command {
		None => omniplex::run(config()?, &cli.run_dir, &cli.chains, &cli.end_blocks).await,
		Some(Command::ImportSubquery { ref new_chains }) =>
			omniplex::import_subquery(config()?, new_chains).await,
		Some(Command::ExplorerApi { listen }) => omniplex::explorer_api::serve(listen).await,
	}
}

fn parse_end_block(value: &str) -> Result<(String, u64), String> {
	let (chain, block) = value.split_once('=').ok_or("expected chain=block")?;
	let block = block.parse().map_err(|e| format!("{chain}: {e}"))?;
	Ok((chain.to_string(), block))
}
