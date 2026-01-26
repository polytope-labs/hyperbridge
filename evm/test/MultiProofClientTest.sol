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
import {MultiProofClient} from "../src/consensus/MultiProofClient.sol";
import {IConsensus, IntermediateState, StateCommitment} from "@hyperbridge/core/interfaces/IConsensus.sol";
import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";

/**
 * @title MultiProofClientTest
 * @notice Comprehensive test suite for the MultiProofClient contract
 * @dev This test suite verifies the multi-proof consensus client's ability to route
 *      consensus proofs to the appropriate verifier (SP1Beefy or BeefyV1) based on the
 *      first byte of the proof.
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
 *    - Tests routing to BeefyV1 (proof type 0x00)
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
 * - MockBeefyV1: Simulates naive proof verification
 */

/// Mock SP1Beefy consensus client for testing
contract MockSP1Beefy is IConsensus, ERC165 {
    bool public wasCalled;
    bytes public lastEncodedState;
    bytes public lastProof;

    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IConsensus).interfaceId || super.supportsInterface(interfaceId);
    }

    function verifyConsensus(bytes memory encodedState, bytes memory proof)
        external
        returns (bytes memory, IntermediateState[] memory)
    {
        wasCalled = true;
        lastEncodedState = encodedState;
        lastProof = proof;

        StateCommitment memory commitment = StateCommitment({
            timestamp: block.timestamp, overlayRoot: keccak256("sp1_overlay"), stateRoot: keccak256("sp1_state")
        });
        IntermediateState memory intermediate =
            IntermediateState({stateMachineId: 1, height: 100, commitment: commitment});

        IntermediateState[] memory intermediates = new IntermediateState[](1);
        intermediates[0] = intermediate;

        return (encodedState, intermediates);
    }

    function reset() external {
        wasCalled = false;
        lastEncodedState = "";
        lastProof = "";
    }
}

/// Mock BeefyV1 consensus client for testing
contract MockBeefyV1 is IConsensus, ERC165 {
    bool public wasCalled;
    bytes public lastEncodedState;
    bytes public lastProof;

    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IConsensus).interfaceId || super.supportsInterface(interfaceId);
    }

    function verifyConsensus(bytes memory encodedState, bytes memory proof)
        external
        returns (bytes memory, IntermediateState[] memory)
    {
        wasCalled = true;
        lastEncodedState = encodedState;
        lastProof = proof;

        StateCommitment memory commitment = StateCommitment({
            timestamp: block.timestamp, overlayRoot: keccak256("beefy_overlay"), stateRoot: keccak256("beefy_state")
        });
        IntermediateState memory intermediate =
            IntermediateState({stateMachineId: 2, height: 200, commitment: commitment});

        IntermediateState[] memory intermediates = new IntermediateState[](1);
        intermediates[0] = intermediate;

        return (encodedState, intermediates);
    }

    function reset() external {
        wasCalled = false;
        lastEncodedState = "";
        lastProof = "";
    }
}

