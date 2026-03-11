import type { HexString } from "@/types"
import type { IntentOrderStatusUpdate, ExecuteIntentOrderOptions, FillerBid, SelectBidResult } from "@/types"
import { sleep, DEFAULT_POLL_INTERVAL, hexToString } from "@/utils"
import type { IntentsV2Context } from "./types"
import { BidManager } from "./BidManager"

/**
 * Returns the storage key used to persist the deduplication set of already-
 * submitted UserOperation hashes for a given order commitment.
 *
 * @param commitment - The order commitment hash (bytes32).
 * @returns A namespaced string key safe to use with the `usedUserOpsStorage` adapter.
 */
const USED_USEROPS_STORAGE_KEY = (commitment: HexString) => `used-userops:${commitment.toLowerCase()}`

/**
 * Drives the post-placement execution lifecycle of an IntentGatewayV2 order.
 *
 * After an order is placed on the source chain, `OrderExecutor` polls the
 * Hyperbridge coprocessor for solver bids, selects and validates the best
 * bid, submits the corresponding ERC-4337 UserOperation via the bundler, and
 * tracks partial fills until the order is fully satisfied or exhausted.
 *
 * Deduplication of UserOperations is persisted across restarts using
 * `usedUserOpsStorage` so that the executor can resume safely after a crash.
 */
export class OrderExecutor {
	/**
	 * @param ctx - Shared IntentsV2 context providing the destination chain
	 *   client, coprocessor, bundler URL, and storage adapters.
	 * @param bidManager - Handles bid validation, sorting, simulation, and
	 *   UserOperation submission.
	 * @param crypto - Crypto utilities used to compute UserOperation hashes for
	 *   deduplication.
	 */
	constructor(
		private readonly ctx: IntentsV2Context,
		private readonly bidManager: BidManager,
		private readonly crypto: import("./CryptoUtils").CryptoUtils,
	) {}

