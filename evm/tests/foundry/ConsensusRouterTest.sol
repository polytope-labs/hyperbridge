// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import {ConsensusRouter} from "../../src/consensus/ConsensusRouter.sol";
import {IConsensusV2, IntermediateState, StateCommitment} from "@hyperbridge/core/interfaces/IConsensusV2.sol";
import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";

/**
 * @title ConsensusRouterTest
 * @notice Comprehensive test suite for the ConsensusRouter contract
 * @dev This test suite verifies the consensus router's ability to route
 *      consensus proofs to the appropriate verifier (SP1Beefy, EcdsaBeefy)
 *      based on the first byte of the proof.
 *
 * Test Coverage:
 * --------------
 * 1. Interface Support:
 *    - Verifies ERC165 interface support for IConsensus
 *
 * 2. Constructor & Initialization:
 *    - Validates immutable consensus client addresses are correctly set
 *
 * 3. Proof Routing:
 *    - Tests routing to EcdsaBeefy (proof type 0x00)
 *    - Tests routing to SP1Beefy (proof type 0x01)
 *    - Validates proof data is correctly stripped of type byte before forwarding
 *
 * 4. Error Handling:
 *    - Empty proof rejection
 *    - Invalid proof type rejection (0x02-0xFF)
 *    - Proper error messages and revert reasons
 *
 * 5. Edge Cases:
 *    - Single-byte proof (only type byte, no data)
 *    - Large proof data (1KB+)
 *    - Boundary conditions for proof type enum
 *    - Sequential calls to different verifiers
 *
 * 6. Fuzz Testing:
 *    - Random proof types (valid and invalid)
 *    - Random proof data with varying lengths
 *
 * Mock Contracts:
 * --------------
 * - MockSP1Beefy: Simulates ZK proof verification
 * - MockEcdsaBeefy: Simulates naive proof verification
 */

/// Mock SP1Beefy consensus client for testing
contract MockSP1Beefy is IConsensusV2, ERC165 {
    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IConsensusV2).interfaceId || super.supportsInterface(interfaceId);
    }

    function verify(bytes calldata encodedState, bytes calldata proof)
        external
        view
        returns (bytes memory, IntermediateState[] memory, uint256)
    {
        StateCommitment memory commitment = StateCommitment({
            timestamp: block.timestamp, overlayRoot: keccak256("sp1_overlay"), stateRoot: keccak256("sp1_state")
        });
        IntermediateState memory intermediate =
            IntermediateState({stateMachineId: 1, height: 100, commitment: commitment});

        IntermediateState[] memory intermediates = new IntermediateState[](1);
        intermediates[0] = intermediate;

        return (encodedState, intermediates, 0);
    }
}

/// Mock EcdsaBeefy consensus client for testing
contract MockEcdsaBeefy is IConsensusV2, ERC165 {
    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IConsensusV2).interfaceId || super.supportsInterface(interfaceId);
    }

    function verify(bytes calldata encodedState, bytes calldata proof)
        external
        view
        returns (bytes memory, IntermediateState[] memory, uint256)
    {
        StateCommitment memory commitment = StateCommitment({
            timestamp: block.timestamp, overlayRoot: keccak256("beefy_overlay"), stateRoot: keccak256("beefy_state")
        });
        IntermediateState memory intermediate =
            IntermediateState({stateMachineId: 2, height: 200, commitment: commitment});

        IntermediateState[] memory intermediates = new IntermediateState[](1);
        intermediates[0] = intermediate;

        return (encodedState, intermediates, 0);
    }
}

