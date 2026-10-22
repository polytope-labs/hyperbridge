import type { BidSubmissionResult, HexString, IntentsCoprocessor } from "@hyperbridge/sdk"
import type { OrderbookClient } from "@/orderbook/client"
import type { SubmitBidResult } from "@/orderbook/types"
import type { Logger } from "@/services/Logger"

/**
 * Where a bid goes. A strategy builds the signed fill op and hands it here; the venues decide
 * who receives it. The filler passes one of these to `FillerStrategy.executeOrder`.
 */
export interface BidPlacer {
	/**
	 * @param commitment - The swap order the bid is on.
	 * @param userOp - The signed fill UserOp, SCALE-encoded: the same bytes every venue takes.
	 * @param bid - The identifier Hyperbridge files the bid under, `keccak256` of its calldata.
	 */
	place(commitment: HexString, userOp: HexString, bid: HexString): Promise<BidPlacement>
}

/** What each venue said. A venue the bid was not sent to is absent. */
export interface BidPlacement {
	/** Hyperbridge's answer: the extrinsic landed, is pooled, or failed. */
	hyperbridge?: BidSubmissionResult
	/** The HyperFX orderbook's answer. */
	orderbook?: SubmitBidResult
}

/**
 * How long a reading of the filler's BRIDGE balance stands before it is read again. Bids are
 * placed in bursts, and each one that reaches Hyperbridge lowers the balance by what it
 * reserves and pays, so the reading is also lowered locally after each placement.
 */
export const BRIDGE_BALANCE_TTL_MS = 30_000

/**
 * Whether the filler holds the BRIDGE a bid on Hyperbridge costs.
 *
 * `place_bid` reserves `StorageDepositFee` and the extrinsic pays its fee out of the free
 * balance, which must also stay above the existential deposit. A filler without that much
 * cannot bid on Hyperbridge: with nothing for the fee the extrinsic is refused at the pool,
 * which places nothing and costs nothing, and with the fee but not the deposit it is included,
 * fails with `InsufficientBalance`, and pays the fee for nothing (`place_bid` is not `Pays::No`).
 * So the balance is read before each bid and, when it falls short, the bid goes to the orderbook
 * alone.
 *
 * The reading is cached for {@link BRIDGE_BALANCE_TTL_MS} and lowered by one bid's cost after
 * each placement that reached Hyperbridge, so a burst of bids does not overrun a balance the
 * next read has not caught up with. A placement Hyperbridge refuses for want of balance
 * zeroes the reading until the next read. The read is over the coprocessor's query API; a read
 * that fails leaves the last reading in place, or, with none, lets the bid through to
 * Hyperbridge, and is not tried again before the TTL, so a dead endpoint costs one wait per
 * TTL rather than one per bid.
 *
 * The gate is long-lived, one per filler, so it is also where a change of answer is logged:
 * once at `warn` when bids start going to the orderbook alone, once at `info` when they go to
 * Hyperbridge again.
 */
export class BridgeBalanceGate {
	private free?: bigint
	private readAt = 0
	private cost?: bigint
	private read?: Promise<void>
	private hadBalance?: boolean

	constructor(
		private readonly hyperbridge: IntentsCoprocessor,
		private readonly logger: Logger,
		private readonly now: () => number = Date.now,
	) {}

	/** Whether the account can afford one more bid on Hyperbridge. */
	async canPlaceBid(): Promise<boolean> {
		if (this.now() - this.readAt > BRIDGE_BALANCE_TTL_MS) {
			this.read ??= this.refresh().finally(() => {
				this.read = undefined
			})
			await this.read
		}
		const can = this.free === undefined || this.cost === undefined || this.free >= this.cost
		if (!can && this.hadBalance !== false) {
			this.logger.warn(
				{ free: this.free?.toString(), cost: this.cost?.toString() },
				"No BRIDGE for a bid deposit; bids go to the orderbook alone until the balance is topped up",
			)
		} else if (can && this.hadBalance === false) {
			this.logger.info("BRIDGE balance restored; bids go to Hyperbridge and the orderbook again")
		}
		this.hadBalance = can
		return can
	}

	/**
	 * Lowers the reading after a bid reached Hyperbridge (landed or pooled): its deposit is
	 * reserved and its fee paid, before the next read sees either.
	 */
	spent(): void {
		if (this.free !== undefined && this.cost !== undefined) {
			this.free = this.free > this.cost ? this.free - this.cost : 0n
		}
	}

	/** Hyperbridge refused a bid for want of balance: nothing more goes there until the next read. */
	exhausted(): void {
		this.free = 0n
		this.readAt = this.now()
	}

	/** The free balance in BRIDGE's smallest unit, as last read; undefined before the first read. */
	get freeBalance(): bigint | undefined {
		return this.free
	}

