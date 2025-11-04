interface IHyperbridgeVerifier {
    struct StateCommitment {
        uint64 timestamp;
        bytes32 overlayRoot;
        bytes32 stateRoot;
    }

    struct StateCommitmentHeight {
        StateCommitment commitment;
        uint64 height;
    }

    function latestStateCommitment() external view returns (StateCommitmentHeight memory);
}
