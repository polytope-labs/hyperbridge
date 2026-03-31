import type { HexString } from "@/types"
import type { IntentOrderStatusUpdate, ExecuteIntentOrderOptions, FillerBid, SelectBidResult } from "@/types"
import { sleep, DEFAULT_POLL_INTERVAL, hexToString } from "@/utils"
import type { IntentGatewayContext } from "./types"
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
		private readonly ctx: IntentGatewayContext,
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
	 * → (`FILLED` | `PARTIAL_FILL`)* → (`FILLED` | `EXPIRED`)
	 *
	 * **Error statuses:** `FAILED` (retryable error during bid selection/submission,
	 * triggers automatic retry) or `EXPIRED` (deadline reached or no new bids —
	 * terminal, no further retries).
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
			solver,
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

		// For partial fill tracking, initialise per-token accumulators from order.output.assets
		const targetAssets = order.output.assets.map((a) => ({ token: a.token, amount: a.amount }))

		let totalFilledAssets = order.output.assets.map((a) => ({ token: a.token, amount: 0n }))
		let remainingAssets = order.output.assets.map((a) => ({ token: a.token, amount: a.amount }))

		try {
			while (true) {
				const currentBlock = await this.ctx.dest.client.getBlockNumber()
				if (currentBlock >= order.deadline) {
					const deadlineError = `Order deadline reached (block ${currentBlock} >= ${order.deadline})`
					yield {
						status: "EXPIRED",
						commitment,
						totalFilledAssets,
						remainingAssets,
						error: deadlineError,
					}
					return
				}

				yield { status: "AWAITING_BIDS", commitment, totalFilledAssets, remainingAssets }

				const startTime = Date.now()
				let bids: FillerBid[] = []
				let solverLockExpired = false

				while (Date.now() - startTime < bidTimeoutMs) {
					try {
						const fetchedBids = await this.ctx.intentsCoprocessor!.getBidsForOrder(commitment)

						if (solver) {
							const { address, timeoutMs } = solver
							const solverLockActive = Date.now() - startTime < timeoutMs
							if (!solverLockActive) solverLockExpired = true

							bids = solverLockActive
								? fetchedBids.filter((bid) => bid.userOp.sender.toLowerCase() === address.toLowerCase())
								: fetchedBids
						} else {
							bids = fetchedBids
						}

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
					const solverClause = solver && !solverLockExpired ? ` for requested solver ${solver.address}` : ""
					const isPartiallyFilled = totalFilledAssets.some((a) => a.amount > 0n)
					const noBidsError = isPartiallyFilled
						? `No new bids${solverClause} after partial fill`
						: `No new bids${solverClause} available within ${bidTimeoutMs}ms timeout`

					yield {
						status: "EXPIRED",
						commitment,
						totalFilledAssets,
						remainingAssets,
						error: noBidsError,
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
						totalFilledAssets,
						remainingAssets,
						error: `Failed to select bid and submit: ${err instanceof Error ? err.message : String(err)}`,
					}
					// Back off before retrying
					await sleep(pollIntervalMs)
					continue
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
					totalFilledAssets = targetAssets.map((a) => ({ token: a.token, amount: a.amount }))
					remainingAssets = targetAssets.map((a) => ({ token: a.token, amount: 0n }))

					yield {
						status: "FILLED",
						commitment,
						userOpHash: result.userOpHash,
						selectedSolver: result.solverAddress,
						transactionHash: result.txnHash,
						totalFilledAssets,
						remainingAssets,
					}
					return
				}

				if (result.fillStatus === "partial") {
					const filledAssets = result.filledAssets ?? []

					// Accumulate per-token filled amounts
					for (const filled of filledAssets) {
						const entry = totalFilledAssets.find((a) => a.token === filled.token)
						if (entry) entry.amount += filled.amount
					}

					// Recompute remaining per-token
					remainingAssets = targetAssets.map((target) => {
						const filled = totalFilledAssets.find((a) => a.token === target.token)
						const filledAmt = filled?.amount ?? 0n
						return {
							token: target.token,
							amount: filledAmt >= target.amount ? 0n : target.amount - filledAmt,
						}
					})

					const fullyFilled = remainingAssets.every((a) => a.amount === 0n)
					if (fullyFilled) {
						yield {
							status: "FILLED",
							commitment,
							userOpHash: result.userOpHash,
							selectedSolver: result.solverAddress,
							transactionHash: result.txnHash,
							totalFilledAssets,
							remainingAssets,
						}
						return
					}

					yield {
						status: "PARTIAL_FILL",
						commitment,
						userOpHash: result.userOpHash,
						selectedSolver: result.solverAddress,
						transactionHash: result.txnHash,
						filledAssets,
						totalFilledAssets,
						remainingAssets,
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
