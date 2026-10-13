/**
 * The top-up's progress as timeline steps, following the Hyperbridge app: the swap's
 * Order Placed / Offers Received / Order Fulfilled, then the bridge's finality steps for the
 * BRIDGE's trip from BNB Chain into Hyperbridge. Each step knows when it started, when it
 * ended, and, while it runs, when it is expected to end.
 */

export type TopUpStage =
	| "placing"
	| "awaiting_bids"
	| "filling"
	| "delivering"
	| "delivered"
	| "unfilled"
	| "reclaiming"
	| "reclaimed"
	| "filled"
	| "failed"

export type StepStatus = "completed" | "processing" | "upcoming" | "error"

export interface TopUpBidPreview {
	solverAddress: string
	outputs: { token: string; amount: string }[]
}

export interface TopUpExecutionEvent {
	status: "NEW_BID" | "BIDS_RECEIVED" | "BID_SELECTED" | "FILLED"
	timestamp: number
	bidCount?: number
	bids?: TopUpBidPreview[]
	selectedSolver?: string
	userOpHash?: string
	txHash?: string
}

export interface DeliveryStatus {
	status: "SOURCE" | "SOURCE_FINALIZED" | "DESTINATION"
	timestamp: number
	txHash: string
}

/** The parts of a top-up the timeline reads. */
export interface TimelineTopUp {
	chain: string
	token: string
	stage: TopUpStage
	createdAt: number
	updatedAt: number
	placeTxHash?: string
	fillTxHash?: string
	reclaimTxHash?: string
	commitment?: string
	deliveryCommitment?: string
	bidCount?: number
	filler?: string
	stageTimes?: Partial<Record<TopUpStage, number>>
	events?: TopUpExecutionEvent[]
	deliveryStatuses?: DeliveryStatus[]
	recoverySteps?: { step: RecoveryStepKey; timestamp: number; txHash?: string }[]
	error?: string
}

export type RecoveryStepKey =
	| "DESTINATION_FINALIZED"
	| "ORDER_CANCELED"
	| "SOURCE_FINALIZED"
	| "HYPERBRIDGE_DELIVERED"
	| "HYPERBRIDGE_FINALIZED"
	| "FUNDS_CLAIMED"

export type StepKey = "placed" | "offers" | "fulfilled" | "finalized" | "verified" | "claimed" | RecoveryStepKey

/** A transaction on a chain, or (`message`) the request itself in the Hyperbridge explorer. */
export type StepLink = { chain: "source" | "bsc" | "hyperbridge" | "message"; hash: string }

export interface TimelineStep {
	key: StepKey
	/** Level 0 rows carry a token on its network; level 1 rows are a dot under the row above. */
	level: 0 | 1
	caption: string
	status: StepStatus
	secondary?: string
	link?: StepLink
	/** Whether the step shows its countdown while running and its duration once done. */
	timed: boolean
	/** How long the step took, in ms, when both ends are known. */
	duration?: number
	/** When the running step is expected to end, in ms. */
	etaAt?: number
}

/**
 * Each step's line under its caption, after the Hyperbridge app's descriptions but kept short
 * enough to sit on one line in the dialog. A step with a transaction shows its explorer link
 * instead.
 */
const DESCRIPTIONS = {
	placed: { completed: "Order submitted" },
	offers: { pending: "Waiting for offers", completed: "Offers received" },
	fulfilled: {
		pending: "Executing on BNB Chain",
		completed: "Executed on BNB Chain",
	},
	finalized: {
		pending: "Waiting for BNB finality",
		completed: "Finality verified",
	},
	verified: { pending: "Waiting for Hyperbridge", completed: "Delivered to Hyperbridge" },
}

/** Expected step lengths, as the Hyperbridge app estimates them. */
export const ETA_MS = {
	offers: 30_000,
	fulfilled: 5_000,
	/** BNB Chain finality as Hyperbridge sees it. */
	finalized: 3 * 60_000,
	verified: 3.5 * 60_000,
}

export interface TimelineMoments {
	placedAt?: number
	offersAt?: number
	filledAt?: number
	finalizedAt?: number
	verifiedAt?: number
}

