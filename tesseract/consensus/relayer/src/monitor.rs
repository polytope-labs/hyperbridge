use anyhow::anyhow;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	host::StateMachine,
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tesseract_primitives::IsmpHost;
use tesseract_substrate::config::{Blake2SubstrateChain, KeccakSubstrateChain};

use crate::any::HyperbridgeHostConfig;

pub async fn monitor_clients(
	hyperbridge_config: HyperbridgeHostConfig,
	client_map: HashMap<StateMachine, Arc<dyn IsmpHost>>,
	configs: Vec<(StateMachineId, u64)>,
) -> anyhow::Result<()> {
	let hyperbridge = hyperbridge_config
		.clone()
		.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
		.await?;
	let hyperbridge_provider = hyperbridge.provider();

	enum HealthStat {
		Restart,
		Ok,
	}

	let lambda = || async {
		for (id, max_interval) in configs.clone() {
			if id == hyperbridge_provider.state_machine_id() {
				continue;
			}
			log::trace!(target: "tesseract", "Checking update interval for {:?} on {:?}", id.state_id, hyperbridge_provider.state_machine_id().state_id);
			let latest_height = hyperbridge_provider.query_latest_height(id).await?;
			let state_machine_height = StateMachineHeight { id, height: latest_height.into() };
			let last_state_machine_update_time = hyperbridge_provider
				.query_state_machine_update_time(state_machine_height)
				.await?;

			let current_timestamp = hyperbridge_provider.query_timestamp().await?;

			if current_timestamp
				.as_secs()
				.saturating_sub(last_state_machine_update_time.as_secs()) >=
				max_interval
			{
				log::trace!(target: "tesseract", "{:?} -> {:?} Has stalled shutting down", id.state_id, hyperbridge_provider.state_machine_id().state_id);
				return Ok::<_, anyhow::Error>(HealthStat::Restart);
			}

			// Don't check update interval for hyperbridge on evm chains
			if id.state_id.is_evm() {
				continue;
			}

			log::trace!(target: "tesseract", "Checking update interval for {:?} on {:?}", hyperbridge_provider.state_machine_id().state_id, id.state_id);
			let provider = client_map
				.get(&id.state_id)
				.cloned()
				.ok_or_else(|| anyhow!("Client for {:?} not found in config", id.state_id))?
				.provider();
			let latest_height =
				provider.query_latest_height(hyperbridge_provider.state_machine_id()).await?;
			let state_machine_height = StateMachineHeight {
				id: hyperbridge_provider.state_machine_id(),
				height: latest_height.into(),
			};
			let last_state_machine_update_time =
				provider.query_state_machine_update_time(state_machine_height).await?;

			let current_timestamp = provider.query_timestamp().await?;

			if current_timestamp
				.as_secs()
				.saturating_sub(last_state_machine_update_time.as_secs()) >=
				max_interval
			{
				log::trace!(target: "tesseract", "{:?} -> {:?} Has stalled shutting down", hyperbridge_provider.state_machine_id().state_id, id.state_id);
				return Ok::<_, anyhow::Error>(HealthStat::Restart);
			}
		}

		Ok(HealthStat::Ok)
	};

	// Sleep for some minutes to allow some initial updates before starting the monitoring
	tokio::time::sleep(Duration::from_secs(600)).await;

	loop {
		match lambda().await {
			Ok(HealthStat::Restart) => break,
			_ => tokio::time::sleep(Duration::from_secs(180)).await,
		}
	}

	Ok(())
}
