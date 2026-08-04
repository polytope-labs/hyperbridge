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
pragma solidity ^0.8.17;

import {BlsBeefyConsensusProof, BeefyConsensusState} from "./Types.sol";

/**
 * @title The aggregate BLS12-381 BEEFY consensus proof types.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice Carries the ABI shape of a BLS BEEFY proof. The verifier itself is not implemented here
 * yet; the Rust light client verifies these proofs today, and this exists so both sides agree on
 * the encoding before the on-chain verifier is written.
 *
 * @dev When the verifier lands it verifies the aggregate in one pairing check regardless of how
 * many validators signed, which is the point of the BLS path. The steps will be:
 *  1. Hash the SCALE-encoded commitment onto G1. Note BEEFY does not use the IETF ciphersuite
 *     string as the domain separation tag: `w3f-bls` builds its hasher with a one-byte DST of
 *     0x01 and prepends the ciphersuite to the message instead. Reproducing that exactly is the
 *     make-or-break step, and `bls_hash_to_curve_vector` in the Rust verifier pins a test vector.
 *  2. Sum the signers' G2 public keys and check the aggregate against the keyset commitment with
 *     a merkle multi-proof, rejecting repeated or unordered signer indices.
 *  3. One EIP-2537 pairing check over the summed key and the aggregate signature.
 *
 * Signatures live in G1 (48 bytes) and public keys in G2 (96 bytes), the opposite of the Ethereum
 * convention, so an eth2 BLS library does not transfer.
 */
contract BlsBeefy {
    // @dev so these structs are included in the abi
    function noOp(BeefyConsensusState memory s, BlsBeefyConsensusProof memory p) external pure {}
}
