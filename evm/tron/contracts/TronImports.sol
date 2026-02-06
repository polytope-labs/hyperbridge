// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

// =============================================================================
// TronBox Import Hub
// =============================================================================
// This file transitively imports all contracts needed for the TRON deployment.
// TronBox will compile these and generate build artifacts automatically.
//
// No symlinks or file copies required — just relative paths back to evm/src/.
// =============================================================================

// ── Core ────────────────────────────────────────────────────────────────────

import {TronHost} from "../../src/hosts/Tron.sol";
import {EvmHost, HostParams, PerByteFee} from "../../src/core/EvmHost.sol";
import {HandlerV1} from "../../src/core/HandlerV1.sol";
import {HostManager, HostManagerParams} from "../../src/core/HostManager.sol";

// ── Consensus ───────────────────────────────────────────────────────────────

import {BeefyV1FiatShamir} from "../../src/consensus/BeefyV1FiatShamir.sol";
import {MultiProofClient} from "../../src/consensus/MultiProofClient.sol";
import {HeaderImpl} from "../../src/consensus/Header.sol";
import {Codec} from "../../src/consensus/Codec.sol";
import {Transcript} from "../../src/consensus/Transcript.sol";

// ── Libraries ───────────────────────────────────────────────────────────────

import {MerklePatricia} from "@polytope-labs/solidity-merkle-trees/src/MerklePatricia.sol";

// ── Utilities ───────────────────────────────────────────────────────────────

import {CallDispatcher} from "../../src/utils/CallDispatcher.sol";
