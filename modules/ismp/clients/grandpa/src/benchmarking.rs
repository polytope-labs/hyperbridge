#![cfg(feature = "runtime-benchmarks")]
use super::*;
use frame_benchmarking::v2::*;
use frame_support::pallet_prelude::Weight;
use frame_system::RawOrigin;
use sp_std::prelude::*;

/// Benchmarks for the ISMP GRANDPA pallet operations
#[benchmarks]
mod benchmarks {
    use super::*;

    /// Benchmark for add_state_machines extrinsic
    /// The benchmark creates n state machines and measures the time to add them
    /// to the whitelist.
    ///
    /// Parameters:
    /// - `n`: Number of state machines to add in a single call
    #[benchmark]
    fn add_state_machines(n: Linear<1, 100>) -> Result<(), BenchmarkError> {
        let caller: T::AccountId = whitelisted_caller();
        
        let state_machines: Vec<AddStateMachine> = (0..n)
            .map(|i| {
                let id = [i as u8, 0, 0, 0]; // Create unique 4-byte identifier
                AddStateMachine {
                    state_machine: StateMachine::Substrate(id),
                    slot_duration: 6000u64,
                }
            })
            .collect();

        #[extrinsic_call]
        _(RawOrigin::Root, state_machines);

        // Verify operation was successful
        assert!(SupportedStateMachines::<T>::iter().count() == n as usize);
        Ok(())
    }

    /// Benchmark for remove_state_machines extrinsic
    /// The benchmark first adds n state machines, then measures the time to remove them
    /// from the whitelist.
    ///
    /// Parameters:
    /// - `n`: Number of state machines to remove in a single call
    #[benchmark]
    fn remove_state_machines(n: Linear<1, 100>) -> Result<(), BenchmarkError> {
        let caller: T::AccountId = whitelisted_caller();
        
        // Setup: First add state machines that we'll remove
        let setup_machines: Vec<AddStateMachine> = (0..n)
            .map(|i| {
                let id = [i as u8, 0, 0, 0]; // Create unique 4-byte identifier
                AddStateMachine {
                    state_machine: StateMachine::Substrate(id),
                    slot_duration: 6000u64,
                }
            })
            .collect();

        // Add the machines using root origin
        Pallet::<T>::add_state_machines(
            RawOrigin::Root.into(),
            setup_machines.clone(),
        )?;

        // Create removal list
        let remove_machines: Vec<StateMachine> = 
            setup_machines.into_iter().map(|m| m.state_machine).collect();

        // Verify initial state
        assert!(SupportedStateMachines::<T>::iter().count() == n as usize);

        #[extrinsic_call]
        _(RawOrigin::Root, remove_machines);

        // Verify all machines were removed
        assert!(SupportedStateMachines::<T>::iter().count() == 0);
        Ok(())
    }
}

