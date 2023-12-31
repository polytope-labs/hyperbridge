// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "ismp/IConsensusClient.sol";

/// Test consensus client, performs no verification
contract TestConsensusClient is IConsensusClient {
    function verifyConsensus(bytes memory consensusState, bytes memory proof)
        external
        pure
        returns (bytes memory, IntermediateState memory)
    {
        IntermediateState memory intermediate = abi.decode(proof, (IntermediateState));

        return (consensusState, intermediate);
    }
}
