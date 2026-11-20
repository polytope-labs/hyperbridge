import { describe, expect, it } from "vitest"
import type { HexString } from "@hyperbridge/sdk"
import { EventMonitor } from "@/core/event-monitor"
import type { ScannedFill } from "@/scanner/types"
import { stubOrderScanner } from "../helpers/stub-scanner"

/**
 * A fill's block is what the limit order's on-chain tally is read at. A scanner that
 * cannot tell substitutes `0n`, and a read pinned to block 0 fails, so settlement would
 * never lower the order. The monitor leaves such a block unset, which reads the latest.
 */

const OURS = "0xAAAA00000000000000000000000000000000AAAA" as HexString
const COMMITMENT = `0x${"01".repeat(32)}` as HexString

async function observe(blockNumber: bigint) {
	const scanner = stubOrderScanner([8453])
	const monitor = new EventMonitor([{ chainId: 8453 } as never], { loggers: undefined } as never, OURS, scanner)
	const seen: Array<{ blockNumber?: bigint }> = []
	monitor.on("orderFilledOnChain", (event) => seen.push(event))
	await monitor.startListening()

	const fill: ScannedFill = {
		commitment: COMMITMENT,
		filler: OURS,
		chainId: 8453,
		blockNumber,
		blockHash: "0xb",
		logIndex: 0,
		outputs: [],
		inputs: [],
		complete: true,
	}
	scanner.emit.onFill(fill)
	expect(seen).toHaveLength(1)
	return seen[0]
}

describe("the block a fill is forwarded with", () => {
	it("is the block the fill landed in", async () => {
		expect((await observe(12_345n)).blockNumber).toBe(12_345n)
	})

	it("is left unset when the scanner substituted 0n, so the tally reads the latest block", async () => {
		expect((await observe(0n)).blockNumber).toBeUndefined()
	})
})
