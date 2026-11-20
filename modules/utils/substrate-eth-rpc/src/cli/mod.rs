pub mod emit_abi;
pub mod record_fixtures;

use std::path::Path;

use anyhow::anyhow;

use crate::config::Config;

pub fn run_emit_abi(out: &Path) -> anyhow::Result<()> {
	for path in emit_abi::write_abis(out)? {
		println!("{}", path.display());
	}
	Ok(())
}

pub async fn run_record_fixtures(
	config: &Path,
	chain: &str,
	blocks: &[String],
	out: &Path,
) -> anyhow::Result<()> {
	let config = Config::load(config)?;
	let chain = config
		.chains
		.iter()
		.find(|candidate| candidate.name == chain)
		.ok_or_else(|| anyhow!("chain {chain:?} is not in the config"))?;
	let blocks = record_fixtures::parse_blocks(blocks)?;
	record_fixtures::record(chain, &blocks, out).await
}
