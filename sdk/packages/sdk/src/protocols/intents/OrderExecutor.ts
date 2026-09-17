import type { HexString, Order, TokenInfo } from "@/types"
import type { ExecuteIntentOrderOptions, FillerBid, IntentOrderStatusUpdate, SelectBidResult } from "@/types"
import { DEFAULT_POLL_INTERVAL, normalizeStateMachineId } from "@/utils"
import type { BidManager } from "./BidManager"
import { CryptoUtils } from "./CryptoUtils"
import type { IntentGatewayContext } from "./types"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import { BidExecutionPendingError, BidExecutionRejectedError, BidImpl } from "./Bid"
import { SubmissionJournal } from "./submissionJournal"
import EntryPoint from "@/abis/entrypoint"

const USED_USEROPS_STORAGE_KEY = (commitment: HexString) => `used-userops:${commitment.toLowerCase()}`
const ABORTED = Symbol("order-execution-aborted")

function waitUnlessAborted<T>(promise: Promise<T>, signal: AbortSignal): Promise<T | typeof ABORTED> {
	if (signal.aborted) return Promise.resolve(ABORTED)

	return new Promise((resolve, reject) => {
		let settled = false
		const finish = (callback: () => void) => {
			if (settled) return
			settled = true
			signal.removeEventListener("abort", onAbort)
			callback()
		}
		const onAbort = () => finish(() => resolve(ABORTED))
		signal.addEventListener("abort", onAbort, { once: true })
		promise.then(
			(value) => finish(() => resolve(value)),
			(error) => finish(() => reject(error)),
		)
	})
}

function sleepUnlessAborted(ms: number, signal: AbortSignal): Promise<boolean> {
	if (signal.aborted) return Promise.resolve(false)

	return new Promise((resolve) => {
		const timeout = setTimeout(() => {
			signal.removeEventListener("abort", onAbort)
			resolve(true)
		}, ms)
		const onAbort = () => {
			clearTimeout(timeout)
			signal.removeEventListener("abort", onAbort)
			resolve(false)
		}
		signal.addEventListener("abort", onAbort, { once: true })
	})
}

