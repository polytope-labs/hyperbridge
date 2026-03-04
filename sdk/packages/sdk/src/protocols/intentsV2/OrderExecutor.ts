import type { HexString } from "@/types"
import type { IntentOrderStatusUpdate, ExecuteIntentOrderOptions, FillerBid, SelectBidResult } from "@/types"
import { sleep, DEFAULT_POLL_INTERVAL } from "@/utils"
import type { IntentsV2Context } from "./types"
import { BidManager } from "./BidManager"

export class OrderExecutor {
	constructor(
		private readonly ctx: IntentsV2Context,
		private readonly bidManager: BidManager,
	) {}

	async *executeIntentOrder(options: ExecuteIntentOrderOptions): AsyncGenerator<IntentOrderStatusUpdate, void> {
		const {
			order,
			sessionPrivateKey,
			minBids = 1,
			bidTimeoutMs = 60_000,
			pollIntervalMs = DEFAULT_POLL_INTERVAL,
		} = options

		const commitment = order.id as HexString
		const isSameChain = order.source === order.destination

		if (!this.ctx.intentsCoprocessor) {
			yield {
				status: "FAILED",
				metadata: { error: "IntentsCoprocessor required for order execution" },
			}
			return
		}

		if (!this.ctx.bundlerUrl) {
			yield {
				status: "FAILED",
				metadata: { error: "Bundler URL not configured" },
			}
			return
		}

		try {
			const usedUserOps = new Set<string>()

			while (true) {
				yield {
					status: "AWAITING_BIDS",
					metadata: { commitment },
				}

				const startTime = Date.now()
				let bids: FillerBid[] = []

				while (Date.now() - startTime < bidTimeoutMs) {
					try {
						bids = await this.ctx.intentsCoprocessor!.getBidsForOrder(commitment)

						if (bids.length >= minBids) {
							break
						}
					} catch {
						// Continue polling on errors
					}

					await sleep(pollIntervalMs)
				}

				const freshBids = bids.filter((bid) => {
					const key = `${bid.userOp.sender.toLowerCase()}-${bid.userOp.nonce.toString()}`
					return !usedUserOps.has(key)
				})

				if (freshBids.length === 0) {
					yield {
						status: "FAILED",
						metadata: {
							commitment,
							error: `No new bids available within ${bidTimeoutMs}ms timeout`,
						},
					}
					return
				}

				yield {
					status: "BIDS_RECEIVED",
					metadata: {
						commitment,
						bidCount: freshBids.length,
						bids: freshBids,
					},
				}

				let result: SelectBidResult
				try {
					result = await this.bidManager.selectBid(order, freshBids, sessionPrivateKey)
				} catch (err) {
					yield {
						status: "FAILED",
						metadata: {
							commitment,
							error: `Failed to select bid and submit: ${err instanceof Error ? err.message : String(err)}`,
						},
					}
					return
				}

				const usedKey = `${result.userOp.sender.toLowerCase()}-${result.userOp.nonce.toString()}`
				usedUserOps.add(usedKey)

				yield {
					status: "BID_SELECTED",
					metadata: {
						commitment,
						selectedSolver: result.solverAddress,
						userOpHash: result.userOpHash,
						userOp: result.userOp,
					},
				}

				yield {
					status: "USEROP_SUBMITTED",
					metadata: {
						commitment,
						userOpHash: result.userOpHash,
						selectedSolver: result.solverAddress,
						transactionHash: result.txnHash,
					},
				}

				if (!isSameChain) {
					return
				}

				if (result.fillStatus === "full") {
					yield {
						status: "FILLED",
						metadata: {
							commitment,
							userOpHash: result.userOpHash,
							selectedSolver: result.solverAddress,
							transactionHash: result.txnHash,
						},
					}
					return
				}

				if (result.fillStatus === "partial") {
					yield {
						status: "PARTIAL_FILL",
						metadata: {
							commitment,
							userOpHash: result.userOpHash,
							selectedSolver: result.solverAddress,
							transactionHash: result.txnHash,
						},
					}
				}
			}
		} catch (err) {
			yield {
				status: "FAILED",
				metadata: {
					commitment,
					error: `Unexpected error: ${err instanceof Error ? err.message : String(err)}`,
				},
			}
		}
	}
}
