import { hexToBytes } from "viem"
import { env, hasWindow, isNode } from "std-env"
import { postRequestCommitment } from "@/utils"
import type { HexString, IPostRequest } from "@/types"

/**
 * Gets peak position for a given height
 */
function getPeakPosByHeight(height: number): bigint {
	return (1n << BigInt(height + 1)) - 2n
}

/**
 * Gets the height and position of the leftmost peak
 */
function leftPeakHeightPos(mmrSize: bigint): [number, bigint] {
	let height = 1
	let prevPos = 0n
	let pos = getPeakPosByHeight(height)

	while (pos < mmrSize) {
		height += 1
		prevPos = pos
		pos = getPeakPosByHeight(height)
	}

	return [height - 1, prevPos]
}

/**
 * Gets the right peak at a given height and position
 */
function getRightPeak(initialHeight: number, initialPos: bigint, mmrSize: bigint): [number, bigint] | null {
	let height = initialHeight
	let pos = initialPos

	// move to right sibling pos
	pos += siblingOffset(height)

	// loop until we find a pos in mmr
	while (pos > mmrSize - 1n) {
		if (height === 0) {
			return null
		}
		// move to left child
		pos -= parentOffset(height - 1)
		height -= 1
	}

	return [height, pos]
}

/**
 * Gets all peaks in the MMR
 */
function getPeaks(mmrSize: bigint): bigint[] {
	const positions: bigint[] = []
	let [height, pos] = leftPeakHeightPos(mmrSize)

	positions.push(pos)

	while (height > 0) {
		const peak = getRightPeak(height, pos, mmrSize)
		if (!peak) break
		;[height, pos] = peak
		positions.push(pos)
	}

	return positions
}

/**
 * Checks if a number consists of all ones in its binary representation
 */
function allOnes(num: bigint): boolean {
	if (num === 0n) return false
	return num
		.toString(2)
		.split("")
		.every((bit) => bit === "1")
}

/**
 * Calculates the position after jumping left in the tree
 */
function jumpLeft(pos: bigint): bigint {
	const bitLength = pos.toString(2).length
	const mostSignificantBits = 1n << BigInt(bitLength - 1)
	return pos - (mostSignificantBits - 1n)
}

/**
 * Calculates the height of a position in the tree
 */
function posHeightInTree(initialPos: bigint): number {
	let pos = initialPos + 1n

	while (!allOnes(pos)) {
		pos = jumpLeft(pos)
	}

	return pos.toString(2).length - 1
}

/**
 * Calculates the parent offset for a given height
 */
function parentOffset(height: number): bigint {
	return 2n << BigInt(height)
}

/**
 * Calculates the sibling offset for a given height
 */
function siblingOffset(height: number): bigint {
	return (2n << BigInt(height)) - 1n
}

/**
 * Takes elements from a vector while they satisfy a predicate
 */
function takeWhileVec<T>(v: T[], p: (item: T) => boolean): T[] {
	const index = v.findIndex((item) => !p(item))
	if (index === -1) {
		const result = [...v]
		v.length = 0
		return result
	}
	return v.splice(0, index)
}

/**
 * Converts a node's MMR position to its k-index
 * @param leaves - Array of leaf positions
 * @param mmrSize - Size of the MMR
 * @returns Array of tuples containing position and k-index
 */
export function mmrPositionToKIndex(initialLeaves: bigint[], mmrSize: bigint): Array<[bigint, bigint]> {
	const leaves = [...initialLeaves]
	const peaks = getPeaks(mmrSize)
	const leavesWithKIndices: Array<[bigint, bigint]> = []

	for (const peak of peaks) {
		const peakLeaves = takeWhileVec(leaves, (pos) => pos <= peak)

		if (peakLeaves.length > 0) {
			for (const pos of peakLeaves) {
				const height = posHeightInTree(peak)
				let index = 0n
				let parentPos = peak

				for (let h = height; h >= 1; h--) {
					const leftChild = parentPos - parentOffset(h - 1)
					const rightChild = leftChild + siblingOffset(h - 1)
					index *= 2n

					if (leftChild >= pos) {
						parentPos = leftChild
					} else {
						parentPos = rightChild
						index += 1n
					}
				}

				leavesWithKIndices.push([pos, index])
			}
		}
	}

	return leavesWithKIndices
}

/**
 * Calculate the total size of MMR (number of nodes) from the number of leaves.
 * @param numberOfLeaves - The number of leaves in the MMR
 * @returns The total size of the MMR (total number of nodes)
 */
export function calculateMMRSize(numberOfLeaves: bigint): bigint {
	const numberOfPeaks = numberOfLeaves.toString(2).split("1").length - 1
	return 2n * numberOfLeaves - BigInt(numberOfPeaks)
}

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
 *   - kIndex: The k-index of the unmodified request in the MMR
 *   - treeSize: The number of leaves in the MMR
 *   - mmrSize: The size of the MMR in nodes
 */
export async function generateRootWithProof(
	postRequest: IPostRequest,
	treeSize: bigint,
): Promise<{ root: HexString; proof: HexString[]; index: bigint; kIndex: bigint; treeSize: bigint; mmrSize: bigint }> {
	const { generate_root_with_proof } = await load_ckb_mmr()
	const { commitment: hash, encodePacked } = postRequestCommitment(postRequest)

	const result = JSON.parse(generate_root_with_proof(hexToBytes(encodePacked), treeSize))
	const { root, proof, mmr_size, leaf_positions, keccak_hash_calldata } = result

	if (keccak_hash_calldata !== hash) {
		console.log("keccak_hash", keccak_hash_calldata)
		console.log("hash", hash)
		throw new Error("Abi keccak hash mismatch")
	}

	const [[, kIndex]] = mmrPositionToKIndex(leaf_positions, BigInt(mmr_size))

	return {
		root: root as HexString,
		proof: proof as HexString[],
		index: treeSize - 1n,
		kIndex,
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
