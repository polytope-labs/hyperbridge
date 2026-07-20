import { SubstrateExtrinsic } from "@subql/types"

/**
 * Finds a call within an extrinsic's call tree and returns one of its arguments.
 *
 * Dispatches routinely arrive wrapped — utility.batch, proxy.proxy, sudo — so the call that raised
 * an event is often not the extrinsic's top-level method. `match` picks the call of interest (it
 * receives the call's decoded args, so it can discriminate between sibling calls of the same kind),
 * and its return value is what comes back.
 *
 * Kept free of SDK imports so it stays cheap to unit test.
 */
export function findInCallTree<T>(
	extrinsic: SubstrateExtrinsic | undefined,
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	match: (call: { section: string; method: string; args: any[] }) => T | undefined,
): T | undefined {
	if (!extrinsic) return undefined

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	const visit = (call: any, depth: number): T | undefined => {
		// Wrapped calls nest at most a few levels; the bound just stops a malformed or cyclic tree
		// from looping forever.
		if (!call || depth > 6) return undefined

		const args = call.args
		if (!Array.isArray(args)) return undefined

		if (typeof call.section === "string" && typeof call.method === "string") {
			const matched = match(call)
			if (matched !== undefined) return matched
		}

		for (const arg of args) {
			// A nested call arg carries section/method itself; batch-style args carry an array of them.
			if (Array.isArray(arg)) {
				for (const inner of arg) {
					const found = visit(inner, depth + 1)
					if (found !== undefined) return found
				}
			} else if (arg?.section && arg?.method) {
				const found = visit(arg, depth + 1)
				if (found !== undefined) return found
			}
		}

		return undefined
	}

	try {
		return visit(extrinsic.extrinsic.method, 0)
	} catch {
		return undefined
	}
}

/**
 * Pulls the `user_op` argument out of the place_bid call that raised a BidPlaced event.
 *
 * Matches on commitment, not just on the call being a place_bid: one batch may carry bids for
 * several orders, and taking the first would attribute another order's quote to this event.
 */
export function extractUserOpFromExtrinsic(
	extrinsic: SubstrateExtrinsic | undefined,
	commitment: string,
): string | undefined {
	return findInCallTree(extrinsic, (call) => {
		if (call.section !== "intentsCoprocessor" || call.method !== "placeBid") return undefined
		const [commitmentArg, userOpArg] = call.args
		if (commitmentArg?.toHex?.() !== commitment) return undefined
		return userOpArg?.toHex?.() as string | undefined
	})
}