/** When each step ended, from the executor's events, the indexer's statuses and the stage times. */
export function timelineMoments(topUp: TimelineTopUp): TimelineMoments {
	const times = topUp.stageTimes ?? {}
	const events = topUp.events ?? []
	const delivery = (status: DeliveryStatus["status"]) =>
		topUp.deliveryStatuses?.find((entry) => entry.status === status)?.timestamp
	const firstEvent = (...statuses: TopUpExecutionEvent["status"][]) =>
		events.find((event) => statuses.includes(event.status))?.timestamp
	const filledAt = delivery("SOURCE") ?? firstEvent("FILLED") ?? times.delivering
	return {
		placedAt: times.awaiting_bids,
		offersAt: firstEvent("BIDS_RECEIVED") ?? times.filling ?? firstEvent("BID_SELECTED") ?? filledAt,
		filledAt,
		finalizedAt: delivery("SOURCE_FINALIZED"),
		verifiedAt: delivery("DESTINATION") ?? (topUp.stage === "delivered" ? times.delivered : undefined),
	}
}

/** Offers the solvers made, once each, in the order they arrived. */
export function timelineOffers(topUp: TimelineTopUp): { key: string; preview: TopUpBidPreview; timestamp: number }[] {
	const offers: { key: string; preview: TopUpBidPreview; timestamp: number }[] = []
	const seen = new Set<string>()
	for (const event of topUp.events ?? []) {
		if (event.status !== "NEW_BID" && event.status !== "BIDS_RECEIVED") continue
		for (const preview of event.bids ?? []) {
			const key = `${preview.solverAddress.toLowerCase()}:${JSON.stringify(preview.outputs)}`
			if (seen.has(key)) continue
			seen.add(key)
			offers.push({ key, preview, timestamp: event.timestamp })
		}
	}
	return offers
}

/** The offer count the executor last reported, else the offers seen. */
export function offerCount(topUp: TimelineTopUp): number {
	const received = [...(topUp.events ?? [])].reverse().find((event) => event.status === "BIDS_RECEIVED")
	if (received?.bidCount) return received.bidCount
	const seen = timelineOffers(topUp).length
	return seen || topUp.bidCount || 0
}

/** The solver whose bid was selected: from the selection, the fill, or the indexer. */
export function selectedSolver(topUp: TimelineTopUp): string | undefined {
	const selection = [...(topUp.events ?? [])]
		.reverse()
		.find((event) => (event.status === "BID_SELECTED" || event.status === "FILLED") && event.selectedSolver)
	return selection?.selectedSolver ?? topUp.filler
}

