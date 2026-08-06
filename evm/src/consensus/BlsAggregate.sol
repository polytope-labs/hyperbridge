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

import {BlsHashToCurve} from "./BlsHashToCurve.sol";

/**
 * @title Aggregate BLS12-381 signature verification for BEEFY.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice Checks that a supermajority of the validator set signed a commitment, in a single
 * pairing operation regardless of how many of them there are. This is the point of the BLS path:
 * the ECDSA verifier spends one `ecrecover` per signature, so its cost grows with the set, while
 * this does not.
 *
 * @dev The identity checked is the standard aggregate:
 *
 *     e(sum(sig_i), g2_generator) == e(H(commitment), sum(pubkey_i))
 *
 * `PAIRING_CHECK` tests whether a product of pairings equals one, so it is rearranged as
 *
 *     e(sum(sig_i), -g2_generator) * e(H(commitment), sum(pubkey_i)) == 1
 *
 * which is why the negated generator is a constant here.
 *
 * Signatures live in G1 and public keys in G2, the opposite of the Ethereum convention.
 *
 * Points are taken **uncompressed**. EIP-2537 has no decompression precompile, and recovering a
 * G2 point from its compressed form needs an Fp2 square root, which is expensive in Solidity. The
 * prover therefore supplies uncompressed coordinates. Compressing them again is cheap, so
 * [`compressG2`] derives the form the relay chain commits to when a merkle leaf is needed.
 */
library BlsAggregate {
    /// @dev EIP-2537 BLS12_G2ADD
    address internal constant G2_ADD = address(0x0d);
    /// @dev EIP-2537 BLS12_PAIRING_CHECK
    address internal constant PAIRING_CHECK = address(0x0f);

    /// @dev A G1 point: x || y, each a 64 byte field element.
    uint256 internal constant G1_POINT_LEN = 128;
    /// @dev A G2 point: x.c0 || x.c1 || y.c0 || y.c1, each a 64 byte field element.
    uint256 internal constant G2_POINT_LEN = 256;

    /**
     * @dev The negated BLS12-381 G2 generator, uncompressed, as EIP-2537 encodes it. Negation on
     * this curve is `y -> p - y`, applied to both Fp2 coefficients; x is unchanged.
     */
    bytes internal constant NEG_G2_GENERATOR = hex"00000000000000000000000000000000"
        hex"024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8"
        hex"00000000000000000000000000000000"
        hex"13e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e"
        hex"00000000000000000000000000000000"
        hex"0d1b3cc2c7027888be51d9ef691d77bcb679afda66c73f17f9ee3837a55024f78c71363275a75d75d86bab79f74782aa"
        hex"00000000000000000000000000000000"
        hex"13fa4d4a0ad8b1ce186ed5061789213d993923066dddaf1040bc3ff59f825c78df74f2d75467e25e0f55f8a00fa030ed";

    /// @dev `(p - 1) / 2`. A compressed point records which of the two square roots `y` is, and
    /// the convention is "the larger one", meaning `y > (p - 1) / 2`.
    bytes internal constant HALF_MODULUS =
        hex"0d0088f51cbff34d258dd3db21a5d66bb23ba5c279c2895fb39869507b587b120f55ffff58a9ffffdcff7fffffffd555";

    /// @dev Set on the first byte of a compressed point.
    uint8 internal constant COMPRESSION_FLAG = 0x80;
    /// @dev Set when `y` is the larger of the two roots.
    uint8 internal constant SIGN_FLAG = 0x20;

    error G2AddFailed();
    error PairingFailed();
    error InvalidPointLength();
    error NoSigners();

    /**
     * @notice Verify that `aggregateSignature` is the sum of signatures over `commitment` by the
     * holders of `publicKeys`.
     * @param commitment the SCALE-encoded BEEFY commitment, hashed onto G1 internally
     * @param aggregateSignature a G1 point, 128 bytes uncompressed
     * @param publicKeys the signers' G2 points, 256 bytes uncompressed each
     */
    function verify(bytes memory commitment, bytes memory aggregateSignature, bytes[] memory publicKeys)
        internal
        view
        returns (bool)
    {
        if (publicKeys.length == 0) revert NoSigners();
        if (aggregateSignature.length != G1_POINT_LEN) revert InvalidPointLength();

        bytes memory aggregateKey = sumG2(publicKeys);
        bytes memory messagePoint = BlsHashToCurve.hashCommitmentToG1(commitment);

        // Two pairs: (sig, -g2_gen) and (H(msg), aggregate key). Their product is one exactly when
        // the aggregate signature is valid for the aggregate key.
        bytes memory input = bytes.concat(aggregateSignature, NEG_G2_GENERATOR, messagePoint, aggregateKey);

        (bool ok, bytes memory result) = PAIRING_CHECK.staticcall(input);
        if (!ok || result.length != 32) revert PairingFailed();

        return abi.decode(result, (uint256)) == 1;
    }

    /// @notice Sum a set of uncompressed G2 points with `G2ADD`.
    function sumG2(bytes[] memory points) internal view returns (bytes memory) {
        if (points.length == 0) revert NoSigners();
        if (points[0].length != G2_POINT_LEN) revert InvalidPointLength();

        bytes memory acc = points[0];
        for (uint256 i = 1; i < points.length; ++i) {
            if (points[i].length != G2_POINT_LEN) revert InvalidPointLength();

            (bool ok, bytes memory sum) = G2_ADD.staticcall(bytes.concat(acc, points[i]));
            if (!ok || sum.length != G2_POINT_LEN) revert G2AddFailed();
            acc = sum;
        }

        return acc;
    }

    /**
     * @notice Compress an uncompressed G2 point to the 96 byte form the relay chain commits to.
     *
     * @dev The keyset commitment is built over compressed keys, but EIP-2537 only accepts
     * uncompressed ones, so the prover sends uncompressed and this derives the compressed form for
     * the merkle leaf. Going this direction is cheap: take `x` and set two flag bits. The reverse
     * would need an Fp2 square root, which is why the proof does not simply carry compressed keys.
     *
     * The layout is `x.c1 || x.c0`, c1 first, with the flags in the top bits of the first byte.
     * The sign bit tracks `y.c1`, confirmed against `bls_compression_rule` in the Rust verifier.
     */
    function compressG2(bytes memory point) internal pure returns (bytes memory) {
        if (point.length != G2_POINT_LEN) revert InvalidPointLength();

        bytes memory out = new bytes(96);
        for (uint256 i = 0; i < 48; ++i) {
            // x.c1 occupies bytes 64..128, its value in the trailing 48; x.c0 is bytes 0..64.
            out[i] = point[80 + i];
            out[48 + i] = point[16 + i];
        }

        uint8 flags = COMPRESSION_FLAG;
        // y.c1 occupies bytes 192..256, its value in the trailing 48.
        if (greaterThanHalfModulus(point, 208)) flags |= SIGN_FLAG;
        out[0] = bytes1(uint8(out[0]) | flags);

        return out;
    }

    /// @dev Whether the 48 byte big-endian value at `offset` exceeds `(p - 1) / 2`.
    function greaterThanHalfModulus(bytes memory value, uint256 offset) internal pure returns (bool) {
        for (uint256 i = 0; i < 48; ++i) {
            uint8 a = uint8(value[offset + i]);
            uint8 b = uint8(HALF_MODULUS[i]);
            if (a != b) return a > b;
        }
        return false;
    }
}
