// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.30;

import {Test} from "forge-std/Test.sol";
import {IntermediateState} from "@hyperbridge/core/interfaces/IConsensusV2.sol";

import {BlsBeefy} from "../../src/consensus/BlsBeefy.sol";
import {BeefyConsensusState} from "../../src/consensus/Types.sol";

/**
 * @title The BLS BEEFY consensus client's entry point.
 *
 * @notice `state` and `proof` come from `bls_solidity_proof_fixture` in the Rust verifier. The
 * validators there sign the SCALE encoding of the same commitment this contract hashes, over a
 * single-leaf MMR whose root the commitment carries, so every check in `verify()` runs against
 * data the Rust side produced rather than anything constructed to suit the contract.
 *
 * Needs EIP-2537:
 *   FOUNDRY_PROFILE=bls forge test --match-contract BlsBeefyTest -vv
 */
contract BlsBeefyTest is Test {
    BlsBeefy internal client;

    function setUp() public {
        client = new BlsBeefy();
    }

    function _state() internal view returns (bytes memory) {
        return vm.parseBytes(vm.readFile("tests/foundry/fixtures/bls-beefy-state.hex"));
    }

    function _proof() internal view returns (bytes memory) {
        return vm.parseBytes(vm.readFile("tests/foundry/fixtures/bls-beefy-proof.hex"));
    }

    /// The whole client: authority set, threshold, ordering, merkle membership, aggregate pairing,
    /// MMR leaf inclusion, then the state advances.
    function test_verify_advances_state() public view {
        (bytes memory newStateBytes,, uint256 nextSetId) = client.verify(_state(), _proof());

        BeefyConsensusState memory newState = abi.decode(newStateBytes, (BeefyConsensusState));
        BeefyConsensusState memory oldState = abi.decode(_state(), (BeefyConsensusState));

        assertGt(newState.latestHeight, oldState.latestHeight, "height should advance");
        assertEq(newState.latestHeight, 100, "height should match the proven commitment");
        assertGt(nextSetId, 0, "should report the next authority set id");
    }

    /// Replaying a proof the state has already passed is a no-op rather than a revert, so callers
    /// do not have to guard against it.
    function test_stale_proof_is_a_noop() public view {
        (bytes memory advanced,,) = client.verify(_state(), _proof());

        (bytes memory again, IntermediateState[] memory intermediates,) = client.verify(advanced, _proof());

        assertEq(again, advanced, "state should be unchanged");
        assertEq(intermediates.length, 0, "a stale proof finalizes nothing");
    }

    function _liveState() internal view returns (bytes memory) {
        return vm.parseBytes(vm.readFile("tests/foundry/fixtures/bls-beefy-live-state.hex"));
    }

    function _liveProof() internal view returns (bytes memory) {
        return vm.parseBytes(vm.readFile("tests/foundry/fixtures/bls-beefy-live-proof.hex"));
    }

    /// The real thing: a proof built by the prover from a running BLS relay, carrying a genuine
    /// parachain header and an MMR proof with actual depth, rather than a synthetic single leaf.
    /// The keys arrive uncompressed and the client compresses them to rebuild the merkle leaves,
    /// so the whole wire format is exercised too.
    function test_verify_live_proof() public view {
        (bytes memory newStateBytes, IntermediateState[] memory intermediates,) =
            client.verify(_liveState(), _liveProof());

        BeefyConsensusState memory newState = abi.decode(newStateBytes, (BeefyConsensusState));
        BeefyConsensusState memory oldState = abi.decode(_liveState(), (BeefyConsensusState));

        assertGt(newState.latestHeight, oldState.latestHeight, "height should advance");
        assertEq(intermediates.length, 1, "should finalize the registered parachain");
        assertEq(intermediates[0].stateMachineId, 4009, "should be para 4009");
        assertGt(intermediates[0].height, 0, "parachain height should be non-zero");
    }
}
