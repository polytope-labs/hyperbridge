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
 * G2 point from its compressed form needs an Fp2 square root, which is expensive in Solidity, so
 * the prover supplies uncompressed coordinates.
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
}
