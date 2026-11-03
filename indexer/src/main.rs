use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(
	name = "hyperbridge-indexer",
	about = "Indexes Hyperbridge into the tables omnihedron serves"
)]
struct Cli {
	/** Chain config, e.g. config/mainnet.toml. */
	#[arg(long, env = "INDEXER_CONFIG")]
	config: PathBuf,
	/** Where the generated rindexer manifest is written. */
	#[arg(long, env = "INDEXER_RUN_DIR", default_value = "/tmp/hyperbridge-indexer")]
	run_dir: PathBuf,
	/** Chains to index, comma-separated; all of the config's when unset. Run one process per
	 * chain so a slow or stuck chain can't hold back the others. */
	#[arg(long, env = "INDEXER_CHAINS", value_delimiter = ',')]
	chains: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	// rindexer and reqwest compile in different rustls backends, so pick one for the process.
	rustls::crypto::ring::default_provider()
		.install_default()
		.map_err(|_| anyhow::anyhow!("a rustls crypto provider was already installed"))?;
	rindexer::setup_info_logger();
	let cli = Cli::parse();
	hyperbridge_indexer::run(&cli.config, &cli.run_dir, &cli.chains).await
}