export function timelineSteps(topUp: TimelineTopUp, networkLabel: string, now = Date.now()): TimelineStep[] {
	const moments = timelineMoments(topUp)
	const placed = topUp.stage !== "placing" && topUp.stage !== "failed"
	const between = (start?: number, end?: number) =>
		start !== undefined && end !== undefined && end >= start ? end - start : undefined
	const after = (start: number | undefined, ms: number) => (start === undefined ? undefined : start + ms)

	const placedStep: TimelineStep = {
		key: "placed",
		level: 0,
		caption: placed ? "Order Placed" : topUp.stage === "failed" ? "Order Not Placed" : "Placing Order",
		status: topUp.stage === "failed" ? "error" : placed ? "completed" : "processing",
		// Once sent, the caption says it all and the row reads "Processing", as in the Hyperbridge app.
		secondary: topUp.stage === "failed" ? "Nothing was escrowed" : placed ? DESCRIPTIONS.placed.completed : "Processing",
		link: topUp.placeTxHash && placed ? { chain: "source", hash: topUp.placeTxHash } : undefined,
		timed: false,
	}
	// A placement that failed escrowed nothing; there is nothing further to show.
	if (topUp.stage === "failed") return [placedStep]

	const expired = topUp.stage === "unfilled" || topUp.stage === "reclaiming" || topUp.stage === "reclaimed"
	const offersDone = moments.offersAt !== undefined
	const filled = moments.filledAt !== undefined || topUp.stage === "delivering" || topUp.stage === "delivered" || topUp.stage === "filled"

	const offers: TimelineStep = {
		key: "offers",
		level: 1,
		caption: offersDone || filled ? "Offers Received" : "Awaiting Offers",
		status: offersDone || filled ? "completed" : expired || !placed ? "upcoming" : "processing",
		secondary:
			offersDone || filled
				? DESCRIPTIONS.offers.completed
				: placed && !expired
					? "Waiting for solver bids"
					: DESCRIPTIONS.offers.pending,
		timed: offersDone,
		duration: between(moments.placedAt, moments.offersAt),
		etaAt: after(moments.placedAt, ETA_MS.offers),
	}

	if (expired) return recoverySteps(topUp, placedStep, offers, networkLabel, now)

	const finalized = moments.finalizedAt !== undefined
	const verified = moments.verifiedAt !== undefined || topUp.stage === "delivered" || topUp.stage === "filled"
	// A record from before statuses were tracked has no times to show, only that it finished.
	const settled = verified && moments.finalizedAt === undefined

	return [
		placedStep,
		offers,
		{
			key: "fulfilled",
			level: 0,
			caption: filled ? "Order Fulfilled" : "Filling Order",
			status: filled ? "completed" : offersDone ? "processing" : "upcoming",
			secondary: !filled
				? DESCRIPTIONS.fulfilled.pending
				: topUp.fillTxHash
					? DESCRIPTIONS.fulfilled.completed
					: "Waiting for tx...",
			link: topUp.fillTxHash ? { chain: "bsc", hash: topUp.fillTxHash } : undefined,
			timed: true,
			duration: between(moments.offersAt, moments.filledAt),
			etaAt: after(moments.offersAt, ETA_MS.fulfilled),
		},
		{
			key: "finalized",
			level: 1,
			caption: "Transaction Finalized",
			status: finalized || settled ? "completed" : filled ? "processing" : "upcoming",
			secondary: finalized || settled ? DESCRIPTIONS.finalized.completed : DESCRIPTIONS.finalized.pending,
			link: topUp.deliveryStatuses?.find((s) => s.status === "SOURCE_FINALIZED")
				? { chain: "hyperbridge", hash: topUp.deliveryStatuses.find((s) => s.status === "SOURCE_FINALIZED")!.txHash }
				: undefined,
			timed: !settled,
			duration: between(moments.filledAt, moments.finalizedAt),
			etaAt: after(moments.filledAt, ETA_MS.finalized),
		},
		{
			// Delivery into Hyperbridge is the request's DESTINATION, its last status: for a request
			// to Hyperbridge itself, the SDK's status stream ends there.
			key: "verified",
			level: 0,
			caption: "Transaction Complete",
			status: verified ? "completed" : finalized ? "processing" : "upcoming",
			secondary: verified ? DESCRIPTIONS.verified.completed : DESCRIPTIONS.verified.pending,
			// The delivery's own transaction once indexed; until then, the request in the explorer.
			link: topUp.deliveryStatuses?.find((s) => s.status === "DESTINATION")
				? { chain: "hyperbridge", hash: topUp.deliveryStatuses.find((s) => s.status === "DESTINATION")!.txHash }
				: topUp.deliveryCommitment
					? { chain: "message", hash: topUp.deliveryCommitment }
					: undefined,
			timed: !settled,
			duration: between(moments.finalizedAt, moments.verifiedAt),
			etaAt: after(moments.finalizedAt, ETA_MS.verified),
		},
	]
}

/** A finished step's length, as the Hyperbridge app words it: "5 seconds", "2 minutes", "1 hour". */
export function formatStepDuration(ms: number): string {
	if (!Number.isFinite(ms) || ms < 0) return ""
	const seconds = Math.round(ms / 1000)
	const unit = (value: number, name: string) => `${value} ${name}${value === 1 ? "" : "s"}`
	if (seconds < 60) return unit(seconds, "second")
	const minutes = Math.round(seconds / 60)
	if (minutes < 60) return unit(minutes, "minute")
	return unit(Math.round(minutes / 60), "hour")
}

/** Elapsed time from the order's placement, as the offers list shows it: "9s", "1m 5s". */
export function formatElapsed(start?: number, end?: number): string | null {
	if (!start || !end || end < start) return null
	const totalSeconds = Math.round((end - start) / 1000)
	if (totalSeconds < 60) return `${totalSeconds}s`
	const hours = Math.floor(totalSeconds / 3600)
	const minutes = Math.floor((totalSeconds % 3600) / 60)
	const seconds = totalSeconds % 60
	if (hours > 0) return minutes > 0 ? `${hours}h ${minutes}m` : `${hours}h`
	return seconds > 0 ? `${minutes}m ${seconds}s` : `${minutes}m`
}

/** `0x1234...abcdef`, as the Hyperbridge app shortens an account. */
export function shortenAddress(address: string, length = 6): string {
	return `${address.substring(0, length)}...${address.substring(address.length - length)}`
}

