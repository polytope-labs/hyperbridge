pub use pallet::*;
use pallet_ismp::host::Host;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use crate::{beacon_client::BEACON_CONSENSUS_STATE_ID, types::ConsensusState};
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use ismp::host::{IsmpHost, StateMachine};
    use sp_core::{H160, H256};

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// The config trait
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_ismp::Config {
        /// Origin allowed to add or remove parachains in Consensus State
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Contract Address Already Exists
        ContractAddressAlreadyExists,
        /// Contract Address Consensus Does not Exist
        ContractAddressDontExists,
        /// Error fetching consensus state
        ErrorFetchingConsensusState,
        /// Error decoding consensus state
        ErrorDecodingConsensusState,
        /// Incorrect consensus state id length
        IncorrectConsensusStateIdLength,
        /// Error storing consensus state
        ErrorStoringConsensusState,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        <T as frame_system::Config>::Hash: From<H256>,
    {
        /// Add or update an ismp contract address
        #[pallet::call_index(0)]
        #[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1))]
        pub fn update_ismp_address(
            origin: OriginFor<T>,
            contract_address: H160,
            state_machine: StateMachine,
        ) -> DispatchResult {
            <T as Config>::AdminOrigin::ensure_origin(origin)?;

            let ismp_host = Host::<T>::default();

            let encoded_consensus_state = ismp_host
                .consensus_state(BEACON_CONSENSUS_STATE_ID)
                .map_err(|_| Error::<T>::ErrorFetchingConsensusState)?;
            let mut consensus_state: ConsensusState =
                codec::Decode::decode(&mut &encoded_consensus_state[..])
                    .map_err(|_| Error::<T>::ErrorDecodingConsensusState)?;

            consensus_state.ismp_contract_addresses.insert(state_machine, contract_address);

            let encoded_consensus_state = consensus_state.encode();
            ismp_host
                .store_consensus_state(BEACON_CONSENSUS_STATE_ID, encoded_consensus_state)
                .map_err(|_| Error::<T>::ErrorStoringConsensusState)?;
            Ok(())
        }

        /// Update l2 oracle or rollup core address
        #[pallet::call_index(1)]
        #[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1))]
        pub fn update_l2_addresses(
            origin: OriginFor<T>,
            l2_oracle: Option<H160>,
            rollup_core: Option<H160>,
        ) -> DispatchResult {
            <T as Config>::AdminOrigin::ensure_origin(origin)?;

            let ismp_host = Host::<T>::default();

            let encoded_consensus_state = ismp_host
                .consensus_state(BEACON_CONSENSUS_STATE_ID)
                .map_err(|_| Error::<T>::ErrorFetchingConsensusState)?;
            let mut consensus_state: ConsensusState =
                codec::Decode::decode(&mut &encoded_consensus_state[..])
                    .map_err(|_| Error::<T>::ErrorDecodingConsensusState)?;
            if let Some(l2_oracle) = l2_oracle {
                consensus_state.l2_oracle_address = l2_oracle;
            }
            if let Some(rollup_core) = rollup_core {
                consensus_state.rollup_core_address = rollup_core;
            }

            let encoded_consensus_state = consensus_state.encode();
            ismp_host
                .store_consensus_state(BEACON_CONSENSUS_STATE_ID, encoded_consensus_state)
                .map_err(|_| Error::<T>::ErrorStoringConsensusState)?;
            Ok(())
        }
    }
}
