// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import {Test} from "forge-std/Test.sol";
import {ApkDigest, Header, Digest, DigestItem, HeaderImpl} from "../../src/consensus/Types.sol";

/// The client side of `pallet-beefy-apk-digest`: reading the APK commitment out of a hyperbridge
/// header that a verifier has already authenticated through the BEEFY MMR's parachain heads root.
///
/// The payload bytes here are the SCALE encoding the pallet produces, a little-endian u64 set id
/// followed by the 32 byte commitment. Both sides are pinned: `digest_payload_round_trips` in the
/// pallet asserts the same 8 + 32 layout.
contract ApkCommitmentDigestTest is Test {
    using HeaderImpl for Header;

    bytes constant PAYLOAD_577 =
        hex"4102000000000000020000000303030303030303030303030303030303030303030303030303030303030303";

    function _header(Digest[] memory digests) internal pure returns (Header memory) {
        return Header({
            parentHash: bytes32(0),
            number: 1,
            stateRoot: bytes32(0),
            extrinsicRoot: bytes32(0),
            digests: digests
        });
    }

    function _consensus(bytes4 id, bytes memory data) internal pure returns (Digest memory d) {
        d.isConsensus = true;
        d.consensus = DigestItem({consensusId: id, data: data});
    }

    function _preRuntime(bytes4 id) internal pure returns (Digest memory d) {
        d.isPreRuntime = true;
        d.preruntime = DigestItem({consensusId: id, data: hex"0102"});
    }

    function test_reads_the_commitment_the_pallet_wrote() public view {
        Digest[] memory digests = new Digest[](1);
        digests[0] = _consensus(HeaderImpl.APK_COMMITMENT_ID, PAYLOAD_577);

        ApkDigest memory digest = _header(digests).apkCommitment();
        assertEq(digest.setId, 577, "set id decoded wrong; check little-endian");
        assertEq(digest.len, 2, "set size decoded wrong");
        assertEq(digest.commitment, uint256(0x0303030303030303030303030303030303030303030303030303030303030303));
    }

    /// The normal case: a header carrying aura's items alongside ours.
    function test_finds_it_among_other_digests() public view {
        Digest[] memory digests = new Digest[](3);
        digests[0] = _preRuntime(bytes4("aura"));
        digests[1] = _consensus(HeaderImpl.APK_COMMITMENT_ID, PAYLOAD_577);
        digests[2] = _preRuntime(bytes4("aura"));

        ApkDigest memory digest = _header(digests).apkCommitment();
        assertEq(digest.setId, 577);
    }

    /// Most headers do not complete a set, and that must read as absent rather than revert.
    function test_absent_on_a_header_without_one() public view {
        Digest[] memory digests = new Digest[](1);
        digests[0] = _preRuntime(bytes4("aura"));

        ApkDigest memory digest = _header(digests).apkCommitment();
        assertEq(digest.setId, 0, "no digest, so the set id stays zero");
        assertEq(digest.commitment, 0);
    }

    /// Another engine's consensus item must not be mistaken for ours. Same digest variant, so the
    /// engine id is the only thing separating them.
    function test_ignores_another_engines_consensus_item() public view {
        Digest[] memory digests = new Digest[](1);
        digests[0] = _consensus(bytes4("ISMP"), PAYLOAD_577);

        assertEq(_header(digests).apkCommitment().setId, 0, "an ISMP digest was read as an APK commitment");
    }

    /// A wrong-length payload under our engine id is skipped rather than decoded into garbage.
    function test_malformed_payload_reads_as_absent() public view {
        Digest[] memory digests = new Digest[](1);
        digests[0] = _consensus(HeaderImpl.APK_COMMITMENT_ID, hex"4102000000000000");

        assertEq(_header(digests).apkCommitment().setId, 0, "a truncated payload should not decode");
    }
}
