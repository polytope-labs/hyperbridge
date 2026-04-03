import type { HexString, TokenInfo, Order } from "@/types"
import type { IntentOrderStatusUpdate, ExecuteIntentOrderOptions, FillerBid, SelectBidResult } from "@/types"
import { sleep, DEFAULT_POLL_INTERVAL, hexToString } from "@/utils"
import type { IntentGatewayContext } from "./types"
import { BidManager } from "./BidManager"
// @ts-ignore
import mergeRace from "@async-generator/merge-race"

const USED_USEROPS_STORAGE_KEY = (commitment: HexString) => `used-userops:${commitment.toLowerCase()}`

/**
 * Drives the post-placement execution lifecycle of an intent order.
 *
 * After an order is placed on the source chain, `OrderExecutor` polls the
 * Hyperbridge coprocessor for solver bids, selects the best bid, submits
 * the corresponding ERC-4337 UserOperation via the bundler, and tracks
 * partial fills until the order is fully satisfied or its on-chain block
 * deadline is reached.
 *
 * Execution is structured as two racing async generators combined via
 * `mergeRace`: an `executionStream` that polls for bids and submits
 * UserOperations, and a `deadlineStream` that sleeps until the order's
 * block deadline and yields `EXPIRED`. Whichever yields first wins.
 *
 * Deduplication of UserOperations is persisted across restarts using
 * `usedUserOpsStorage` so that the executor can resume safely after a crash.
 */
export class OrderExecutor {
	constructor(
		private readonly ctx: IntentGatewayContext,
		private readonly bidManager: BidManager,
		private readonly crypto: import("./CryptoUtils").CryptoUtils,
	) {}

	/**
	 * Sleeps until the order's block deadline is reached, then yields EXPIRED.
	 * Uses the chain's block time to calculate the sleep duration.
	 */
	private async *deadlineStream(
		deadline: bigint,
		commitment: HexString,
	): AsyncGenerator<IntentOrderStatusUpdate, void> {
		const client = this.ctx.dest.client
		const blockTimeMs = client.chain?.blockTime ?? 2_000

		while (true) {
			const currentBlock = await client.getBlockNumber()
			if (currentBlock >= deadline) break

			const blocksRemaining = Number(deadline - currentBlock)
			const sleepMs = blocksRemaining * blockTimeMs
			await sleep(sleepMs)
		}

		yield {
			status: "EXPIRED",
			commitment,
			error: "Order deadline reached",
		}
	}

	/** Loads the persisted deduplication set of already-submitted UserOp hashes for a given order commitment. */
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
	 * Fetches bids from the coprocessor for a given order commitment.
	 * If a preferred solver is configured and the solver lock has not expired,
	 * only bids from that solver are returned.
	 */
	private async fetchBids(params: {
		commitment: HexString
		solver?: { address: HexString; timeoutMs: number }
		solverLockStartTime: number
	}): Promise<FillerBid[]> {
		const { commitment, solver, solverLockStartTime } = params

		const fetchedBids = await this.ctx.intentsCoprocessor!.getBidsForOrder(commitment)

		if (solver) {
			const { address, timeoutMs } = solver
			const solverLockActive = Date.now() - solverLockStartTime < timeoutMs

			return solverLockActive
				? fetchedBids.filter((bid) => bid.userOp.sender.toLowerCase() === address.toLowerCase())
				: fetchedBids
		}

		return fetchedBids
	}

	/**
	 * Selects the best bid from the provided candidates, submits the
	 * UserOperation, and persists the dedup entry to prevent resubmission.
	 */
	private async submitBid(params: {
		order: Order
		freshBids: FillerBid[]
		sessionPrivateKey?: HexString
		commitment: HexString
		usedUserOps: Set<string>
		userOpHashKey: (userOp: SelectBidResult["userOp"] | FillerBid["userOp"]) => string
	}): Promise<SelectBidResult> {
		const { order, freshBids, sessionPrivateKey, commitment, usedUserOps, userOpHashKey } = params

		const result = await this.bidManager.selectBid(order, freshBids, sessionPrivateKey)

		usedUserOps.add(userOpHashKey(result.userOp))
		await this.persistUsedUserOps(commitment, usedUserOps)

		return result
	}

