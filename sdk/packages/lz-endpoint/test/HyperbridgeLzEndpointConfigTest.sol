// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

import "forge-std/Test.sol";

import {HyperbridgeLzEndpoint} from "../contracts/HyperbridgeLzEndpoint.sol";

/// @dev Fork-free unit tests for `setEidMapping` cleanup semantics (HYPERBR-427).
contract HyperbridgeLzEndpointConfigTest is Test {
    HyperbridgeLzEndpoint internal endpoint;

    uint32 internal constant EID_A = 101;
    uint32 internal constant EID_B = 102;

    bytes internal constant SM_A = hex"01020304";
    bytes internal constant SM_B = hex"05060708";

    function setUp() public {
        endpoint = new HyperbridgeLzEndpoint(address(this));
    }

    function testRemapClearsPriorReverseEntry() public {
        endpoint.setEidMapping(EID_A, SM_A);
        assertEq(endpoint.eidFor(SM_A), EID_A);

        endpoint.setEidMapping(EID_A, SM_B);

        assertEq(endpoint.eidFor(SM_A), 0, "stale reverse entry must be cleared");
        assertEq(endpoint.eidFor(SM_B), EID_A, "new reverse entry must be set");
        assertEq(keccak256(endpoint.eidMapping(EID_A)), keccak256(SM_B));
    }

    function testDisableClearsReverseAndAvoidsEmptyKeyResidue() public {
        endpoint.setEidMapping(EID_A, SM_A);
        endpoint.setEidMapping(EID_A, hex"");

        assertEq(endpoint.eidFor(SM_A), 0, "old reverse entry must be cleared on disable");
        assertEq(endpoint.eidFor(hex""), 0, "empty-bytes reverse key must not be written");
        assertEq(endpoint.eidMapping(EID_A).length, 0, "forward mapping must be empty");
        assertFalse(endpoint.isSupportedEid(EID_A), "EID must report unsupported after disable");
    }

    function testSetEidMappingIsIdempotent() public {
        endpoint.setEidMapping(EID_A, SM_A);
        endpoint.setEidMapping(EID_A, SM_A);

        assertEq(endpoint.eidFor(SM_A), EID_A);
        assertEq(keccak256(endpoint.eidMapping(EID_A)), keccak256(SM_A));
        assertTrue(endpoint.isSupportedEid(EID_A));
    }

    function testReassignStateMachineAcrossEidsPointsReverseToLatest() public {
        endpoint.setEidMapping(EID_A, SM_A);
        endpoint.setEidMapping(EID_B, SM_A);

        assertEq(endpoint.eidFor(SM_A), EID_B, "reverse must point to the latest EID");
        assertEq(keccak256(endpoint.eidMapping(EID_A)), keccak256(SM_A));
        assertEq(keccak256(endpoint.eidMapping(EID_B)), keccak256(SM_A));
    }
}
