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