export async function isUserOperationNonceConsumed(
	client: { readContract: (args: unknown) => Promise<unknown> },
	entryPoint: HexString,
	userOp: FillerBid["userOp"],
): Promise<boolean> {
	const key = userOp.nonce >> 64n
	const current = BigInt(
		await client.readContract({
			address: entryPoint,
			abi: EntryPoint.ABI,
			functionName: "getNonce",
			args: [userOp.sender, key],
		}),
	)
	return current > userOp.nonce
}

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
		signal: AbortSignal,
	): AsyncGenerator<IntentOrderStatusUpdate, void> {
		const client = this.ctx.dest.client
		const blockTimeMs = client.chain?.blockTime ?? 2_000

		while (!signal.aborted) {
			const currentBlock = await waitUnlessAborted(client.getBlockNumber(), signal)
			if (currentBlock === ABORTED) return
			if (currentBlock >= deadline) break

			const blocksRemaining = Number(deadline - currentBlock)
			const sleepMs = Math.min(blocksRemaining * blockTimeMs, 60_000)
			if (!(await sleepUnlessAborted(sleepMs, signal))) return
		}
		if (signal.aborted) return

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
				const parsed = JSON.parse(persisted)
				if (
					!Array.isArray(parsed) ||
					parsed.some((key) => typeof key !== "string" || !/^0x[0-9a-fA-F]{64}$/.test(key))
				)
					throw new Error("Invalid terminal hashes")
				for (const key of parsed) usedUserOps.add(key.toLowerCase())
			} catch {
				throw new Error("Terminal submission storage is corrupt; restore it before resuming")
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

	/** Reads cumulative credited progress so restarts and concurrent fillers do not rely on stale local totals. */
	private async readCreditedProgress(
		order: Order,
		commitment: HexString,
		fallback: TokenInfo[],
	): Promise<TokenInfo[]> {
		const client = this.ctx.dest.client as any
		if (typeof client.readContract !== "function") return fallback
		const gateway = this.ctx.dest.configService.getIntentGatewayAddress(normalizeStateMachineId(order.destination))
		return Promise.all(
			order.output.assets.map(async (asset, index) => {
				const credited = BigInt(
					await client.readContract({
						address: gateway,
						abi: IntentGatewayV2ABI,
						functionName: "_partialFills",
						args: [commitment, asset.token],
					}),
				)
				const previous = fallback[index]?.amount ?? 0n
				return { token: asset.token, amount: credited > previous ? credited : previous }
			}),
		)
	}

	private async readFinalizer(order: Order, commitment: HexString): Promise<HexString | undefined> {
		const client = this.ctx.dest.client as any
		if (typeof client.readContract !== "function") return undefined
		const gateway = this.ctx.dest.configService.getIntentGatewayAddress(normalizeStateMachineId(order.destination))
		const finalizer = (await client.readContract({
			address: gateway,
			abi: IntentGatewayV2ABI,
			functionName: "_filled",
			args: [commitment],
		})) as HexString
		return /^0x0{40}$/i.test(finalizer) ? undefined : finalizer
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
		const {
			order,
			sessionPrivateKey,
			auctionTimeMs,
			pollIntervalMs = DEFAULT_POLL_INTERVAL,
			solver,
			automatic,
		} = options

		const commitment = order.id as HexString

		if (!this.ctx.intentsCoprocessor) {
			yield { status: "FAILED", error: "IntentsCoprocessor required for order execution" }
			return
		}

		if (!this.ctx.bundlerUrl) {
			yield { status: "FAILED", error: "Bundler URL not configured" }
			return
		}
		let usedUserOps: Set<string>
		const userOpHashKey = this.createUserOpHasher(order)
		const entryPoint = this.ctx.dest.configService.getEntryPointV08Address(
			normalizeStateMachineId(order.destination),
		)
		const journal = new SubmissionJournal(this.ctx.usedUserOpsStorage, {
			chainId:
				this.ctx.dest.client.chain?.id ?? Number.parseInt(this.ctx.dest.config.stateMachineId.split("-")[1]),
			gateway: this.ctx.dest.configService.getIntentGatewayAddress(normalizeStateMachineId(order.destination)),
			entryPoint,
			commitment,
		})
		const retire = async (submission: SelectBidResult) => {
			try {
				usedUserOps.add(userOpHashKey(submission.userOp).toLowerCase())
				await this.persistUsedUserOps(commitment, usedUserOps)
				await journal.clear()
			} catch (error) {
				throw new BidExecutionPendingError(
					submission.userOpHash,
					`Could not retire submission durably: ${error instanceof Error ? error.message : String(error)}`,
				)
			}
		}
		let recovered: SelectBidResult | undefined
		try {
			usedUserOps = await this.loadUsedUserOps(commitment)
			const pending = await journal.read()
			if (pending) {
				const submission = pending.submission
				if (usedUserOps.has(submission.userOpHash.toLowerCase())) {
					// Crash after terminal write: no rebroadcast and no duplicate fill accounting.
					await journal.clear()
				} else {
					const crypto = new CryptoUtils(this.ctx)
					const receipt = await BidImpl.receipt(crypto, submission.userOpHash)
					if (receipt) {
						try {
							recovered = await BidImpl.verifyReceipt(this.ctx, order, submission, receipt)
						} catch (error) {
							if (!(error instanceof BidExecutionRejectedError)) throw error
						}
						await retire(submission)
					} else {
						// A finalized nonce/deadline proves this exact operation can no longer execute.
						// Providers without finalized state fail closed; latest state alone is insufficient.
						const finalized = await this.ctx.dest.client.getBlock({ blockTag: "finalized" })
						if (finalized.number === null) throw new Error("Finalized block unavailable")
						const nonce = BigInt(
							await this.ctx.dest.client.readContract({
								address: entryPoint,
								abi: EntryPoint.ABI,
								functionName: "getNonce",
								args: [submission.userOp.sender, submission.userOp.nonce >> 64n],
								blockNumber: finalized.number,
							}),
						)
						if (nonce > submission.userOp.nonce || finalized.number > order.deadline) {
							// Reconcile progress before retirement, then read again before polling below.
							await this.readCreditedProgress(order, commitment, [])
							await this.readFinalizer(order, commitment)
							await retire(submission)
						} else {
							if (
								await isUserOperationNonceConsumed(this.ctx.dest.client, entryPoint, submission.userOp)
							) {
								throw new BidExecutionPendingError(
									submission.userOpHash,
									"Nonce consumed; waiting for finalized state",
								)
							}
							await BidImpl.broadcast(crypto, submission, entryPoint, true)
							const replayReceipt = await BidImpl.receipt(crypto, submission.userOpHash)
							if (!replayReceipt)
								throw new BidExecutionPendingError(
									submission.userOpHash,
									"Rebroadcast operation is awaiting inclusion",
								)
							try {
								recovered = await BidImpl.verifyReceipt(this.ctx, order, submission, replayReceipt)
							} catch (error) {
								if (!(error instanceof BidExecutionRejectedError)) throw error
							}
							await retire(submission)
						}
					}
				}
			}
		} catch (error) {
			yield { status: "FAILED", commitment, error: error instanceof Error ? error.message : String(error) }
			return
		}

		const targetAssets = order.output.assets.map((a) => ({ token: a.token, amount: a.amount }))
		let totalFilledAssets: TokenInfo[]
		let initialFinalizer: HexString | undefined
		try {
			totalFilledAssets = await this.readCreditedProgress(
				order,
				commitment,
				order.output.assets.map((a) => ({ token: a.token, amount: 0n })),
			)
			initialFinalizer = await this.readFinalizer(order, commitment)
			if (initialFinalizer) {
				// Once finalized, progress is immutable. Reread it after the finalizer so a fill that
				// lands between the two independent latest-state reads cannot look like cancellation.
				totalFilledAssets = await this.readCreditedProgress(order, commitment, totalFilledAssets)
			}
		} catch (err) {
			yield { status: "FAILED", commitment, error: err instanceof Error ? err.message : String(err) }
			return
		}
		let remainingAssets = order.output.assets.map((a) => ({ token: a.token, amount: a.amount }))
		remainingAssets = targetAssets.map((target, index) => ({
			token: target.token,
			amount:
				(totalFilledAssets[index]?.amount ?? 0n) >= target.amount
					? 0n
					: target.amount - (totalFilledAssets[index]?.amount ?? 0n),
		}))
		if (initialFinalizer) {
			if (remainingAssets.every((asset) => asset.amount === 0n)) {
				yield {
					status: "FILLED",
					commitment,
					selectedSolver: initialFinalizer,
					totalFilledAssets,
					remainingAssets,
				}
			} else {
				yield { status: "CANCELLED", commitment, totalFilledAssets, remainingAssets }
			}
			return
		}
		if (recovered) {
			yield {
				status: recovered.fillStatus === "full" ? "FILLED" : "PARTIAL_FILL",
				commitment,
				userOpHash: recovered.userOpHash,
				selectedSolver: recovered.solverAddress,
				transactionHash: recovered.txnHash,
				filledAssets: recovered.filledAssets,
				totalFilledAssets,
				remainingAssets,
			}
			if (recovered.fillStatus === "full") return
		}

		const abortController = new AbortController()
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
			signal: abortController.signal,
		})

		const deadlineTimeout = this.deadlineStream(order.deadline, commitment, abortController.signal)
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
				if (value.status === "BIDS_RECEIVED") {
					if (automatic) {
						const executableBids =
							order.inputs.length === 1 && order.output.assets.length === 1
								? value.bids
								: value.bids.filter((bid) => bid.inputs.length === 0)
						if (executableBids.length === 0 && value.bids.some((bid) => bid.inputs.length > 0)) {
							yield {
								status: "FAILED",
								commitment,
								error: "Automatic rate execution supports single-leg orders only",
							}
							return
						}
						try {
							input = await this.bidManager.selectAndExecuteBest(
								order,
								executableBids,
								async (submission) => {
									await journal.write(submission)
								},
								retire,
							)
						} catch (err) {
							if (err instanceof BidExecutionPendingError) {
								yield { status: "FAILED", commitment, error: err.message }
								return
							}
							// The manager has exhausted this ranked batch. Let the stream poll for fresh bids.
							input = undefined
						}
					} else {
						input = fed
					}
				}
				if (value.status === "EXPIRED" || value.status === "FILLED") return
			}
		} finally {
			// Tear the streams down explicitly so neither keeps polling in the
			// background after the consumer stops iterating.
			console.log(`[OrderExecutor] Tearing down streams for commitment=${commitment}`)
			abortController.abort()
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
		signal: AbortSignal
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
			signal,
		} = params
		let { totalFilledAssets, remainingAssets } = params

		const isFreshBid = (bid: FillerBid) => !usedUserOps.has(userOpHashKey(bid.userOp))
		const entryPointAddress = this.ctx.dest.configService.getEntryPointV08Address(
			normalizeStateMachineId(order.destination),
		)

		const solverLockStartTime = Date.now()
		yield { status: "AWAITING_BIDS", commitment, totalFilledAssets, remainingAssets }

		try {
			// Poll for bids during the auction period, yielding NEW_BID for each new bid seen
			const auctionEnd = Date.now() + auctionTimeMs
			const auctionSeenBids = new Set<string>()
			while (!signal.aborted && Date.now() < auctionEnd) {
				try {
					const bids = await waitUnlessAborted(
						this.fetchBids({ commitment, solver, solverLockStartTime }),
						signal,
					)
					if (bids === ABORTED) return
					const newBids = bids.filter(
						(bid) => isFreshBid(bid) && !auctionSeenBids.has(userOpHashKey(bid.userOp)),
					)
					for (const fillerBid of newBids) {
						auctionSeenBids.add(userOpHashKey(fillerBid.userOp))
						const [bid] = this.bidManager.buildBids(order, [fillerBid], sessionPrivateKey)
						if (bid) yield { status: "NEW_BID", commitment, bid }
					}
				} catch {
					if (signal.aborted) return
					// Ignore fetch errors during auction, will retry next interval
				}
				const remaining = auctionEnd - Date.now()
				if (remaining > 0) {
					if (!(await sleepUnlessAborted(Math.min(pollIntervalMs, remaining), signal))) return
				}
			}

			while (!signal.aborted) {
				const creditedProgress = await waitUnlessAborted(
					this.readCreditedProgress(order, commitment, totalFilledAssets),
					signal,
				)
				if (creditedProgress === ABORTED) return
				totalFilledAssets = creditedProgress
				remainingAssets = targetAssets.map((target, index) => ({
					token: target.token,
					amount:
						(totalFilledAssets[index]?.amount ?? 0n) >= target.amount
							? 0n
							: target.amount - (totalFilledAssets[index]?.amount ?? 0n),
				}))
				const finalizer = await waitUnlessAborted(this.readFinalizer(order, commitment), signal)
				if (finalizer === ABORTED) return
				if (finalizer) {
					const finalizedProgress = await waitUnlessAborted(
						this.readCreditedProgress(order, commitment, totalFilledAssets),
						signal,
					)
					if (finalizedProgress === ABORTED) return
					totalFilledAssets = finalizedProgress
					remainingAssets = targetAssets.map((target, index) => ({
						token: target.token,
						amount:
							(totalFilledAssets[index]?.amount ?? 0n) >= target.amount
								? 0n
								: target.amount - (totalFilledAssets[index]?.amount ?? 0n),
					}))
					if (remainingAssets.every((asset) => asset.amount === 0n)) {
						yield {
							status: "FILLED",
							commitment,
							selectedSolver: finalizer,
							totalFilledAssets,
							remainingAssets,
						}
					} else {
						yield { status: "CANCELLED", commitment, totalFilledAssets, remainingAssets }
					}
					return
				}
				let freshBids: FillerBid[]
				try {
					const bids = await waitUnlessAborted(
						this.fetchBids({ commitment, solver, solverLockStartTime }),
						signal,
					)
					if (bids === ABORTED) return
					const unseen = bids.filter(isFreshBid)
					const client = this.ctx.dest.client as any
					const checkedBids =
						typeof client.readContract !== "function"
							? Promise.resolve(unseen)
							: Promise.all(
									unseen.map(async (bid) => ({
										bid,
										consumed: await isUserOperationNonceConsumed(
											client,
											entryPointAddress,
											bid.userOp,
										),
									})),
								).then((results) => results.filter(({ consumed }) => !consumed).map(({ bid }) => bid))
					const availableBids = await waitUnlessAborted(checkedBids, signal)
					if (availableBids === ABORTED) return
					freshBids = availableBids
				} catch {
					if (signal.aborted) return
					if (!(await sleepUnlessAborted(pollIntervalMs, signal))) return
					continue
				}

				if (freshBids.length === 0) {
					if (!(await sleepUnlessAborted(pollIntervalMs, signal))) return
					continue
				}

				const bids = this.bidManager.buildBids(order, freshBids, sessionPrivateKey)
				if (bids.length === 0) {
					if (!(await sleepUnlessAborted(pollIntervalMs, signal))) return
					continue
				}

				// Hand the bids to the consumer and wait for them to execute one.
				const result = yield { status: "BIDS_RECEIVED", commitment, bidCount: bids.length, bids }

				if (!result) {
					// Consumer did not execute a bid this round; poll again.
					if (!(await sleepUnlessAborted(pollIntervalMs, signal))) return
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
