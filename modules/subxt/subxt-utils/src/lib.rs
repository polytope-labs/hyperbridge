pub mod gargantua;

mod gargantua_conversion {
    use ismp::{
        consensus::{StateCommitment, StateMachineHeight, StateMachineId},
        host::{Ethereum, StateMachine},
    };

    impl From<crate::gargantua::api::runtime_types::ismp::consensus::StateCommitment>
        for StateCommitment
    {
        fn from(
            commitment: crate::gargantua::api::runtime_types::ismp::consensus::StateCommitment,
        ) -> Self {
            StateCommitment {
                timestamp: commitment.timestamp,
                overlay_root: commitment.overlay_root,
                state_root: commitment.state_root,
            }
        }
    }

    impl From<crate::gargantua::api::runtime_types::ismp::consensus::StateMachineHeight>
        for StateMachineHeight
    {
        fn from(
            state_machine_height: crate::gargantua::api::runtime_types::ismp::consensus::StateMachineHeight,
        ) -> Self {
            StateMachineHeight {
                id: state_machine_height.id.into(),
                height: state_machine_height.height,
            }
        }
    }

    impl From<crate::gargantua::api::runtime_types::ismp::consensus::StateMachineId>
        for StateMachineId
    {
        fn from(
            state_machine_id: crate::gargantua::api::runtime_types::ismp::consensus::StateMachineId,
        ) -> Self {
            StateMachineId {
                state_id: state_machine_id.state_id.into(),
                consensus_state_id: state_machine_id.consensus_state_id,
            }
        }
    }

    impl From<crate::gargantua::api::runtime_types::ismp::host::StateMachine> for StateMachine {
        fn from(
            state_machine_id: crate::gargantua::api::runtime_types::ismp::host::StateMachine,
        ) -> Self {
            match state_machine_id {
                crate::gargantua::api::runtime_types::ismp::host::StateMachine::Ethereum(
                    ethereum,
                ) => match ethereum {
                    crate::gargantua::api::runtime_types::ismp::host::Ethereum::ExecutionLayer =>
                        StateMachine::Ethereum(Ethereum::ExecutionLayer),
                    crate::gargantua::api::runtime_types::ismp::host::Ethereum::Optimism =>
                        StateMachine::Ethereum(Ethereum::Optimism),
                    crate::gargantua::api::runtime_types::ismp::host::Ethereum::Arbitrum =>
                        StateMachine::Ethereum(Ethereum::Arbitrum),
                    crate::gargantua::api::runtime_types::ismp::host::Ethereum::Base =>
                        StateMachine::Ethereum(Ethereum::Base),
                },
                crate::gargantua::api::runtime_types::ismp::host::StateMachine::Polkadot(id) =>
                    StateMachine::Polkadot(id),
                crate::gargantua::api::runtime_types::ismp::host::StateMachine::Kusama(id) =>
                    StateMachine::Kusama(id),
                crate::gargantua::api::runtime_types::ismp::host::StateMachine::Grandpa(
                    consensus_state_id,
                ) => StateMachine::Grandpa(consensus_state_id),
                crate::gargantua::api::runtime_types::ismp::host::StateMachine::Beefy(
                    consensus_state_id,
                ) => StateMachine::Beefy(consensus_state_id),
                crate::gargantua::api::runtime_types::ismp::host::StateMachine::Polygon =>
                    StateMachine::Polygon,
                crate::gargantua::api::runtime_types::ismp::host::StateMachine::Bsc =>
                    StateMachine::Bsc,
            }
        }
    }

    impl From<StateMachineHeight>
        for crate::gargantua::api::runtime_types::ismp::consensus::StateMachineHeight
    {
        fn from(state_machine_height: StateMachineHeight) -> Self {
            crate::gargantua::api::runtime_types::ismp::consensus::StateMachineHeight {
                id: state_machine_height.id.into(),
                height: state_machine_height.height,
            }
        }
    }

    impl From<StateMachineId>
        for crate::gargantua::api::runtime_types::ismp::consensus::StateMachineId
    {
        fn from(state_machine_id: StateMachineId) -> Self {
            Self {
                state_id: state_machine_id.state_id.into(),
                consensus_state_id: state_machine_id.consensus_state_id,
            }
        }
    }

    impl From<StateMachine> for crate::gargantua::api::runtime_types::ismp::host::StateMachine {
        fn from(state_machine_id: StateMachine) -> Self {
            match state_machine_id {
                StateMachine::Ethereum(ethereum) => match ethereum {
                    Ethereum::ExecutionLayer =>
                        crate::gargantua::api::runtime_types::ismp::host::StateMachine::Ethereum(
                            crate::gargantua::api::runtime_types::ismp::host::Ethereum::ExecutionLayer,
                        ),
                    Ethereum::Optimism =>
                        crate::gargantua::api::runtime_types::ismp::host::StateMachine::Ethereum(
                            crate::gargantua::api::runtime_types::ismp::host::Ethereum::Optimism,
                        ),
                    Ethereum::Arbitrum =>
                        crate::gargantua::api::runtime_types::ismp::host::StateMachine::Ethereum(
                            crate::gargantua::api::runtime_types::ismp::host::Ethereum::Arbitrum,
                        ),
                    Ethereum::Base => crate::gargantua::api::runtime_types::ismp::host::StateMachine::Ethereum(
                        crate::gargantua::api::runtime_types::ismp::host::Ethereum::Base,
                    ),
                },
                StateMachine::Polkadot(id) =>
                    crate::gargantua::api::runtime_types::ismp::host::StateMachine::Polkadot(id),
                StateMachine::Kusama(id) =>
                    crate::gargantua::api::runtime_types::ismp::host::StateMachine::Kusama(id),
                StateMachine::Grandpa(consensus_state_id) =>
                    crate::gargantua::api::runtime_types::ismp::host::StateMachine::Grandpa(consensus_state_id),
                StateMachine::Beefy(consensus_state_id) =>
                    crate::gargantua::api::runtime_types::ismp::host::StateMachine::Beefy(consensus_state_id),

                StateMachine::Polygon => crate::gargantua::api::runtime_types::ismp::host::StateMachine::Polygon,
                StateMachine::Bsc => crate::gargantua::api::runtime_types::ismp::host::StateMachine::Bsc,
            }
        }
    }
}