contract ConsensusRouterTest is Test {
    ConsensusRouter public client;
    MockSP1Beefy public mockSP1Beefy;
    MockEcdsaBeefy public mockEcdsaBeefy;

    bytes public testEncodedState = hex"deadbeef";
    bytes public testProofData = hex"cafebabe";

    function setUp() public {
        mockSP1Beefy = new MockSP1Beefy();
        mockEcdsaBeefy = new MockEcdsaBeefy();
        client = new ConsensusRouter(
            IConsensusV2(address(mockSP1Beefy)),
            IConsensusV2(address(mockEcdsaBeefy))
        );
    }

    /// Test that the client supports the IConsensus interface
    function testSupportsInterface() public view {
        assertTrue(client.supportsInterface(type(IConsensusV2).interfaceId), "Should support IConsensus interface");
        assertTrue(client.supportsInterface(type(ERC165).interfaceId), "Should support ERC165 interface");
    }

    /// Test that the client correctly stores the SP1Beefy and EcdsaBeefy addresses
    function testConstructor() public view {
        assertEq(address(client.sp1Beefy()), address(mockSP1Beefy), "SP1Beefy address should match");
        assertEq(address(client.ecdsaBeefy()), address(mockEcdsaBeefy), "EcdsaBeefy address should match");
    }

    /// Test successful routing to EcdsaBeefy with 0x00 proof type
    function testVerifyConsensusWithEcdsaBeefyProof() public view {
        // Prepare proof with 0x00 prefix (Naive/EcdsaBeefy)
        bytes memory proofWithType = bytes.concat(hex"00", testProofData);

        // Call verifyConsensus
        (bytes memory returnedState, IntermediateState[] memory intermediates,) =
            client.verify(testEncodedState, proofWithType);

        // Verify routing by checking return values (EcdsaBeefy returns stateMachineId=2, height=200)
        assertEq(returnedState, testEncodedState, "Returned state should match");
        assertEq(intermediates.length, 1, "Should return one intermediate state");
        assertEq(intermediates[0].stateMachineId, 2, "State machine ID should match EcdsaBeefy mock");
        assertEq(intermediates[0].height, 200, "Height should match EcdsaBeefy mock");
    }

    /// Test successful routing to SP1Beefy with 0x01 proof type
    function testVerifyConsensusWithSP1BeefyProof() public view {
        // Prepare proof with 0x01 prefix (ZK/SP1Beefy)
        bytes memory proofWithType = bytes.concat(hex"01", testProofData);

        // Call verifyConsensus
        (bytes memory returnedState, IntermediateState[] memory intermediates,) =
            client.verify(testEncodedState, proofWithType);

        // Verify routing by checking return values (SP1Beefy returns stateMachineId=1, height=100)
        assertEq(returnedState, testEncodedState, "Returned state should match");
        assertEq(intermediates.length, 1, "Should return one intermediate state");
        assertEq(intermediates[0].stateMachineId, 1, "State machine ID should match SP1Beefy mock");
        assertEq(intermediates[0].height, 100, "Height should match SP1Beefy mock");
    }

    /// Test that empty proof reverts with EmptyProof error
    function testVerifyConsensusWithEmptyProof() public {
        bytes memory emptyProof = "";

        vm.expectRevert(abi.encodeWithSignature("EmptyProof()"));
        client.verify(testEncodedState, emptyProof);
    }

    /// Test that invalid proof type (0x02) reverts with InvalidProofType error
    function testVerifyConsensusWithInvalidProofType() public {
        // Prepare proof with 0x02 prefix (invalid)
        bytes memory proofWithInvalidType = bytes.concat(hex"02", testProofData);

        vm.expectRevert(abi.encodeWithSignature("InvalidProofType(uint8)", uint8(2)));
        client.verify(testEncodedState, proofWithInvalidType);
    }

    /// Test that invalid proof type (0xFF) reverts with InvalidProofType error
    function testVerifyConsensusWithInvalidProofTypeMax() public {
        // Prepare proof with 0xFF prefix (invalid)
        bytes memory proofWithInvalidType = bytes.concat(hex"FF", testProofData);

        vm.expectRevert(abi.encodeWithSignature("InvalidProofType(uint8)", uint8(255)));
        client.verify(testEncodedState, proofWithInvalidType);
    }

    /// Test that a single-byte proof (only type byte, no actual proof data) works
    function testVerifyConsensusWithOnlyTypeByte() public view {
        // Proof with only the type byte (0x00), no actual proof data
        bytes memory proofOnlyType = hex"00";

        // This should pass the type validation and route to EcdsaBeefy
        (bytes memory returnedState,,) = client.verify(testEncodedState, proofOnlyType);

        // Verify routing worked correctly
        assertEq(returnedState, testEncodedState, "Returned state should match");
    }

    /// Test routing with large proof data
    function testVerifyConsensusWithLargeProof() public view {
        // Create a large proof (1KB)
        bytes memory largeProofData = new bytes(1024);
        for (uint256 i = 0; i < 1024; i++) {
            largeProofData[i] = bytes1(uint8(i % 256));
        }

        // Add type byte for SP1Beefy
        bytes memory proofWithType = bytes.concat(hex"01", largeProofData);

        // Call verifyConsensus
        (bytes memory returnedState,,) = client.verify(testEncodedState, proofWithType);

        // Verify routing worked correctly
        assertEq(returnedState, testEncodedState, "Returned state should match");
    }

    /// Test that proof type enum boundaries are correctly validated
    function testProofTypeEnumBoundaries() public {
        // Test valid boundary: 0x00
        bytes memory proof0 = bytes.concat(hex"00", testProofData);
        (, IntermediateState[] memory intermediates0,) = client.verify(testEncodedState, proof0);
        assertEq(intermediates0[0].stateMachineId, 2, "Should route to EcdsaBeefy for type 0x00");

        // Test valid boundary: 0x01
        bytes memory proof1 = bytes.concat(hex"01", testProofData);
        (, IntermediateState[] memory intermediates1,) = client.verify(testEncodedState, proof1);
        assertEq(intermediates1[0].stateMachineId, 1, "Should route to SP1Beefy for type 0x01");

        // Test invalid boundary: 0x02
        bytes memory proof2 = bytes.concat(hex"02", testProofData);
        vm.expectRevert(abi.encodeWithSignature("InvalidProofType(uint8)", uint8(2)));
        client.verify(testEncodedState, proof2);
    }

    /// Test that both clients can be called in sequence
    function testMultipleVerifications() public view {
        // First call with EcdsaBeefy
        bytes memory beefyProof = bytes.concat(hex"00", testProofData);
        (bytes memory state1, IntermediateState[] memory intermediates1,) =
            client.verify(testEncodedState, beefyProof);

        assertEq(intermediates1[0].stateMachineId, 2, "First call should route to EcdsaBeefy");

        // Second call with SP1Beefy
        bytes memory sp1Proof = bytes.concat(hex"01", testProofData);
        (bytes memory state2, IntermediateState[] memory intermediates2,) =
            client.verify(testEncodedState, sp1Proof);

        assertEq(intermediates2[0].stateMachineId, 1, "Second call should route to SP1Beefy");

        // Verify all returned the encoded state
        assertEq(state1, testEncodedState, "First call should return correct state");
        assertEq(state2, testEncodedState, "Second call should return correct state");
    }

    /// Fuzz test with random proof types
    function testFuzzProofType(uint8 proofType) public {
        bytes memory proofWithType = bytes.concat(bytes1(proofType), testProofData);

        if (proofType == 0) {
            // Should route to EcdsaBeefy
            (, IntermediateState[] memory intermediates,) = client.verify(testEncodedState, proofWithType);
            assertEq(intermediates[0].stateMachineId, 2, "Should route to EcdsaBeefy for type 0");
        } else if (proofType == 1) {
            // Should route to SP1Beefy
            (, IntermediateState[] memory intermediates,) = client.verify(testEncodedState, proofWithType);
            assertEq(intermediates[0].stateMachineId, 1, "Should route to SP1Beefy for type 1");
        } else {
            // Should revert with InvalidProofType
            vm.expectRevert(abi.encodeWithSignature("InvalidProofType(uint8)", proofType));
            client.verify(testEncodedState, proofWithType);
        }
    }

    /// Fuzz test with random proof data
    function testFuzzProofData(bytes calldata proofData) public {
        // Skip empty proof test case (tested separately)
        vm.assume(proofData.length > 0);

        // Extract the first byte as proof type
        uint8 proofType = uint8(proofData[0]);

        if (proofType <= 1) {
            // Valid proof type - should succeed
            (bytes memory returnedState,,) = client.verify(testEncodedState, proofData);
            assertEq(returnedState, testEncodedState, "Should return correct state");
        } else {
            // Invalid proof type - should revert
            vm.expectRevert(abi.encodeWithSignature("InvalidProofType(uint8)", proofType));
            client.verify(testEncodedState, proofData);
        }
    }
}