	private async refresh(): Promise<void> {
		// Whatever the read says, it stands for the TTL: a dead endpoint is waited out once, not per bid.
		this.readAt = this.now()
		try {
			const api = await this.hyperbridge.queryApi()
			const address = this.hyperbridge.getKeyPair().address
			const account = (await api.query.system.account(address)) as any
			this.free = BigInt(account.data.free.toString())
			if (this.cost === undefined) this.cost = await this.bidCost(api)
		} catch (err) {
			this.logger.warn(
				{ err },
				this.free === undefined
					? "Could not read the BRIDGE balance; bids go to Hyperbridge as usual"
					: "Could not read the BRIDGE balance; keeping the last reading",
			)
		}
	}

	/**
	 * What one bid costs in free balance: the storage deposit the pallet reserves (the runtime's
	 * current value, or its constant when none has been set), the existential deposit the free
	 * balance must keep, and the extrinsic's fee as estimated for a representative `place_bid`.
	 */
	private async bidCost(api: any): Promise<bigint> {
		const stored = BigInt((await api.query.intentsCoprocessor.storageDepositFee()).toString())
		const deposit = stored > 0n ? stored : BigInt(api.consts.intentsCoprocessor.storageDepositFee.toString())
		const existential = BigInt(api.consts.balances.existentialDeposit.toString())
		let fee = 0n
		try {
			const zero = `0x${"00".repeat(32)}`
			const info = await api.tx.intentsCoprocessor
				.placeBid(zero, zero, `0x${"00".repeat(512)}`)
				.paymentInfo(this.hyperbridge.getKeyPair().address)
			fee = BigInt(info.partialFee.toString())
		} catch (err) {
			this.logger.debug({ err }, "Could not estimate the bid fee; counting the deposit alone")
		}
		return deposit + existential + fee
	}
}

/**
 * Hyperbridge's answer when the account cannot pay for the bid, as the SDK renders it: the pool's
 * fee refusal, or the pallet's `InsufficientBalance` on the deposit. Substrate's generic 1010 code
 * is not matched, since a stale nonce or a bad signature carries it too.
 */
function wantsBalance(result: BidSubmissionResult): boolean {
	return (
		!result.success &&
		!result.pending &&
		/Inability to pay some fees|InsufficientBalance|FundsUnavailable/.test(result.error ?? "")
	)
}

/**
 * The venues a bid goes to: the HyperFX orderbook always, and Hyperbridge when the filler holds
 * BRIDGE for the deposit. The orderbook post goes out first and does not wait on the balance
 * read, which can take a dead query endpoint's whole timeout; Hyperbridge's own submission waits
 * for its extrinsic to land. One venue's failure is not the other's: each answer is returned on
 * its own, and the caller decides what the bid is worth from them.
 */
export class BidVenues implements BidPlacer {
	constructor(
		private readonly venues: {
			hyperbridge?: IntentsCoprocessor
			orderbook?: OrderbookClient
			/** Absent when there is no Hyperbridge connection to read, so no gate to pass. */
			bridgeBalance?: BridgeBalanceGate
		},
		private readonly logger: Logger,
	) {}

	async place(commitment: HexString, userOp: HexString, bid: HexString): Promise<BidPlacement> {
		const { hyperbridge, orderbook, bridgeBalance } = this.venues
		const onOrderbookPending = orderbook ? this.postToOrderbook(commitment, userOp) : undefined
		const toHyperbridge = hyperbridge !== undefined && (bridgeBalance === undefined || (await bridgeBalance.canPlaceBid()))
		const [onHyperbridge, onOrderbook] = await Promise.all([
			toHyperbridge ? hyperbridge.submitBid(commitment, userOp, bid) : undefined,
			onOrderbookPending,
		])
		if (onHyperbridge && bridgeBalance) {
			if (onHyperbridge.success || onHyperbridge.pending) bridgeBalance.spent()
			else if (wantsBalance(onHyperbridge)) bridgeBalance.exhausted()
		}
		return {
			...(onHyperbridge ? { hyperbridge: onHyperbridge } : {}),
			...(onOrderbook ? { orderbook: onOrderbook } : {}),
		}
	}

	/** The orderbook's answer, with a request that never got one rendered as a failed submission. */
	private async postToOrderbook(commitment: HexString, userOp: HexString): Promise<SubmitBidResult> {
		try {
			const result = await this.venues.orderbook!.submitBid(userOp)
			if (result.kind === "rejected") {
				this.logger.info({ commitment, code: result.code, message: result.message }, "The orderbook refused the bid")
			} else if (result.kind === "failed") {
				this.logger.warn({ commitment, code: result.code, message: result.message }, "The orderbook could not take the bid")
			}
			return result
		} catch (err) {
			const message = err instanceof Error ? err.message : String(err)
			this.logger.warn({ commitment, err }, "Could not post the bid to the orderbook")
			return { kind: "failed", code: "REQUEST_FAILED", message, retryable: true }
		}
	}
}

/** Whether the orderbook holds the bid after this answer: it took it now, or already had it. */
export function orderbookHoldsBid(result: SubmitBidResult | undefined): boolean {
	return result?.kind === "accepted" || result?.kind === "unchanged"
}