	/**
	 * Processes a fill result and returns updated fill accumulators,
	 * the status update to yield (if any), and whether the order is
	 * fully satisfied.
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

			totalFilledAssets = totalFilledAssets.map((a) => {
				const filled = filledAssets.find((f) => f.token === a.token)
				return filled ? { token: a.token, amount: a.amount + filled.amount } : { ...a }
			})

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
	 * Executes an intent order by racing bid polling against the order's
	 * block deadline. Yields status updates at each lifecycle stage.
	 *
	 * **Same-chain:** `AWAITING_BIDS` → `BIDS_RECEIVED` → `BID_SELECTED`
	 *   → (`FILLED` | `PARTIAL_FILL`)* → (`FILLED` | `EXPIRED`)
	 *
	 * **Cross-chain:** `AWAITING_BIDS` → `BIDS_RECEIVED` → `BID_SELECTED`
	 *   (terminates — settlement is confirmed async via Hyperbridge)
	 */
	async *executeOrder(options: ExecuteIntentOrderOptions): AsyncGenerator<IntentOrderStatusUpdate, void> {
		const { order, sessionPrivateKey, auctionTimeMs, pollIntervalMs = DEFAULT_POLL_INTERVAL, solver } = options

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

		const targetAssets = order.output.assets.map((a) => ({ token: a.token, amount: a.amount }))
		let totalFilledAssets = order.output.assets.map((a) => ({ token: a.token, amount: 0n }))
		let remainingAssets = order.output.assets.map((a) => ({ token: a.token, amount: a.amount }))

		const executionStream = this.executionStream({
			order,
			sessionPrivateKey,
			commitment,
			auctionTimeMs,
			pollIntervalMs,
			solver,
			usedUserOps,
			userOpHashKey,
			targetAssets,
			totalFilledAssets,
			remainingAssets,
		})

		const deadlineTimeout = this.deadlineStream(order.deadline, commitment)
		const combined = mergeRace(deadlineTimeout, executionStream)

		for await (const update of combined) {
			yield update

			if (update.status === "EXPIRED" || update.status === "FILLED") return

			// Cross-chain orders terminate after submission
			if (update.status === "BID_SELECTED" && !isSameChain) return
		}
	}

	/**
	 * Core execution loop that polls for bids, submits UserOperations,
	 * and tracks fill progress. Yields between each poll iteration so
	 * that `mergeRace` can interleave the deadline stream.
	 */
	private async *executionStream(params: {
		order: Order
		sessionPrivateKey?: HexString
		commitment: HexString
		auctionTimeMs: number
		pollIntervalMs: number
		solver?: { address: HexString; timeoutMs: number }
		usedUserOps: Set<string>
		userOpHashKey: (userOp: SelectBidResult["userOp"] | FillerBid["userOp"]) => string
		targetAssets: TokenInfo[]
		totalFilledAssets: TokenInfo[]
		remainingAssets: TokenInfo[]
	}): AsyncGenerator<IntentOrderStatusUpdate, void> {
		const {
			order,
			sessionPrivateKey,
			commitment,
			auctionTimeMs,
			pollIntervalMs,
			solver,
			usedUserOps,
			userOpHashKey,
			targetAssets,
		} = params
		let { totalFilledAssets, remainingAssets } = params

		const solverLockStartTime = Date.now()
		yield { status: "AWAITING_BIDS", commitment, totalFilledAssets, remainingAssets }

		try {
			// Wait for auction time to collect bids
			const auctionEnd = Date.now() + auctionTimeMs
			while (Date.now() < auctionEnd) {
				await sleep(Math.min(pollIntervalMs, auctionEnd - Date.now()))
			}

			while (true) {
				let freshBids: FillerBid[]
				try {
					const bids = await this.fetchBids({ commitment, solver, solverLockStartTime })
					freshBids = bids.filter((bid) => !usedUserOps.has(userOpHashKey(bid.userOp)))
				} catch {
					await sleep(pollIntervalMs)
					continue
				}

				if (freshBids.length === 0) {
					await sleep(pollIntervalMs)
					continue
				}

				yield { status: "BIDS_RECEIVED", commitment, bidCount: freshBids.length, bids: freshBids }

				let submitResult: SelectBidResult
				try {
					submitResult = await this.submitBid({
						order,
						freshBids,
						sessionPrivateKey,
						commitment,
						usedUserOps,
						userOpHashKey,
					})
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

				yield {
					status: "BID_SELECTED",
					commitment,
					selectedSolver: submitResult.solverAddress,
					userOpHash: submitResult.userOpHash,
					userOp: submitResult.userOp,
					transactionHash: submitResult.txnHash,
				}

				const fill = this.processFillResult(
					submitResult,
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
		} catch (err) {
			yield {
				status: "FAILED",
				commitment,
				error: `Unexpected error: ${err instanceof Error ? err.message : String(err)}`,
			}
		}
	}
}
