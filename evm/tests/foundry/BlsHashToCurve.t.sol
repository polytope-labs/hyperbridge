// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.30;

import {Test} from "forge-std/Test.sol";
import {BlsHashToCurve} from "../../src/consensus/BlsHashToCurve.sol";

/**
 * @title Cross-language check of BEEFY's hash-to-curve.
 *
 * @notice The expected values come from `bls_hash_to_curve_vector` in the Rust verifier, which
 * generates them through the very `w3f-bls` code path the relay chain signs with. If these two
 * disagree, an on-chain aggregate BLS check would fail against real chain output while looking
 * perfectly correct in isolation.
 *
 * Needs EIP-2537, so run under an EVM version of at least Prague:
 *   FOUNDRY_PROFILE=bls forge test --match-contract BlsHashToCurveTest -vv
 */
contract BlsHashToCurveTest is Test {
    /// The message the Rust vector was generated for.
    bytes constant MESSAGE = "beefy-bls-hash-to-curve-vector";

    /// `u[0]` and `u[1]` from the Rust vector, before the map to the curve.
    bytes constant EXPECTED_U0 =
        hex"19fa8d7582393438a7bd7ef7e789de283142e027986bc7f5f1919c106071b346a735f1cde455b6d0e9547783b4ffbbc3";
    bytes constant EXPECTED_U1 =
        hex"15a0511a246ac447601f9abf8f3c6639328a907e15ba3981923b222f26d5440390996a0a065a227f75c23c552cf37960";

    /// The resulting G1 point.
    bytes constant EXPECTED_X =
        hex"0e1caf33eecf4c4d00dc4c2d7dc2f1d9ffef352cdcf50a359caf0c9ddcb49de2a4124773cc45d92901079834fc0743ea";
    bytes constant EXPECTED_Y =
        hex"1427e29396b9a044ac8475c5939054e529c70a5c01cdb71e6a845bcb2ad0342dfb148c6860e084fa7a84aa1ea4c54385";

    /// The ciphersuite is prepended to the message rather than used as the domain separation tag.
    function test_ciphersuite_matches_w3f_bls() public pure {
        assertEq(
            BlsHashToCurve.CIPHER_SUITE,
            bytes("BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_NUL_"),
            "ciphersuite drifted from what w3f-bls composes"
        );
        // DST_prime = DST || len(DST), and the DST really is the single byte 0x01.
        assertEq(BlsHashToCurve.DST_PRIME, hex"0101", "dst is not the one byte 0x01");
    }

    /// `hash_to_field` is the part the contract implements by hand, so it is checked on its own.
    function test_hash_to_field_matches_rust() public view {
        bytes memory preimage = bytes.concat(BlsHashToCurve.CIPHER_SUITE, MESSAGE);
        (bytes memory u0, bytes memory u1) = BlsHashToCurve.hashToField(preimage);

        // Field elements are returned padded to 64 bytes; the value is the trailing 48.
        assertEq(_trim(u0), EXPECTED_U0, "u[0] differs from the Rust vector");
        assertEq(_trim(u1), EXPECTED_U1, "u[1] differs from the Rust vector");
    }

    /// The whole pipeline, including the EIP-2537 map and addition.
    function test_hash_commitment_to_g1_matches_rust() public view {
        bytes memory point = BlsHashToCurve.hashCommitmentToG1(MESSAGE);
        assertEq(point.length, 128, "G1 points are 128 bytes");

        bytes memory x = new bytes(48);
        bytes memory y = new bytes(48);
        for (uint256 i = 0; i < 48; ++i) {
            x[i] = point[16 + i];
            y[i] = point[64 + 16 + i];
        }

        assertEq(x, EXPECTED_X, "point.x differs from the Rust vector");
        assertEq(y, EXPECTED_Y, "point.y differs from the Rust vector");
    }

    /// Strip the 16 bytes of zero padding EIP-2537 puts in front of a field element.
    function _trim(bytes memory padded) private pure returns (bytes memory) {
        bytes memory out = new bytes(48);
        for (uint256 i = 0; i < 48; ++i) {
            out[i] = padded[16 + i];
        }
        return out;
    }
}
