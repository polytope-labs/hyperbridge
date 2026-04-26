import { hexToBytes } from "viem"
import { env, hasWindow, isNode } from "std-env"
import { postRequestCommitment } from "@/utils"
import type { HexString, IPostRequest } from "@/types"

/**
 * Generates a Merkle Mountain Range (MMR) root hash and proof for a post request.
 *
 * This function takes a post request and tree size, encodes it according to the PostRequest format,
 * and generates both the MMR root hash and a proof. The function builds an MMR with `treeSize` leaves,
 * where most leaves are variations of the encoded request (XORed with their index), except for the
 * last leaf, which is the unmodified request. The proof is generated for this unmodified leaf.
 *
 * @param postRequest - The post request to generate the MMR root and proof for
 * @param treeSize - Controls how many leaves will be added to the MMR (exactly `treeSize` leaves)
 * @returns An object containing:
 *   - root: The MMR root hash as a hex string
 *   - proof: An array of hex strings representing the MMR proof for the unmodified request
 *   - index: The index of the unmodified request in the MMR
 *   - treeSize: The number of leaves in the MMR
 *   - mmrSize: The size of the MMR in nodes
 */
export async function generateRootWithProof(
	postRequest: IPostRequest,
	treeSize: bigint,
): Promise<{ root: HexString; proof: HexString[]; index: bigint; treeSize: bigint; mmrSize: bigint }> {
	const { generate_root_with_proof } = await load_ckb_mmr()
	const { commitment: hash, encodePacked } = postRequestCommitment(postRequest)

	const result = JSON.parse(generate_root_with_proof(hexToBytes(encodePacked), treeSize))
	const { root, proof, mmr_size, keccak_hash_calldata } = result

	if (keccak_hash_calldata !== hash) {
		console.log("keccak_hash", keccak_hash_calldata)
		console.log("hash", hash)
		throw new Error("Abi keccak hash mismatch")
	}

	return {
		root: root as HexString,
		proof: proof as HexString[],
		index: treeSize - 1n,
		treeSize,
		mmrSize: mmr_size,
	}
}

async function load_ckb_mmr() {
	if (hasWindow) {
		const wasm = await import("@/ckb-utils/web")
		await wasm.default()

		return wasm
	}

	if (isNode) {
		const wasm = await import("@/ckb-utils/node")
		return wasm
	}

	throw new Error(`SDK not setup for ${env}`)
}

export async function __test() {
	const { generate_root_with_proof } = await load_ckb_mmr()

	return generate_root_with_proof(new Uint8Array(), 120n)
}
