// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

import "forge-std/Test.sol";

import {HyperbridgeLzEndpoint} from "../contracts/HyperbridgeLzEndpoint.sol";

import {PostRequest} from "@hyperbridge/core/libraries/Message.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";

/// @dev Stub host: HyperbridgeLzEndpoint.setHost queries feeToken().decimals(), so the
/// mock host returns a stub feeToken whose `decimals()` returns 18. No other methods
/// are needed for the onAccept-path tests.
contract MockFeeToken {
    function decimals() external pure returns (uint8) {
        return 18;
    }
}

contract MockHost {
    address internal _feeToken;
    constructor() {
        _feeToken = address(new MockFeeToken());
    }
    function feeToken() external view returns (address) {
        return _feeToken;
    }
}

/// @dev Fork-free unit tests for `setEidMapping` cleanup semantics and the
/// `onAccept` source-eid gate against the `srcEid = 0` collision.
contract HyperbridgeLzEndpointConfigTest is Test {
    HyperbridgeLzEndpoint internal endpoint;
    MockHost internal mockHost;

    uint32 internal constant LOCAL_EID = 200;
    uint32 internal constant EID_A = 101;
    uint32 internal constant EID_B = 102;

    bytes internal constant SM_A = hex"01020304";
    bytes internal constant SM_B = hex"05060708";

    function setUp() public {
        endpoint = new HyperbridgeLzEndpoint(address(this));
        mockHost = new MockHost();
        endpoint.setHost(address(mockHost), LOCAL_EID);
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

    function testRejectZeroEid() public {
        vm.expectRevert(HyperbridgeLzEndpoint.InvalidEid.selector);
        endpoint.setEidMapping(0, SM_A);

        vm.expectRevert(HyperbridgeLzEndpoint.InvalidEid.selector);
        endpoint.setEidMapping(0, hex"");
    }

    /// @dev Attacker submits an ISMP message from an UNREGISTERED source (the
    /// zero-default reverse-map regime) with `srcEid = 0` in the body. The pre-fix
    /// gate `expectedEid != srcEid` collapses to `0 != 0`, admitting the forgery.
    /// The fixed gate explicitly rejects `expectedEid == 0`.
    function testOnAcceptRejectsSrcEidZeroFromUnregisteredSource() public {
        bytes memory unregisteredSource = SM_A; // never passed to setEidMapping

        bytes memory body = abi.encode(
            bytes32(0),       // guid
            uint32(0),        // srcEid — the collision value
            bytes32(uint256(uint160(address(0xBEEF)))), // sender
            uint64(1),        // nonce
            bytes32(uint256(uint160(address(0xC0DE)))), // receiver
            bytes("")          // message
        );

        PostRequest memory request = PostRequest({
            source: unregisteredSource,
            dest: hex"00",
            nonce: 0,
            from: abi.encodePacked(address(endpoint)), // CREATE2 self-check passes
            to: abi.encodePacked(address(endpoint)),
            timeoutTimestamp: 0,
            body: body
        });

        vm.prank(address(mockHost));
        vm.expectRevert(HyperbridgeLzEndpoint.UnknownSource.selector);
        endpoint.onAccept(IncomingPostRequest({request: request, relayer: address(0)}));
    }

    /// @dev Same scenario but the source was previously registered and then disabled
    /// via `setEidMapping(eid, "")` — reverse map is now zero. The gate must still
    /// reject `srcEid = 0` rather than collide with the cleared mapping.
    function testOnAcceptRejectsSrcEidZeroFromDisabledSource() public {
        endpoint.setEidMapping(EID_A, SM_A);
        endpoint.setEidMapping(EID_A, hex""); // disable; clears reverse for SM_A
        assertEq(endpoint.eidFor(SM_A), 0, "precondition: reverse cleared");

        bytes memory body = abi.encode(
            bytes32(0),
            uint32(0),
            bytes32(uint256(uint160(address(0xBEEF)))),
            uint64(1),
            bytes32(uint256(uint160(address(0xC0DE)))),
            bytes("")
        );

        PostRequest memory request = PostRequest({
            source: SM_A,
            dest: hex"00",
            nonce: 0,
            from: abi.encodePacked(address(endpoint)),
            to: abi.encodePacked(address(endpoint)),
            timeoutTimestamp: 0,
            body: body
        });

        vm.prank(address(mockHost));
        vm.expectRevert(HyperbridgeLzEndpoint.UnknownSource.selector);
        endpoint.onAccept(IncomingPostRequest({request: request, relayer: address(0)}));
    }
}
