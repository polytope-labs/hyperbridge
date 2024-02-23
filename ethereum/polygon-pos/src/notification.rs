use anyhow::anyhow;
use codec::{Decode, Encode};
use ismp_polygon_pos::{ConsensusState, PolygonClientUpdate};

use ethers::types::Block;
use geth_primitives::CodecHeader;
use ismp::messaging::{ConsensusMessage, Message};
use polygon_pos_prover::PolygonPosProver;
use primitive_types::H256;

use tesseract_primitives::{IsmpHost, IsmpProvider};

use crate::PolygonPosHost;

pub async fn consensus_notification<C>(
	client: &PolygonPosHost,
	counterparty: C,
	block: Block<H256>,
) -> Result<Option<PolygonClientUpdate>, anyhow::Error>
where
	C: IsmpHost + IsmpProvider + 'static,
{
	let consensus_state =
		counterparty.query_consensus_state(None, client.consensus_state_id).await?;
	let consensus_state = ConsensusState::decode(&mut &*consensus_state)?;

	let mut chain_heads = consensus_state
		.forks
		.iter()
		.map(|chain| chain.hashes[chain.hashes.len() - 1].1)
		.collect::<Vec<_>>();
	chain_heads.push(consensus_state.finalized_hash);
	let mut headers = next_headers(
		&client.prover,
		chain_heads.clone(),
		block.hash.ok_or_else(|| anyhow!("Hash should be present in block"))?,
		consensus_state.finalized_hash,
	)
	.await?;

	if headers.len() > 1000 {
		for chunk in headers.chunks(1000) {
			let chain_head = chunk[0].parent_hash;
			let update = PolygonClientUpdate {
				consensus_update: chunk.to_vec().try_into().expect("Infallible"),
				chain_head,
			};
			let message = ConsensusMessage {
				consensus_proof: update.encode(),
				consensus_state_id: client.consensus_state_id,
			};
			let _ = counterparty.submit(vec![Message::Consensus(message)]).await;
		}
		headers = vec![];
	}

	if !headers.is_empty() {
		let chain_head = headers[0].parent_hash;
		Ok(Some(PolygonClientUpdate {
			consensus_update: headers.try_into().expect("Infallible"),
			chain_head,
		}))
	} else {
		Ok(None)
	}
}

// Find the next headers to be submitted

async fn next_headers(
	prover: &PolygonPosProver,
	chain_heads: Vec<H256>,
	latest_block_hash: H256,
	finalized_hash: H256,
) -> Result<Vec<CodecHeader>, anyhow::Error> {
	let mut headers = vec![];
	let latest_header = prover
		.fetch_header(latest_block_hash)
		.await?
		.ok_or_else(|| anyhow!("Header should exist"))?;
	let finalized_header = prover
		.fetch_header(finalized_hash)
		.await?
		.ok_or_else(|| anyhow!("Header should exist"))?;
	let mut parent_hash = latest_header.parent_hash;
	headers.push(latest_header);
	while !chain_heads.contains(&parent_hash) {
		let header = prover
			.fetch_header(parent_hash)
			.await?
			.ok_or_else(|| anyhow!("Header should exist"))?;
		if header.number <= finalized_header.number {
			// Unknown chain
			headers = vec![];
			break
		}
		parent_hash = header.parent_hash;
		headers.insert(0, header)
	}
	Ok(headers)
}