contract MultiProofClientTest is Test {
    MultiProofClient public client;
    MockSP1Beefy public mockSP1Beefy;
    MockBeefyV1 public mockBeefyV1;

    bytes public testEncodedState = hex"deadbeef";
    bytes public testProofData = hex"cafebabe";

    function setUp() public {
        mockSP1Beefy = new MockSP1Beefy();
        mockBeefyV1 = new MockBeefyV1();
        client = new MultiProofClient(IConsensus(address(mockSP1Beefy)), IConsensus(address(mockBeefyV1)));
    }

    /// Test that the client supports the IConsensus interface
    function testSupportsInterface() public view {
        assertTrue(client.supportsInterface(type(IConsensus).interfaceId), "Should support IConsensus interface");
        assertTrue(client.supportsInterface(type(ERC165).interfaceId), "Should support ERC165 interface");
    }

    /// Test that the client correctly stores the SP1Beefy and BeefyV1 addresses
    function testConstructor() public view {
        assertEq(address(client.sp1Beefy()), address(mockSP1Beefy), "SP1Beefy address should match");
        assertEq(address(client.beefyV1()), address(mockBeefyV1), "BeefyV1 address should match");
    }

    /// Test successful routing to BeefyV1 with 0x00 proof type
    function testVerifyConsensusWithBeefyV1Proof() public {
        // Prepare proof with 0x00 prefix (Naive/BeefyV1)
        bytes memory proofWithType = bytes.concat(hex"00", testProofData);

        // Call verifyConsensus
        (bytes memory returnedState, IntermediateState[] memory intermediates) =
            client.verifyConsensus(testEncodedState, proofWithType);

        // Verify BeefyV1 was called
        assertTrue(mockBeefyV1.wasCalled(), "BeefyV1 should have been called");
        assertFalse(mockSP1Beefy.wasCalled(), "SP1Beefy should not have been called");

        // Verify the correct data was passed (without the type byte)
        assertEq(mockBeefyV1.lastEncodedState(), testEncodedState, "Encoded state should match");
        assertEq(mockBeefyV1.lastProof(), testProofData, "Proof data should match (without type byte)");

        // Verify return values
        assertEq(returnedState, testEncodedState, "Returned state should match");
        assertEq(intermediates.length, 1, "Should return one intermediate state");
        assertEq(intermediates[0].stateMachineId, 2, "State machine ID should match BeefyV1 mock");
        assertEq(intermediates[0].height, 200, "Height should match BeefyV1 mock");
    }

    /// Test successful routing to SP1Beefy with 0x01 proof type
    function testVerifyConsensusWithSP1BeefyProof() public {
        // Prepare proof with 0x01 prefix (ZK/SP1Beefy)
        bytes memory proofWithType = bytes.concat(hex"01", testProofData);

        // Call verifyConsensus
        (bytes memory returnedState, IntermediateState[] memory intermediates) =
            client.verifyConsensus(testEncodedState, proofWithType);

        // Verify SP1Beefy was called
        assertTrue(mockSP1Beefy.wasCalled(), "SP1Beefy should have been called");
        assertFalse(mockBeefyV1.wasCalled(), "BeefyV1 should not have been called");

        // Verify the correct data was passed (without the type byte)
        assertEq(mockSP1Beefy.lastEncodedState(), testEncodedState, "Encoded state should match");
        assertEq(mockSP1Beefy.lastProof(), testProofData, "Proof data should match (without type byte)");

        // Verify return values
        assertEq(returnedState, testEncodedState, "Returned state should match");
        assertEq(intermediates.length, 1, "Should return one intermediate state");
        assertEq(intermediates[0].stateMachineId, 1, "State machine ID should match SP1Beefy mock");
        assertEq(intermediates[0].height, 100, "Height should match SP1Beefy mock");
    }

    /// Test that empty proof reverts with EmptyProof error
    function testVerifyConsensusWithEmptyProof() public {
        bytes memory emptyProof = "";

        vm.expectRevert(abi.encodeWithSignature("EmptyProof()"));
        client.verifyConsensus(testEncodedState, emptyProof);
    }

    /// Test that invalid proof type (0x02) reverts with InvalidProofType error
    function testVerifyConsensusWithInvalidProofType() public {
        // Prepare proof with 0x02 prefix (invalid)
        bytes memory proofWithInvalidType = bytes.concat(hex"02", testProofData);

        vm.expectRevert(abi.encodeWithSignature("InvalidProofType(uint8)", uint8(2)));
        client.verifyConsensus(testEncodedState, proofWithInvalidType);
    }

    /// Test that invalid proof type (0xFF) reverts with InvalidProofType error
    function testVerifyConsensusWithInvalidProofTypeMax() public {
        // Prepare proof with 0xFF prefix (invalid)
        bytes memory proofWithInvalidType = bytes.concat(hex"FF", testProofData);

        vm.expectRevert(abi.encodeWithSignature("InvalidProofType(uint8)", uint8(255)));
        client.verifyConsensus(testEncodedState, proofWithInvalidType);
    }

    /// Test that a single-byte proof (only type byte, no actual proof data) works
    function testVerifyConsensusWithOnlyTypeByte() public {
        // Proof with only the type byte (0x00), no actual proof data
        bytes memory proofOnlyType = hex"00";

        // This should pass the type validation and route to BeefyV1
        (bytes memory returnedState,) = client.verifyConsensus(testEncodedState, proofOnlyType);

        // Verify BeefyV1 was called with empty proof data
        assertTrue(mockBeefyV1.wasCalled(), "BeefyV1 should have been called");
        assertEq(mockBeefyV1.lastProof().length, 0, "Proof data should be empty");
        assertEq(returnedState, testEncodedState, "Returned state should match");
    }

    /// Test routing with large proof data
    function testVerifyConsensusWithLargeProof() public {
        // Create a large proof (1KB)
        bytes memory largeProofData = new bytes(1024);
        for (uint256 i = 0; i < 1024; i++) {
            largeProofData[i] = bytes1(uint8(i % 256));
        }

        // Add type byte for SP1Beefy
        bytes memory proofWithType = bytes.concat(hex"01", largeProofData);

        // Call verifyConsensus
        (bytes memory returnedState,) = client.verifyConsensus(testEncodedState, proofWithType);

        // Verify SP1Beefy was called
        assertTrue(mockSP1Beefy.wasCalled(), "SP1Beefy should have been called");
        assertEq(mockSP1Beefy.lastProof(), largeProofData, "Large proof data should match");
        assertEq(returnedState, testEncodedState, "Returned state should match");
    }

    /// Test that proof type enum boundaries are correctly validated
    function testProofTypeEnumBoundaries() public {
        // Test valid boundary: 0x00
        bytes memory proof0 = bytes.concat(hex"00", testProofData);
        client.verifyConsensus(testEncodedState, proof0);
        assertTrue(mockBeefyV1.wasCalled(), "BeefyV1 should be called for type 0x00");

        // Reset mocks
        mockBeefyV1.reset();
        mockSP1Beefy.reset();

        // Test valid boundary: 0x01
        bytes memory proof1 = bytes.concat(hex"01", testProofData);
        client.verifyConsensus(testEncodedState, proof1);
        assertTrue(mockSP1Beefy.wasCalled(), "SP1Beefy should be called for type 0x01");

        // Reset mocks
        mockBeefyV1.reset();
        mockSP1Beefy.reset();

        // Test invalid boundary: 0x02
        bytes memory proof2 = bytes.concat(hex"02", testProofData);
        vm.expectRevert(abi.encodeWithSignature("InvalidProofType(uint8)", uint8(2)));
        client.verifyConsensus(testEncodedState, proof2);
    }

    /// Test that both clients can be called in sequence
    function testMultipleVerifications() public {
        // First call with BeefyV1
        bytes memory beefyProof = bytes.concat(hex"00", testProofData);
        (bytes memory state1, IntermediateState[] memory intermediates1) =
            client.verifyConsensus(testEncodedState, beefyProof);

        assertTrue(mockBeefyV1.wasCalled(), "BeefyV1 should be called first");
        assertEq(intermediates1[0].stateMachineId, 2, "First call should route to BeefyV1");

        // Reset mocks
        mockBeefyV1.reset();
        mockSP1Beefy.reset();

        // Second call with SP1Beefy
        bytes memory sp1Proof = bytes.concat(hex"01", testProofData);
        (bytes memory state2, IntermediateState[] memory intermediates2) =
            client.verifyConsensus(testEncodedState, sp1Proof);

        assertTrue(mockSP1Beefy.wasCalled(), "SP1Beefy should be called second");
        assertEq(intermediates2[0].stateMachineId, 1, "Second call should route to SP1Beefy");

        // Verify both returned the encoded state
        assertEq(state1, testEncodedState, "First call should return correct state");
        assertEq(state2, testEncodedState, "Second call should return correct state");
    }

    /// Fuzz test with random proof types
    function testFuzzProofType(uint8 proofType) public {
        bytes memory proofWithType = bytes.concat(bytes1(proofType), testProofData);

        if (proofType == 0) {
            // Should route to BeefyV1
            client.verifyConsensus(testEncodedState, proofWithType);
            assertTrue(mockBeefyV1.wasCalled(), "Should route to BeefyV1 for type 0");
        } else if (proofType == 1) {
            // Should route to SP1Beefy
            client.verifyConsensus(testEncodedState, proofWithType);
            assertTrue(mockSP1Beefy.wasCalled(), "Should route to SP1Beefy for type 1");
        } else {
            // Should revert with InvalidProofType
            vm.expectRevert(abi.encodeWithSignature("InvalidProofType(uint8)", proofType));
            client.verifyConsensus(testEncodedState, proofWithType);
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
            (bytes memory returnedState,) = client.verifyConsensus(testEncodedState, proofData);
            assertEq(returnedState, testEncodedState, "Should return correct state");
        } else {
            // Invalid proof type - should revert
            vm.expectRevert(abi.encodeWithSignature("InvalidProofType(uint8)", proofType));
            client.verifyConsensus(testEncodedState, proofData);
        }
    }
}
