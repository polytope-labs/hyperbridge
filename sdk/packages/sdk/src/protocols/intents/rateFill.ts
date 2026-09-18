export interface RateFillPreview {
	credit: bigint
	release: bigint
	delivered: bigint
	surplus: bigint
}

function ceilDiv(numerator: bigint, denominator: bigint): bigint {
	return numerator === 0n ? 0n : (numerator - 1n) / denominator + 1n
}

/** Mirrors IntentGatewayV2's integer settlement arithmetic for one quoted leg. */
export function previewRateFill(
	escrow: bigint,
	required: bigint,
	filled: bigint,
	take: bigint,
	offered: bigint,
): RateFillPreview {
	if (escrow <= 0n || required <= 0n || take <= 0n || offered <= 0n) {
		throw new Error("RateFillInvalidAmount")
	}
	if (filled < 0n || filled >= required) throw new Error("RateFillNoProgress")
	if (ceilDiv(take * required, escrow) > offered) throw new Error("RateBelowOrder")

	const uncappedCredit = (take * required) / escrow
	const credit = uncappedCredit < required - filled ? uncappedCredit : required - filled
	const release = (escrow * (filled + credit)) / required - (escrow * filled) / required
	if (credit === 0n || release === 0n) throw new Error("RateFillNoProgress")

	let delivered = offered
	if (uncappedCredit > required - filled) {
		const prorated = ceilDiv(offered * release, take)
		delivered = prorated > credit ? prorated : credit
	}

	return { credit, release, delivered, surplus: delivered - credit }
}
