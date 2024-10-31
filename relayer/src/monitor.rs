use ismp::consensus::{StateMachineHeight, StateMachineId};
use std::time::Duration;
use tesseract_beefy::BeefyConfig;
use tesseract_primitives::IsmpHost;
use tesseract_substrate::config::{Blake2SubstrateChain, KeccakSubstrateChain};

pub async fn monitor_clients(
	hyperbridge_config: BeefyConfig,
	configs: Vec<(StateMachineId, u64)>,
) -> anyhow::Result<()> {
	let hyperbridge = hyperbridge_config
		.clone()
		.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
		.await?;
	let provider = hyperbridge.provider();

	enum HealthStat {
		Restart,
		Ok,
	}

	let lambda = || async {
		for (id, max_interval) in configs.clone() {
			log::trace!(target: "tesseract", "Checking update interval for {:?}", id.state_id);
			let latest_height = provider.query_latest_height(id).await?;
			let state_machine_height = StateMachineHeight { id, height: latest_height.into() };
			let last_state_machine_update_time =
				provider.query_state_machine_update_time(state_machine_height).await?;

			let current_timestamp = provider.query_timestamp().await?;

			if current_timestamp
				.as_secs()
				.saturating_sub(last_state_machine_update_time.as_secs()) >=
				max_interval
			{
				log::trace!(target: "tesseract", "{:?} Has stalled shutting down", id.state_id);
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
