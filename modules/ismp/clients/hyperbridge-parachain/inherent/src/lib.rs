use anyhow::anyhow;
use cumulus_relay_chain_interface::RelayChainInterface;
use hyperbridge_ismp_parachain::{parachain_header_storage_key, HyperbridgeConsensusProof, INHERENT_IDENTIFIER};
use polkadot_sdk::*;
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_inherents::{Error, InherentData, InherentDataProvider, InherentIdentifier};
use sp_runtime::{ generic::BlockId, traits::{Block as BlockT, Hash as HashT, Header as HeaderT}};
use std::sync::Arc;

sp_api::decl_runtime_apis! {
    /// Hyperbridge Parachain consensus client runtime APIs
    pub trait HyperbridgeVerifierApi {
        /// Return the hyperbridge para id
        fn hyperbridge_para_id() -> u32;
        /// Return the current relay chain state.
        fn current_relay_chain_state() -> Option<cumulus_pallet_parachain_system::RelayChainState>;
    }
}

/// Provides the `HyperbridgeConsensusProof` inherent data.
pub struct HyperbridgeInherentProvider {
    relay_chain_interface: Arc<dyn RelayChainInterface>,
    consensus_proof: Option<HyperbridgeConsensusProof>,
}

impl HyperbridgeInherentProvider {
    /// Creates the inherent provider and fetches the necessary proof data for Hyperbridge.
    pub async fn create<C, B>(
        parent_hash: B::Hash,
        client: Arc<C>,
        relay_chain_interface: Arc<dyn RelayChainInterface>,
    ) -> Result<Self, anyhow::Error>
    where
        B: BlockT,
        B::Hash: HashT,
        C: ProvideRuntimeApi<B> + HeaderBackend<B> + Send + Sync + 'static,
        C::Api: HyperbridgeVerifierApi<B>,
        <B::Header as HeaderT>::Hash: HashT + From<B::Hash>,
    {
        if !client.runtime_api().has_api::<dyn HyperbridgeVerifierApi<B>>(parent_hash)? {
            log::trace!("HyperbridgeVerifierApi not implemented by runtime");
            return Ok(Self { relay_chain_interface, consensus_proof: None });
        }

        let hyperbridge_para_id = client.runtime_api().hyperbridge_para_id(parent_hash)?;
        log::trace!("Target Hyperbridge ParaId from runtime: {}", hyperbridge_para_id);

        let maybe_relay_state = client
            .runtime_api()
            .current_relay_chain_state(parent_hash)
            .map_err(|e| anyhow!("Failed to get current relay chain state: {:?}", e))?;

        let relay_state = match maybe_relay_state {
            Some(state) => state,
            None => {
                log::warn!("Runtime not providing relay chain state via API.");
                return Ok(Self { relay_chain_interface, consensus_proof: None });
            }
        };

        let relay_height = relay_state.number;
        log::trace!("Current relay chain height from runtime API: {}", relay_height);

        if relay_height == 0 {
            return Ok(Self { relay_chain_interface, consensus_proof: None });
        }

        let relay_header = match relay_chain_interface.header(BlockId::Number(relay_height)).await {
            Ok(Some(header)) => header,
            _ => {
                log::trace!("Relay chain header not available for height {}", relay_height);
                return Ok(Self { relay_chain_interface, consensus_proof: None });
            }
        };
        let relay_hash = relay_header.hash();

        let header_key = parachain_header_storage_key(hyperbridge_para_id).0;
        let keys_to_prove = vec![header_key];

        let storage_proof = match relay_chain_interface.prove_read(relay_hash, &keys_to_prove).await {
            Ok(proof) => proof.into_iter_nodes().collect(),
            Err(e) => {
                log::error!("Failed to get storage proof from relay chain for height {}: {:?}", relay_height, e);
                return Ok(Self { relay_chain_interface, consensus_proof: None });
            }
        };

        let proof = HyperbridgeConsensusProof {
            relay_height,
            storage_proof,
        };

        log::trace!("Successfully created Hyperbridge consensus proof for relay height {}", relay_height);
        Ok(Self { relay_chain_interface, consensus_proof: Some(proof) })
    }
}

#[async_trait::async_trait]
impl InherentDataProvider for HyperbridgeInherentProvider {
    async fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), Error> {
        if let Some(ref proof) = self.consensus_proof {
            inherent_data.put_data(INHERENT_IDENTIFIER, proof)?;
            log::trace!("Provided inherent data for {}", String::from_utf8_lossy(&INHERENT_IDENTIFIER));
        }
        Ok(())
    }

    async fn try_handle_error(&self, identifier: &InherentIdentifier, error: &[u8]) -> Option<Result<(), Error>> {
        if *identifier == INHERENT_IDENTIFIER {
            log::error!(
                target: "hyperbridge-inherent",
                "Inherent error for {}: {}",
                String::from_utf8_lossy(identifier),
                String::from_utf8_lossy(error)
            );
            None
        } else {
            None
        }
    }
}

