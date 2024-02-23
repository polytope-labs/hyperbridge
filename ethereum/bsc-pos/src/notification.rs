use codec::Decode;
use ismp_bsc_pos::ConsensusState;

use bsc_pos_verifier::primitives::{compute_epoch, BscClientUpdate};
use ethers::types::Block;
use primitive_types::H256;

use tesseract_primitives::{IsmpHost, IsmpProvider};

use crate::{BscPosHost, KeccakHasher};

pub async fn consensus_notification<C>(
	client: &BscPosHost,
	counterparty: C,
	_block: Block<H256>,
) -> Result<Option<BscClientUpdate>, anyhow::Error>
where
	C: IsmpHost + IsmpProvider + 'static,
{
	let consensus_state =
		counterparty.query_consensus_state(None, client.consensus_state_id).await?;
	let consensus_state = ConsensusState::decode(&mut &*consensus_state)?;
	let current_epoch = compute_epoch(consensus_state.finalized_height);
	let attested_header = client.prover.latest_header().await?;

	let attested_epoch = compute_epoch(attested_header.number.low_u64());

	if attested_epoch < current_epoch ||
		consensus_state.finalized_height >= attested_header.number.low_u64()
	{
		return Ok(None);
	}

	let bsc_client_update = client.prover.fetch_bsc_update::<KeccakHasher>(attested_header).await?;
	return Ok(bsc_client_update);
}