/**
 * A token amount for display: whole amounts of 1 or more keep 4 fraction digits, smaller
 * ones keep 4 significant digits, trailing zeros dropped.
 */
export function formatDisplayAmount(raw: string, decimals: number): string {
	const value = BigInt(raw)
	const scale = 10n ** BigInt(decimals)
	const whole = value / scale
	const fraction = (value % scale).toString().padStart(decimals, "0")
	if (whole > 0n) {
		const kept = fraction.slice(0, 4).replace(/0+$/, "")
		return kept ? `${whole}.${kept}` : `${whole}`
	}
	const firstDigit = fraction.search(/[1-9]/)
	if (firstDigit === -1) return "0"
	return `0.${fraction.slice(0, firstDigit + 4).replace(/0+$/, "")}`
}

/** BSC, where the order fills and where an unfilled order's expiry is finalized. */
export const FILL_CHAIN = "EVM-56"

/** How long a chain takes to finalize, as the Hyperbridge app estimates it, by state machine. */
export function finalityEstimateMs(chain: string): number {
	switch (chain) {
		case "EVM-10":
		case "EVM-42161":
		case "EVM-8453":
			return 90 * 60_000
		case "EVM-1":
			return 20 * 60_000
		case "EVM-137":
		case "EVM-56":
			return 3 * 60_000
		case "EVM-100":
			return 7 * 60_000
		default:
			return 6 * 60_000
	}
}

/** Hyperbridge (Nexus) finality, as the Hyperbridge app estimates it. */
const HYPERBRIDGE_FINALITY_MS = 5 * 60_000
/** How long a relayer has to deliver the refund before it is claimed, as in the Hyperbridge app. */
export const RELAYER_DELIVERY_WAIT_MS = 3 * 60_000

/**
 * An unfilled order, as the Hyperbridge app's swap timeline shows its recovery.
 *
 * - Paid on BSC itself (same chain): Order Placed, then Claim Funds; cancelling refunds at once.
 * - Paid elsewhere, before recovery starts: the order's steps, with Order Expired.
 * - Paid elsewhere, once recovering: BSC's expiry finality, the cancel, its finality on the
 *   paying chain, Hyperbridge verifying and finalizing it, and the refund.
 */
