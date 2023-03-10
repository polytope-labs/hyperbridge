pub enum Error {
    DelayNotElapsed,
    ConsensusStateNotFound,
    StateCommitmentNotFound,
    RequestCommitmentMissing,
    FrozenConsensusClient,
    FrozenStateMachine,
}
