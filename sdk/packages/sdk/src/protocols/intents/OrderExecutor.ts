import type { HexString, TokenInfo, Order } from "@/types"
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
 * Estimated average block time in milliseconds, used to decide whether the
 * deadline is far enough away that a `getBlockNumber` RPC call can be skipped
 * on a given poll iteration. Conservative (low) so that the check fires
 * before the actual deadline rather than after.
 */
const ESTIMATED_BLOCK_TIME_MS = 2_000

/**
 * Drives the post-placement execution lifecycle of an IntentGatewayV2 order.
 *
 * After an order is placed on the source chain, `OrderExecutor` polls the
 * Hyperbridge coprocessor for solver bids, selects and validates the best
 * bid, submits the corresponding ERC-4337 UserOperation via the bundler, and
 * tracks partial fills until the order is fully satisfied or its on-chain
 * block deadline is reached. Polling is bounded solely by the order's
 * deadline — there is no separate wall-clock timeout.
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
	 * Cached block number and the wall-clock time it was fetched, used by
	 * {@link isDeadlineReached} to skip redundant `getBlockNumber` RPC calls
	 * when the deadline is estimated to be far away.
	 */
	private lastBlockCheck: { block: bigint; checkedAt: number } | undefined

	/**
	 * Checks whether the order's block deadline has been reached. Caches the
	 * last-seen block number and skips the RPC call when the deadline is
	 * estimated to be more than 10 blocks away based on wall-clock time
	 * elapsed since the last check.
	 */
	private async isDeadlineReached(deadline: bigint): Promise<boolean> {
		const now = Date.now()

		if (this.lastBlockCheck) {
			const elapsed = now - this.lastBlockCheck.checkedAt
			const estimatedBlocksElapsed = BigInt(Math.floor(elapsed / ESTIMATED_BLOCK_TIME_MS))
			const estimatedCurrentBlock = this.lastBlockCheck.block + estimatedBlocksElapsed
			if (estimatedCurrentBlock + 10n < deadline) {
				return false
			}
		}

		const currentBlock = await this.ctx.dest.client.getBlockNumber()
		this.lastBlockCheck = { block: currentBlock, checkedAt: now }
		return currentBlock >= deadline
	}

	/**
	 * Loads the persisted deduplication set of already-submitted UserOp hashes
	 * for a given order commitment.
	 */
	private async loadUsedUserOps(commitment: HexString): Promise<Set<string>> {
		const usedUserOps = new Set<string>()
		const persisted = await this.ctx.usedUserOpsStorage.getItem(USED_USEROPS_STORAGE_KEY(commitment))
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
		return usedUserOps
	}

	/** Persists the deduplication set of UserOp hashes to storage. */
	private async persistUsedUserOps(commitment: HexString, usedUserOps: Set<string>): Promise<void> {
		await this.ctx.usedUserOpsStorage.setItem(
			USED_USEROPS_STORAGE_KEY(commitment),
			JSON.stringify([...usedUserOps]),
		)
	}

	/**
	 * Creates a closure that computes the deduplication hash key for a
	 * UserOperation, pre-bound to the order's destination chain and entry point.
	 */
	private createUserOpHasher(order: {
		destination: HexString
	}): (userOp: SelectBidResult["userOp"] | FillerBid["userOp"]) => string {
		const entryPointAddress = this.ctx.dest.configService.getEntryPointV08Address(hexToString(order.destination))
		const chainId = BigInt(
			this.ctx.dest.client.chain?.id ?? Number.parseInt(this.ctx.dest.config.stateMachineId.split("-")[1]),
		)
		return (userOp) => this.crypto.computeUserOpHash(userOp, entryPointAddress, chainId)
	}

	/**
	 * Polls the coprocessor for bids until either enough bids are collected
	 * or the order's block deadline is reached.
	 */
	private async pollForBids(params: {
		commitment: HexString
		isOrderExpired: () => Promise<boolean>
		minBids: number
		pollIntervalMs: number
		solver?: { address: HexString; timeoutMs: number }
	}): Promise<{ bids: FillerBid[]; solverLockExpired: boolean }> {
		const { commitment, isOrderExpired, minBids, pollIntervalMs, solver } = params
		const startTime = Date.now()
		let bids: FillerBid[] = []
		let solverLockExpired = false

		while (!(await isOrderExpired())) {
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

		return { bids, solverLockExpired }
	}

	/**
	 * Selects the best bid, submits the UserOp, and persists the dedup entry.
	 * Yields `BID_SELECTED` and `USEROP_SUBMITTED` status updates.
	 */
	private async *submitBid(params: {
		order: Order
		freshBids: FillerBid[]
		sessionPrivateKey?: HexString
		commitment: HexString
		usedUserOps: Set<string>
		userOpHashKey: (userOp: SelectBidResult["userOp"] | FillerBid["userOp"]) => string
	}): AsyncGenerator<IntentOrderStatusUpdate, SelectBidResult> {
		const { order, freshBids, sessionPrivateKey, commitment, usedUserOps, userOpHashKey } = params

		const result = await this.bidManager.selectBid(order, freshBids, sessionPrivateKey)

		usedUserOps.add(userOpHashKey(result.userOp))
		await this.persistUsedUserOps(commitment, usedUserOps)

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

		return result
	}

	/**
	 * Processes a fill result, updating the fill accumulators in-place and
	 * returning the status update to yield (if any) and whether the order
	 * is fully satisfied.
	 */
	private processFillResult(
		result: SelectBidResult,
		commitment: HexString,
		targetAssets: TokenInfo[],
		totalFilledAssets: TokenInfo[],
		remainingAssets: TokenInfo[],
	): {
		update: IntentOrderStatusUpdate | null
		done: boolean
		totalFilledAssets: TokenInfo[]
		remainingAssets: TokenInfo[]
	} {
		if (result.fillStatus === "full") {
			totalFilledAssets = targetAssets.map((a) => ({ token: a.token, amount: a.amount }))
			remainingAssets = targetAssets.map((a) => ({ token: a.token, amount: 0n }))

			return {
				update: {
					status: "FILLED",
					commitment,
					userOpHash: result.userOpHash,
					selectedSolver: result.solverAddress,
					transactionHash: result.txnHash,
					totalFilledAssets,
					remainingAssets,
				},
				done: true,
				totalFilledAssets,
				remainingAssets,
			}
		}

		if (result.fillStatus === "partial") {
			const filledAssets = result.filledAssets ?? []

			for (const filled of filledAssets) {
				const entry = totalFilledAssets.find((a) => a.token === filled.token)
				if (entry) entry.amount += filled.amount
			}

			remainingAssets = targetAssets.map((target) => {
				const filled = totalFilledAssets.find((a) => a.token === target.token)
				const filledAmt = filled?.amount ?? 0n
				return {
					token: target.token,
					amount: filledAmt >= target.amount ? 0n : target.amount - filledAmt,
				}
			})

			const fullyFilled = remainingAssets.every((a) => a.amount === 0n)

			return {
				update: fullyFilled
					? {
							status: "FILLED",
							commitment,
							userOpHash: result.userOpHash,
							selectedSolver: result.solverAddress,
							transactionHash: result.txnHash,
							totalFilledAssets,
							remainingAssets,
						}
					: {
							status: "PARTIAL_FILL",
							commitment,
							userOpHash: result.userOpHash,
							selectedSolver: result.solverAddress,
							transactionHash: result.txnHash,
							filledAssets,
							totalFilledAssets,
							remainingAssets,
						},
				done: fullyFilled,
				totalFilledAssets,
				remainingAssets,
			}
		}

		return { update: null, done: false, totalFilledAssets, remainingAssets }
	}

	/**
	 * Async generator that executes an intent order by polling for bids and
	 * submitting UserOperations until the order is filled, partially exhausted,
	 * or the on-chain deadline is reached.
	 *
	 * Bid polling is bounded by the order's block deadline (`order.deadline`),
	 * which is checked before every poll iteration. There is no separate
	 * wall-clock timeout — the order's on-chain lifetime is the single
	 * source of truth for expiry.
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
		const { order, sessionPrivateKey, minBids = 1, pollIntervalMs = DEFAULT_POLL_INTERVAL, solver } = options

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

		const usedUserOps = await this.loadUsedUserOps(commitment)
		const userOpHashKey = this.createUserOpHasher(order)

		// For partial fill tracking, initialise per-token accumulators from order.output.assets
		const targetAssets = order.output.assets.map((a) => ({ token: a.token, amount: a.amount }))
		let totalFilledAssets = order.output.assets.map((a) => ({ token: a.token, amount: 0n }))
		let remainingAssets = order.output.assets.map((a) => ({ token: a.token, amount: a.amount }))

		this.lastBlockCheck = undefined
		const isOrderExpired = () => this.isDeadlineReached(order.deadline)

		try {
			while (!(await isOrderExpired())) {
				yield { status: "AWAITING_BIDS", commitment, totalFilledAssets, remainingAssets }

				const { bids, solverLockExpired } = await this.pollForBids({
					commitment,
					isOrderExpired,
					minBids,
					pollIntervalMs,
					solver,
				})

				const freshBids = bids.filter((bid) => !usedUserOps.has(userOpHashKey(bid.userOp)))

				if (freshBids.length === 0) {
					const solverClause = solver && !solverLockExpired ? ` for requested solver ${solver.address}` : ""
					const isPartiallyFilled = totalFilledAssets.some((a) => a.amount > 0n)
					const noBidsError = isPartiallyFilled
						? `No new bids${solverClause} after partial fill`
						: `No new bids${solverClause} for order`

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
					const gen = this.submitBid({
						order,
						freshBids,
						sessionPrivateKey,
						commitment,
						usedUserOps,
						userOpHashKey,
					})
					let step = await gen.next()
					while (!step.done) {
						yield step.value
						step = await gen.next()
					}
					result = step.value
				} catch (err) {
					yield {
						status: "FAILED",
						commitment,
						totalFilledAssets,
						remainingAssets,
						error: `Failed to select bid and submit: ${err instanceof Error ? err.message : String(err)}`,
					}
					await sleep(pollIntervalMs)
					continue
				}

				if (!isSameChain) {
					return
				}

				const fill = this.processFillResult(
					result,
					commitment,
					targetAssets,
					totalFilledAssets,
					remainingAssets,
				)
				totalFilledAssets = fill.totalFilledAssets
				remainingAssets = fill.remainingAssets

				if (fill.update) {
					yield fill.update
					if (fill.done) return
				}
			}

			yield {
				status: "EXPIRED",
				commitment,
				totalFilledAssets,
				remainingAssets,
				error: "Order deadline reached",
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
