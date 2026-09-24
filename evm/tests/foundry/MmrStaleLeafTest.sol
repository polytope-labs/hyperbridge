// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Test.sol";
import {MerkleMountainRange} from "@polytope-labs/solidity-merkle-trees/src/MerkleMountainRange.sol";

/// @dev Regression coverage for HYPERBR-2497 — the malleability that `EcdsaBeefy`'s
/// `StaleMmrLeaf` pin (`parentNumber + 1 == commitment.blockNumber`) now neutralises.
///
/// A BEEFY mmr leaf carries `parentNumber` (bound into the leaf hash, and the sole input to the
/// mmr `leafCount`) and a separate `leafIndex` field that is NOT bound to the hash. Because
/// `MerkleMountainRange` bags peaks with `keccak(smaller || larger)` — the same keccak, with no
/// domain tag, that internal nodes use as `keccak(left || right)` — a genuine *older* leaf
/// presented with its own honest (shorter) leaf count at a *fabricated* index re-assembles
/// genuine tree nodes into the current, larger signed root. Downstream that would advance the
/// trusted height while dropping the authority rotation carried by the true latest leaf.
///
/// This test pins the underlying library behaviour that makes the `EcdsaBeefy` guard necessary:
/// the honest latest-leaf proof verifies, the forged stale-leaf-at-fabricated-index proof also
/// verifies against the same root, and a naive stale submission (correct index) does not.
contract MmrStaleLeafTest is Test {
    // Real size-12 MMR: peaks over leaf blocks [0,8) and [8,12).
    bytes32[12] internal L;

    function setUp() public {
        for (uint256 i; i < 12; ++i) L[i] = keccak256(abi.encodePacked("leaf", uint64(i)));
    }

    function _leaf(uint256 idx, bytes32 hash) internal pure returns (MerkleMountainRange.Leaf[] memory a) {
        a = new MerkleMountainRange.Leaf[](1);
        a[0] = MerkleMountainRange.Leaf(idx, hash);
    }

    // Root of the perfect 8-leaf subtree over leaves [s, s+8): keccak(left || right) throughout.
    function _peak8(uint256 s) internal view returns (bytes32) {
        bytes32 a = keccak256(
            abi.encodePacked(
                keccak256(abi.encodePacked(L[s], L[s + 1])), keccak256(abi.encodePacked(L[s + 2], L[s + 3]))
            )
        );
        bytes32 b = keccak256(
            abi.encodePacked(
                keccak256(abi.encodePacked(L[s + 4], L[s + 5])), keccak256(abi.encodePacked(L[s + 6], L[s + 7]))
            )
        );
        return keccak256(abi.encodePacked(a, b));
    }

    function testStaleLeafReassemblesSignedRoot() public view {
        // Ground-truth signed root of the size-12 tree: all leaves, empty proof.
        MerkleMountainRange.Leaf[] memory all = new MerkleMountainRange.Leaf[](12);
        for (uint256 i; i < 12; ++i) all[i] = MerkleMountainRange.Leaf(i, L[i]);
        bytes32 root = MerkleMountainRange.CalculateRoot(new bytes32[](0), all, 12);

        bytes32 peak1 = _peak8(0); // genuine peak over leaves 0..7
        bytes32 node89 = keccak256(abi.encodePacked(L[8], L[9])); // genuine internal node of peak2

        // Honest path: the true latest leaf (#11) at index 11, honest leafCount 12, verifies.
        bytes32[] memory honest = new bytes32[](3);
        honest[0] = peak1;
        honest[1] = L[10];
        honest[2] = node89;
        assertTrue(
            MerkleMountainRange.VerifyProof(root, honest, _leaf(11, L[11]), 12), "honest latest leaf must verify"
        );

        // Forgery: genuine OLDER leaf (#10) at a FABRICATED index (8), with its own honest
        // leafCount (11) and a proof of genuine tree nodes, reproduces the same signed root.
        bytes32[] memory forged = new bytes32[](3);
        forged[0] = peak1;
        forged[1] = L[11];
        forged[2] = node89;
        assertTrue(
            MerkleMountainRange.VerifyProof(root, forged, _leaf(8, L[10]), 11),
            "stale leaf at fabricated index reassembles the signed root"
        );

        // Control: the same older leaf at its TRUE index (10) — a naive stale submission — does not.
        assertFalse(_verifyQuiet(root, forged, _leaf(10, L[10]), 11), "naive stale submission must not verify");
    }

    // VerifyProof reverts (rather than returning false) on some malformed shapes; swallow it.
    function _verifyQuiet(bytes32 root, bytes32[] memory proof, MerkleMountainRange.Leaf[] memory leaves, uint256 c)
        internal
        view
        returns (bool ok)
    {
        try this.verifyExternal(root, proof, leaves, c) returns (bool r) {
            ok = r;
        } catch {
            ok = false;
        }
    }

    function verifyExternal(bytes32 root, bytes32[] memory proof, MerkleMountainRange.Leaf[] memory leaves, uint256 c)
        external
        pure
        returns (bool)
    {
        return MerkleMountainRange.VerifyProof(root, proof, leaves, c);
    }
}
