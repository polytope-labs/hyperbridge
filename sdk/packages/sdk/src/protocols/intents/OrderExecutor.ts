import type { HexString, Order, TokenInfo } from "@/types"
import type { ExecuteIntentOrderOptions, FillerBid, IntentOrderStatusUpdate, SelectBidResult } from "@/types"
import { DEFAULT_POLL_INTERVAL, normalizeStateMachineId, sleep } from "@/utils"
import type { BidManager } from "./BidManager"
import { CryptoUtils } from "./CryptoUtils"
import type { IntentGatewayContext } from "./types"

const USED_USEROPS_STORAGE_KEY = (commitment: HexString) => `used-userops:${commitment.toLowerCase()}`

/**
 * Drives the post-placement execution lifecycle of an intent order.
 *
 * After an order is placed on the source chain, `OrderExecutor` polls the
 * Hyperbridge coprocessor for solver bids, selects the best bid, submits
 * the corresponding ERC-4337 UserOperation via the bundler, and tracks
 * partial fills until the order is fully satisfied or its on-chain block
 * deadline is reached. Cross-chain fills are confirmed from the destination
 * chain `OrderFilled` log returned by the executed bid.
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
			const sleepMs = Math.min(blocksRemaining * blockTimeMs, 60_000)
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
		destination: string
	}): (userOp: SelectBidResult["userOp"] | FillerBid["userOp"]) => string {
		const entryPointAddress = this.ctx.dest.configService.getEntryPointV08Address(
			normalizeStateMachineId(order.destination),
		)
		const chainId = BigInt(
			this.ctx.dest.client.chain?.id ?? Number.parseInt(this.ctx.dest.config.stateMachineId.split("-")[1]),
		)
		return (userOp) => CryptoUtils.computeUserOpHash(userOp, entryPointAddress, chainId)
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

		const intentsCoprocessor = this.ctx.intentsCoprocessor
		if (!intentsCoprocessor) {
			throw new Error("IntentsCoprocessor required for order execution")
		}

		const fetchedBids = await intentsCoprocessor.getBidsForOrder(commitment)

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
	 * block deadline. Yields status updates at each lifecycle stage and hands
	 * bid selection to the consumer.
	 *
	 * This is a **bidirectional** generator: when it yields `BIDS_RECEIVED`, the
	 * consumer picks a bid, calls `bid.execute()`, and feeds the resulting
	 * {@link SelectBidResult} back via `gen.next(result)`. The generator then
	 * records the dedup entry, emits `BID_SELECTED`, tracks the fill, and either
	 * terminates or continues polling for the remaining amount. Feeding back
	 * `undefined` (no bid executed this round) causes it to keep polling.
	 *
	 * **Same-chain:** `AWAITING_BIDS` → `BIDS_RECEIVED` → `BID_SELECTED`
	 *   → (`FILLED` | `PARTIAL_FILL`)* → (`FILLED` | `EXPIRED`)
	 *
	 * **Cross-chain:** `AWAITING_BIDS` → `BIDS_RECEIVED` → `BID_SELECTED`
	 *   → `FILLED`
	 */
	async *executeOrder(
		options: ExecuteIntentOrderOptions,
	): AsyncGenerator<IntentOrderStatusUpdate, void, SelectBidResult | undefined> {
		const { order, sessionPrivateKey, auctionTimeMs, pollIntervalMs = DEFAULT_POLL_INTERVAL, solver } = options

		const commitment = order.id as HexString

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
		// The deadline stream resolves once, when the order's block deadline is
		// reached. We race every execution-stream step against it.
		const deadlinePromise = deadlineTimeout.next()

		try {
			// Drive the execution stream manually so we can forward the consumer's
			// fed-back SelectBidResult into it (the bidirectional handshake), while
			// racing each step against the deadline. We cannot use a merge helper
			// here because those do not forward values passed to `.next()`.
			let input: SelectBidResult | undefined
			while (true) {
				// When we are delivering a fed-back result (the consumer already
				// executed a bid), process it without racing the deadline so an
				// already-submitted UserOp is never dropped by a deadline that
				// elapsed while the consumer was busy in `bid.execute()`. Only
				// poll steps (no pending result) race against the deadline.
				const winner =
					input !== undefined
						? { from: "exec" as const, r: await executionStream.next(input) }
						: await Promise.race([
								executionStream.next(undefined).then((r) => ({ from: "exec" as const, r })),
								deadlinePromise.then((r) => ({ from: "deadline" as const, r })),
							])
				input = undefined

				if (winner.from === "deadline") {
					if (!winner.r.done && winner.r.value) yield winner.r.value
					return
				}

				const { value, done } = winner.r
				if (done) return

				const fed = yield value
				if (value.status === "BIDS_RECEIVED") input = fed
				if (value.status === "EXPIRED" || value.status === "FILLED") return
			}
		} finally {
			// Tear the streams down explicitly so neither keeps polling in the
			// background after the consumer stops iterating.
			console.log(`[OrderExecutor] Tearing down streams for commitment=${commitment}`)
			await executionStream.return(undefined as never)
			await deadlineTimeout.return(undefined as never)
		}
	}

	/**
	 * Core execution loop that polls for bids and tracks fill progress. Builds
	 * first-class {@link Bid} objects from the raw filler bids and yields them to
	 * the consumer, which picks one, calls `bid.execute()`, and feeds the result
	 * back via `gen.next(result)`. The loop then records the dedup entry, emits
	 * `BID_SELECTED`, processes the fill, and continues polling for the remaining
	 * amount on partial fills.
	 *
	 * Bidirectional: the value passed to `.next()` after a `BIDS_RECEIVED` yield is
	 * the {@link SelectBidResult} from the executed bid (or `undefined` to skip the
	 * round and keep polling).
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
	}): AsyncGenerator<IntentOrderStatusUpdate, void, SelectBidResult | undefined> {
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

		const isFreshBid = (bid: FillerBid) => !usedUserOps.has(userOpHashKey(bid.userOp))

		const solverLockStartTime = Date.now()
		yield { status: "AWAITING_BIDS", commitment, totalFilledAssets, remainingAssets }

		try {
			// Poll for bids during the auction period, yielding NEW_BID for each new bid seen
			const auctionEnd = Date.now() + auctionTimeMs
			const auctionSeenBids = new Set<string>()
			while (Date.now() < auctionEnd) {
				try {
					const bids = await this.fetchBids({ commitment, solver, solverLockStartTime })
					const newBids = bids.filter(
						(bid) => isFreshBid(bid) && !auctionSeenBids.has(userOpHashKey(bid.userOp)),
					)
					for (const fillerBid of newBids) {
						auctionSeenBids.add(userOpHashKey(fillerBid.userOp))
						const [bid] = this.bidManager.buildBids(order, [fillerBid], sessionPrivateKey)
						if (bid) yield { status: "NEW_BID", commitment, bid }
					}
				} catch {
					// Ignore fetch errors during auction, will retry next interval
				}
				const remaining = auctionEnd - Date.now()
				if (remaining > 0) {
					await sleep(Math.min(pollIntervalMs, remaining))
				}
			}

			while (true) {
				let freshBids: FillerBid[]
				try {
					const bids = await this.fetchBids({ commitment, solver, solverLockStartTime })
					freshBids = bids.filter(isFreshBid)
				} catch {
					await sleep(pollIntervalMs)
					continue
				}

				if (freshBids.length === 0) {
					await sleep(pollIntervalMs)
					continue
				}

				const bids = this.bidManager.buildBids(order, freshBids, sessionPrivateKey)
				if (bids.length === 0) {
					await sleep(pollIntervalMs)
					continue
				}

				// Hand the bids to the consumer and wait for them to execute one.
				const result = yield { status: "BIDS_RECEIVED", commitment, bidCount: bids.length, bids }

				if (!result) {
					// Consumer did not execute a bid this round; poll again.
					await sleep(pollIntervalMs)
					continue
				}

				usedUserOps.add(userOpHashKey(result.userOp))
				await this.persistUsedUserOps(commitment, usedUserOps)

				yield {
					status: "BID_SELECTED",
					commitment,
					selectedSolver: result.solverAddress,
					userOpHash: result.userOpHash,
					userOp: result.userOp,
					transactionHash: result.txnHash,
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
				}
				if (fill.done) return
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