	/**
	 * Async generator that executes an intent order by polling for bids and
	 * submitting UserOperations until the order is filled, partially exhausted,
	 * or an unrecoverable error occurs.
	 *
	 * **Status progression (cross-chain orders):**
	 * `AWAITING_BIDS` → `BIDS_RECEIVED` → `BID_SELECTED` → `USEROP_SUBMITTED`
	 * then terminates (settlement is confirmed off-chain via Hyperbridge).
	 *
	 * **Status progression (same-chain orders):**
	 * `AWAITING_BIDS` → `BIDS_RECEIVED` → `BID_SELECTED` → `USEROP_SUBMITTED`
	 * → (`FILLED` | `PARTIAL_FILL`)* → (`FILLED` | `PARTIAL_FILL_EXHAUSTED`)
	 *
	 * **Error statuses:** `FAILED` (fatal, no fills) or `PARTIAL_FILL_EXHAUSTED`
	 * (deadline reached or no new bids after at least one partial fill).
	 *
	 * @param options - Execution parameters including the placed order, its
	 *   session private key, bid collection settings, and poll interval.
	 * @yields {@link IntentOrderStatusUpdate} objects describing each stage.
	 * @throws Never throws directly; all errors are reported as `FAILED` yields.
	 */
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
			yield { status: "FAILED", error: "IntentsCoprocessor required for order execution" }
			return
		}

		if (!this.ctx.bundlerUrl) {
			yield { status: "FAILED", error: "Bundler URL not configured" }
			return
		}

		// Load or initialize persistent dedup set for this commitment from storage
		const usedUserOps = new Set<string>()
		const storageKey = USED_USEROPS_STORAGE_KEY(commitment)
		const persisted = await this.ctx.usedUserOpsStorage.getItem(storageKey)
		if (persisted) {
			try {
				const parsed = JSON.parse(persisted) as string[]
				for (const key of parsed) {
					usedUserOps.add(key)
				}
			} catch {
				// Ignore corrupt entries and start fresh
			}
		}

		const persistUsedUserOps = async () => {
			await this.ctx.usedUserOpsStorage.setItem(storageKey, JSON.stringify([...usedUserOps]))
		}

		// Precompute UserOp hashing context for this order
		const entryPointAddress = this.ctx.dest.configService.getEntryPointV08Address(hexToString(order.destination))
		const chainId = BigInt(
			this.ctx.dest.client.chain?.id ?? Number.parseInt(this.ctx.dest.config.stateMachineId.split("-")[1]),
		)

		const userOpHashKey = (userOp: SelectBidResult["userOp"] | FillerBid["userOp"]): string =>
			this.crypto.computeUserOpHash(userOp, entryPointAddress, chainId)

		// For partial fill tracking, take the total desired output amount as the sum of all output asset amounts
		const targetAmount = order.output.assets.reduce((acc, asset) => acc + asset.amount, 0n)

		let totalFilledAmount = 0n
		let remainingAmount = targetAmount

		try {
			while (true) {
				const currentBlock = await this.ctx.dest.client.getBlockNumber()
				if (currentBlock >= order.deadline) {
					const isPartiallyFilled = totalFilledAmount > 0n
					const deadlineError = `Order deadline reached (block ${currentBlock} >= ${order.deadline})`
					if (isPartiallyFilled) {
						yield { status: "PARTIAL_FILL_EXHAUSTED", commitment, totalFilledAmount, remainingAmount, error: deadlineError }
					} else {
						yield { status: "FAILED", commitment, error: deadlineError }
					}
					return
				}

				yield { status: "AWAITING_BIDS", commitment, totalFilledAmount, remainingAmount }

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
					const key = userOpHashKey(bid.userOp)
					return !usedUserOps.has(key)
				})

				if (freshBids.length === 0) {
					const isPartiallyFilled = totalFilledAmount > 0n
					const noBidsError = isPartiallyFilled
						? `No new bids after partial fill (${totalFilledAmount.toString()} filled, ${remainingAmount.toString()} remaining)`
						: `No new bids available within ${bidTimeoutMs}ms timeout`
					if (isPartiallyFilled) {
						yield { status: "PARTIAL_FILL_EXHAUSTED", commitment, totalFilledAmount, remainingAmount, error: noBidsError }
					} else {
						yield { status: "FAILED", commitment, error: noBidsError }
					}
					return
				}

				yield { status: "BIDS_RECEIVED", commitment, bidCount: freshBids.length, bids: freshBids }

				let result: SelectBidResult
				try {
					result = await this.bidManager.selectBid(order, freshBids, sessionPrivateKey)
				} catch (err) {
					yield {
						status: "FAILED",
						commitment,
						totalFilledAmount,
						remainingAmount,
						error: `Failed to select bid and submit: ${err instanceof Error ? err.message : String(err)}`,
					}
					return
				}

				const usedKey = userOpHashKey(result.userOp)
				usedUserOps.add(usedKey)
				await persistUsedUserOps()

				yield {
					status: "BID_SELECTED",
					commitment,
					selectedSolver: result.solverAddress,
					userOpHash: result.userOpHash,
					userOp: result.userOp,
				}

				yield {
					status: "USEROP_SUBMITTED",
					commitment,
					userOpHash: result.userOpHash,
					selectedSolver: result.solverAddress,
					transactionHash: result.txnHash,
				}

				if (!isSameChain) {
					return
				}

				if (result.fillStatus === "full") {
					// On a full fill, treat the order as completely satisfied
					totalFilledAmount = targetAmount
					remainingAmount = 0n

					yield {
						status: "FILLED",
						commitment,
						userOpHash: result.userOpHash,
						selectedSolver: result.solverAddress,
						transactionHash: result.txnHash,
						totalFilledAmount,
						remainingAmount,
					}
					return
				}

				if (result.fillStatus === "partial") {
					if (result.filledAmount !== undefined) {
						totalFilledAmount += result.filledAmount

						if (totalFilledAmount >= targetAmount) {
							totalFilledAmount = targetAmount
							remainingAmount = 0n
						} else {
							remainingAmount = targetAmount - totalFilledAmount
						}
					}

					if (remainingAmount === 0n) {
						yield {
							status: "FILLED",
							commitment,
							userOpHash: result.userOpHash,
							selectedSolver: result.solverAddress,
							transactionHash: result.txnHash,
							totalFilledAmount,
							remainingAmount,
						}
						return
					}

					yield {
						status: "PARTIAL_FILL",
						commitment,
						userOpHash: result.userOpHash,
						selectedSolver: result.solverAddress,
						transactionHash: result.txnHash,
						filledAmount: result.filledAmount,
						totalFilledAmount,
						remainingAmount,
					}
				}
			}
		} catch (err) {
			yield {
				status: "FAILED",
				commitment,
				error: `Unexpected error: ${err instanceof Error ? err.message : String(err)}`,
			}
		}
	}
}
