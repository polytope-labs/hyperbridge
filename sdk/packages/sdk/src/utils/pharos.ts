import { Struct, Tuple, Vector, u8, u32 } from "scale-ts"

/**
 * SCALE codecs for Pharos state proofs. These mirror the Rust types defined in
 * `modules/ismp/state-machines/pharos/src/lib.rs` and
 * `modules/consensus/pharos/primitives/src/types.rs`.
 *
 * Pharos uses a flat hexary hash tree with SHA-256 hashing instead of Ethereum's
 * MPT with Keccak-256. The proof data is emitted by the `eth_getProof` endpoint on
 * Pharos nodes in a custom format (hex-encoded proof node bytes plus offsets that
 * locate the next child hash within each node).
 */

/** Single node in a Pharos hexary hash tree proof path. */
export const PharosProofNodeCodec = Struct({
	proofNode: Vector(u8),
	nextBeginOffset: u32,
	nextEndOffset: u32,
})

/** Sibling leftmost-leaf proof used when witnessing non-existence. */
export const SiblingLeftmostLeafProofCodec = Struct({
	slotIndex: u8,
	leftmostLeafKey: Vector(u8),
	proofPath: Vector(PharosProofNodeCodec),
})

/** Non-existence proof: empty-slot witness or leaf-key-mismatch witness. */
export const NonExistenceProofCodec = Struct({
	proofNodes: Vector(PharosProofNodeCodec),
	siblingProofs: Vector(SiblingLeftmostLeafProofCodec),
})

/** Account proof data: path from state root down to an account leaf. */
export const AccountProofDataCodec = Struct({
	proofNodes: Vector(PharosProofNodeCodec),
	rawValue: Vector(u8),
})

/**
 * Pharos state proof (replaces `EvmStateProof` for Pharos chains).
 *
 * Scale-encoded `BTreeMap<Vec<u8>, _>` is layout-compatible with
 * `Vec<(Vec<u8>, _)>` provided entries are sorted by key; we sort before encoding.
 */
export const PharosStateProof = Struct({
	storageProof: Vector(Tuple(Vector(u8), Vector(PharosProofNodeCodec))),
	storageValues: Vector(Tuple(Vector(u8), Vector(u8))),
	nonExistenceProofs: Vector(Tuple(Vector(u8), NonExistenceProofCodec)),
	accountProofs: Vector(Tuple(Vector(u8), AccountProofDataCodec)),
})