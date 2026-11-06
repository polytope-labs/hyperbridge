use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "omniplex", about = "Indexes Hyperbridge into the tables omnihedron serves")]
struct Cli {
	/** Chain config, e.g. config/mainnet.toml. */
	#[arg(long, env = "INDEXER_CONFIG")]
	config: PathBuf,
	/** Where the generated rindexer manifest is written, in a folder per set of chains. */
	#[arg(long, env = "INDEXER_RUN_DIR", default_value = "/tmp/omniplex")]
	run_dir: PathBuf,
	/** Chains to index, comma-separated; all of the config's when unset. Run one process per
	 * chain so a slow or stuck chain can't hold back the others. */
	#[arg(long, env = "INDEXER_CHAINS", value_delimiter = ',')]
	chains: Vec<String>,
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	// rindexer and reqwest compile in different rustls backends, so pick one for the process.
	rustls::crypto::ring::default_provider()
		.install_default()
		.map_err(|_| anyhow::anyhow!("a rustls crypto provider was already installed"))?;
	rindexer::setup_info_logger();
	let cli = Cli::parse();
	match cli.command {
		None => omniplex::run(&cli.config, &cli.run_dir, &cli.chains).await,
		Some(Command::ImportSubquery { new_chains }) =>
			omniplex::import_subquery(&cli.config, &new_chains).await,
	}
}
