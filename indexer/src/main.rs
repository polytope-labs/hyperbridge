use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(name = "hyperbridge-indexer", about = "Indexes Hyperbridge into the tables omnihedron serves")]
struct Cli {
	/** Chain config, e.g. config/mainnet.toml. */
	#[arg(long, env = "INDEXER_CONFIG")]
	config: PathBuf,
	/** Directory the ABI paths in `Source::abi` resolve against. */
	#[arg(long, env = "INDEXER_ABI_ROOT", default_value = env!("CARGO_MANIFEST_DIR"))]
	abi_root: PathBuf,
	/** Where the generated rindexer manifest is written. */
	#[arg(long, env = "INDEXER_RUN_DIR", default_value = "/tmp/hyperbridge-indexer")]
	run_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	rindexer::setup_info_logger();
	let cli = Cli::parse();
	hyperbridge_indexer::run(&cli.config, &cli.abi_root, &cli.run_dir).await
}