function recoverySteps(
	topUp: TimelineTopUp,
	placedStep: TimelineStep,
	offers: TimelineStep,
	networkLabel: string,
	now: number,
): TimelineStep[] {
	const sameChain = topUp.chain === FILL_CHAIN
	const recorded = (step: RecoveryStepKey) => topUp.recoverySteps?.find((entry) => entry.step === step)
	const claimed = recorded("FUNDS_CLAIMED")
	const offersSeen = offers.status === "completed" ? [offers] : []

	if (sameChain) {
		const claim: TimelineStep = {
			key: "FUNDS_CLAIMED",
			level: 0,
			caption: claimed || topUp.stage === "reclaimed" ? "Funds Claimed" : "Claim Funds",
			// Unclaimed, it stays red until the claim lands, as in the Hyperbridge app.
			status: claimed || topUp.stage === "reclaimed" ? "completed" : topUp.stage === "reclaiming" ? "processing" : "error",
			secondary:
				claimed || topUp.stage === "reclaimed"
					? "Refund received"
					: topUp.stage === "reclaiming"
						? "Claiming refund"
						: "Claim funds below",
			link: claimed?.txHash ? { chain: "source", hash: claimed.txHash } : undefined,
			timed: false,
		}
		return [{ ...placedStep, caption: "Order Placed", secondary: "Expired, refund available" }, ...offersSeen, claim]
	}

	if (topUp.stage === "unfilled") {
		return [
			placedStep,
			offers,
			{ key: "fulfilled", level: 0, caption: "Order Expired", status: "error", secondary: "The order has expired", timed: false },
		]
	}

	const startedAt = topUp.stageTimes?.reclaiming ?? topUp.updatedAt
	const destName = "BNB Chain"
	const order: RecoveryStepKey[] = [
		"DESTINATION_FINALIZED",
		"ORDER_CANCELED",
		"SOURCE_FINALIZED",
		"HYPERBRIDGE_DELIVERED",
		"HYPERBRIDGE_FINALIZED",
		"FUNDS_CLAIMED",
	]
	const done = (step: RecoveryStepKey) => !!recorded(step) || (step === "FUNDS_CLAIMED" && topUp.stage === "reclaimed")
	const current = order.find((step) => !done(step))
	const status = (step: RecoveryStepKey): StepStatus => (done(step) ? "completed" : step === current ? "processing" : "upcoming")
	const at = (step: RecoveryStepKey) => recorded(step)?.timestamp
	const previous = (step: RecoveryStepKey) =>
		step === "DESTINATION_FINALIZED" ? startedAt : at(order[order.indexOf(step) - 1])
	const timing = (step: RecoveryStepKey, etaMs: number) => ({
		timed: true,
		duration: at(step) !== undefined && previous(step) !== undefined ? at(step)! - previous(step)! : undefined,
		etaAt: previous(step) !== undefined ? previous(step)! + etaMs : undefined,
	})
	const text = (step: RecoveryStepKey, pending: [string, string], completed: [string, string]) => {
		const [caption, secondary] = done(step) ? completed : pending
		return { caption, secondary }
	}
	const link = (step: RecoveryStepKey, chain: StepLink["chain"], fallback?: string) => {
		const hash = recorded(step)?.txHash ?? fallback
		return hash ? { chain, hash } : undefined
	}

	const finalizedAt = at("HYPERBRIDGE_FINALIZED")
	const waitingForRelayer = finalizedAt !== undefined && now - finalizedAt < RELAYER_DELIVERY_WAIT_MS

	return [
		{
			key: "DESTINATION_FINALIZED",
			level: 0,
			status: status("DESTINATION_FINALIZED"),
			...text("DESTINATION_FINALIZED", [`Awaiting ${destName} Finality`, "Finalizing expiry height"], [
				`${destName} Finalized`,
				"Expiry height finalized",
			]),
			...timing("DESTINATION_FINALIZED", finalityEstimateMs(FILL_CHAIN)),
		},
		{
			key: "ORDER_CANCELED",
			level: 1,
			status: status("ORDER_CANCELED"),
			...text("ORDER_CANCELED", ["Cancel Order", "Sending cancel transaction"], [
				"Order Canceled",
				"Cancel transaction sent",
			]),
			link: link("ORDER_CANCELED", "source", topUp.reclaimTxHash),
			timed: false,
		},
		{
			key: "SOURCE_FINALIZED",
			level: 1,
			status: status("SOURCE_FINALIZED"),
			...text("SOURCE_FINALIZED", [`Awaiting ${networkLabel} Finality`, "Finalizing cancellation"], [
				`${networkLabel} Finalized`,
				"Cancellation finalized",
			]),
			link: link("SOURCE_FINALIZED", "hyperbridge"),
			...timing("SOURCE_FINALIZED", finalityEstimateMs(topUp.chain)),
		},
		{
			key: "HYPERBRIDGE_DELIVERED",
			level: 1,
			status: status("HYPERBRIDGE_DELIVERED"),
			...text("HYPERBRIDGE_DELIVERED", ["Hyperbridge Verifying", "Verifying non-fulfillment"], [
				"Hyperbridge Verified",
				"Non-fulfillment verified",
			]),
			link: link("HYPERBRIDGE_DELIVERED", "hyperbridge"),
			...timing("HYPERBRIDGE_DELIVERED", 3 * 60_000),
		},
		{
			key: "HYPERBRIDGE_FINALIZED",
			level: 1,
			status: status("HYPERBRIDGE_FINALIZED"),
			...text("HYPERBRIDGE_FINALIZED", ["Hyperbridge Finalizing", "Finalizing recovery"], [
				"Hyperbridge Finalized",
				"Recovery finalized",
			]),
			link: link("HYPERBRIDGE_FINALIZED", "source"),
			...timing("HYPERBRIDGE_FINALIZED", HYPERBRIDGE_FINALITY_MS),
		},
		{
			key: "FUNDS_CLAIMED",
			level: 0,
			status: status("FUNDS_CLAIMED"),
			// The filler claims the refund itself once a relayer has had its window.
			...text(
				"FUNDS_CLAIMED",
				waitingForRelayer
					? ["Waiting for Relayer", "Relayer delivering refund"]
					: ["Claim Funds", "Claiming refund"],
				["Funds Claimed", "Refund received"],
			),
			link: link("FUNDS_CLAIMED", "source"),
			timed: false,
		},
	]
}
