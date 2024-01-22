use anyhow::anyhow;
use codec::{Decode, Encode};
use ismp_bnb_pos::ConsensusState;

use bnb_pos_verifier::primitives::{compute_epoch, BnbClientUpdate, EPOCH_LENGTH};
use ethers::types::Block;
use ismp::messaging::{ConsensusMessage, Message};
use primitive_types::H256;

use tesseract_primitives::{IsmpHost, IsmpProvider};

use crate::{BnbPosHost, KeccakHasher};

pub async fn consensus_notification<C>(
	client: &BnbPosHost,
	counterparty: C,
	_block: Block<H256>,
) -> Result<Option<BnbClientUpdate>, anyhow::Error>
where
	C: IsmpHost + IsmpProvider + 'static,
{
	let consensus_state =
		counterparty.query_consensus_state(None, client.consensus_state_id).await?;
	let consensus_state = ConsensusState::decode(&mut &*consensus_state)?;

	let current_epoch = consensus_state.current_epoch;
	let attested_header = client.prover.latest_header().await?;

	let attested_epoch = compute_epoch(attested_header.number.low_u64());

	if attested_epoch < current_epoch ||
		consensus_state.finalized_height >= attested_header.number.low_u64()
	{
		return Ok(None);
	}

	if attested_epoch > current_epoch {
		let mut last_update = None;
		let mut next_epoch = current_epoch + 1;
		loop {
			if next_epoch > attested_epoch {
				break;
			}
			let epoch_block_number = next_epoch * EPOCH_LENGTH;
			let epoch_header = client.prover.fetch_header(epoch_block_number).await?;
			let bnb_client_update = client
				.prover
				.fetch_bnb_update::<KeccakHasher>(epoch_header)
				.await?
				.ok_or_else(|| anyhow!("Sync failed"))?;
			last_update = Some(bnb_client_update.attested_header.number.low_u64());
			let message = ConsensusMessage {
				consensus_proof: bnb_client_update.encode(),
				consensus_state_id: client.consensus_state_id,
			};

			counterparty.submit(vec![Message::Consensus(message)]).await?;
			next_epoch += 1;
		}
		if let Some(last_update) = last_update {
			if last_update >= attested_header.number.low_u64() {
				return Ok(None)
			}
		}
		let bnb_client_update =
			client.prover.fetch_bnb_update::<KeccakHasher>(attested_header).await?;
		return Ok(bnb_client_update);
	}

	let bnb_client_update = client.prover.fetch_bnb_update::<KeccakHasher>(attested_header).await?;
	return Ok(bnb_client_update);
}
