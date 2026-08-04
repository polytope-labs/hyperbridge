// SPDX-License-Identifier: Apache-2.0
// Copyright (C) Polytope Labs Ltd.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
pragma solidity ^0.8.30;

/**
 * @title Hashing a BEEFY commitment onto the BLS12-381 G1 curve.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice Reproduces exactly what substrate's BEEFY signers do, so an aggregate BLS signature can
 * be checked on chain. This is the step most likely to be got wrong, because BEEFY does not use
 * the IETF ciphersuite string the way a reading of RFC 9380 alone would suggest.
 *
 * @dev `w3f-bls` builds its hasher with a **one byte domain separation tag of 0x01**, and prepends
 * the ciphersuite string to the *message* instead of using it as the tag:
 *
 *   suite    = "BLS_SIG_" || "BLS12381" || "G1" || "_XMD:SHA-256_SSWU_RO_" || "NUL_"
 *   preimage = suite || context || message        // context is empty for BEEFY
 *   point    = hash_to_curve(preimage, DST = 0x01)
 *
 * Using `suite` as the DST, which is what a textbook implementation does, yields a completely
 * different point and a silently failing pairing check. `bls_hash_to_curve_vector` in the Rust
 * verifier pins a test vector that this library is checked against.
 *
 * Everything after that composition is standard RFC 9380 and maps onto EIP-2537 precompiles.
 * Note BEEFY puts signatures in G1 and public keys in G2, the opposite of the Ethereum
 * convention, so an eth2 BLS library does not transfer.
 */
library BlsHashToCurve {
    /// @dev EIP-2537 BLS12_G1ADD
    address internal constant G1_ADD = address(0x0b);
    /// @dev EIP-2537 BLS12_MAP_FP_TO_G1
    address internal constant MAP_FP_TO_G1 = address(0x10);
    /// @dev Big-endian modular exponentiation (EIP-198), used to reduce a wide integer mod p.
    address internal constant MOD_EXP = address(0x05);

    /// @dev The BLS12-381 base field modulus, 48 bytes big-endian.
    bytes internal constant FIELD_MODULUS =
        hex"1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab";

    /// @dev The ciphersuite `w3f-bls` prepends to the message. Not the domain separation tag.
    bytes internal constant CIPHER_SUITE = "BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_NUL_";

    /// @dev `DST_prime` = DST || I2OSP(len(DST), 1), with the one byte DST `w3f-bls` uses.
    bytes internal constant DST_PRIME = hex"0101";

    error MapToG1Failed();
    error G1AddFailed();
    error ModExpFailed();

    /**
     * @notice Hash a SCALE-encoded BEEFY commitment onto G1, exactly as its signers did.
     * @param commitment the SCALE-encoded commitment
     * @return a G1 point, 128 bytes, as EIP-2537 encodes them (x || y, each 64 bytes)
     */
    function hashCommitmentToG1(bytes memory commitment) internal view returns (bytes memory) {
        return hashToG1(bytes.concat(CIPHER_SUITE, commitment));
    }

    /**
     * @notice RFC 9380 `hash_to_curve` for BLS12-381 G1 with the DST `w3f-bls` uses.
     * @dev Cofactor clearing is linear, so it makes no difference that MAP_FP_TO_G1 applies it per
     * point rather than once after the addition.
     */
    function hashToG1(bytes memory preimage) internal view returns (bytes memory) {
        (bytes memory u0, bytes memory u1) = hashToField(preimage);
        return g1Add(mapToG1(u0), mapToG1(u1));
    }

    /**
     * @notice RFC 9380 `hash_to_field` producing the two field elements the map consumes.
     * @return u0 and u1, each 64 bytes: a 48 byte big-endian value left padded to 64, which is how
     * EIP-2537 wants field elements.
     */
    function hashToField(bytes memory preimage) internal view returns (bytes memory u0, bytes memory u1) {
        // L = ceil((ceil(log2(p)) + k) / 8) = ceil((381 + 128) / 8) = 64, and we need two of them.
        bytes memory uniform = expandMessageXmd(preimage, 128);

        bytes memory wide0 = new bytes(64);
        bytes memory wide1 = new bytes(64);
        for (uint256 i = 0; i < 64; ++i) {
            wide0[i] = uniform[i];
            wide1[i] = uniform[64 + i];
        }

        u0 = toFieldElement(wide0);
        u1 = toFieldElement(wide1);
    }

    /**
     * @notice RFC 9380 section 5.3.1 `expand_message_xmd` with SHA-256.
     * @dev Written for `lenInBytes` a multiple of 32 and at most 255 blocks, which covers the only
     * caller (128 bytes, so four blocks).
     */
    function expandMessageXmd(bytes memory message, uint16 lenInBytes) internal pure returns (bytes memory) {
        uint256 ell = (lenInBytes + 31) / 32;

        // msg_prime = Z_pad || msg || l_i_b_str || I2OSP(0, 1) || DST_prime
        bytes memory zPad = new bytes(64); // SHA-256 block size
        bytes memory msgPrime = bytes.concat(zPad, message, bytes2(lenInBytes), hex"00", DST_PRIME);

        bytes32 b0 = sha256(msgPrime);
        bytes32 bi = sha256(bytes.concat(b0, hex"01", DST_PRIME));

        bytes memory out = bytes.concat(bi);
        for (uint256 i = 2; i <= ell; ++i) {
            bi = sha256(bytes.concat(b0 ^ bi, bytes1(uint8(i)), DST_PRIME));
            out = bytes.concat(out, bi);
        }

        return out;
    }

    /// @notice Reduce a 64 byte big-endian integer mod p, returned padded to the 64 bytes
    /// EIP-2537 expects for a field element.
    function toFieldElement(bytes memory wide) internal view returns (bytes memory) {
        // modexp(base = wide, exponent = 1, modulus = p) is just `wide mod p`.
        bytes memory input = bytes.concat(
            bytes32(uint256(64)), // base length
            bytes32(uint256(1)), // exponent length
            bytes32(uint256(48)), // modulus length
            wide,
            hex"01",
            FIELD_MODULUS
        );

        (bool ok, bytes memory reduced) = MOD_EXP.staticcall(input);
        if (!ok || reduced.length != 48) revert ModExpFailed();

        // EIP-2537 field elements are 64 bytes: 16 zero bytes then the 48 byte value.
        return bytes.concat(new bytes(16), reduced);
    }

    /// @notice EIP-2537 `MAP_FP_TO_G1`: field element to a G1 point, cofactor already cleared.
    function mapToG1(bytes memory fieldElement) internal view returns (bytes memory) {
        (bool ok, bytes memory point) = MAP_FP_TO_G1.staticcall(fieldElement);
        if (!ok || point.length != 128) revert MapToG1Failed();
        return point;
    }

    /// @notice EIP-2537 `G1ADD`.
    function g1Add(bytes memory a, bytes memory b) internal view returns (bytes memory) {
        (bool ok, bytes memory sum) = G1_ADD.staticcall(bytes.concat(a, b));
        if (!ok || sum.length != 128) revert G1AddFailed();
        return sum;
    }
}
