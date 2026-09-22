import { EventEmitter } from "node:events"
import { formatUnits } from "viem"
import * as webPush from "web-push"
import { patchRuntimeState } from "@/data/state"
import type {
	ActivityEvent,
	NotificationRuntimeState,
	NotificationSettings,
	OperatorNotification,
	StateStore,
	StoredPushSubscription,
} from "@/data/types"
import type { BalanceProvider, BalanceSnapshot } from "@/services/BalanceProvider"
import { getLogger, type Logger } from "@/services/Logger"
import { availableStablecoinLiquidity } from "@/services/stablecoin-liquidity"

const DEFAULT_SETTINGS: NotificationSettings = { lowLiquidityThresholdUsd: null, swaps: false }

export type { OperatorNotification } from "@/data/types"

export interface NotificationStatus {
	settings: NotificationSettings
	vapidPublicKey: string
	subscriptionCount: number
}

export interface PushDelivery {
	pushSent: number
	pushFailed: number
}

interface ActivitySource {
	on(event: "event", listener: (event: ActivityEvent) => void): unknown
	off(event: "event", listener: (event: ActivityEvent) => void): unknown
}

type BalanceSource = Pick<BalanceProvider, "getSnapshot" | "on" | "off">

function validSubscription(value: unknown): value is StoredPushSubscription {
	if (!value || typeof value !== "object") return false
	const subscription = value as Partial<StoredPushSubscription>
	return Boolean(
		typeof subscription.endpoint === "string" &&
			/^https:\/\//.test(subscription.endpoint) &&
			subscription.keys &&
			typeof subscription.keys.p256dh === "string" &&
			typeof subscription.keys.auth === "string",
	)
}

function initialState(saved?: NotificationRuntimeState): NotificationRuntimeState {
	const vapid = saved?.vapid?.publicKey && saved.vapid.privateKey ? saved.vapid : webPush.generateVAPIDKeys()
	return {
		settings: {
			...DEFAULT_SETTINGS,
			lowLiquidityThresholdUsd:
				typeof saved?.settings?.lowLiquidityThresholdUsd === "number"
					? saved.settings.lowLiquidityThresholdUsd
					: null,
			swaps: saved?.settings?.swaps === true,
		},
		vapid,
		subscriptions: (saved?.subscriptions ?? []).filter(validSubscription),
		lowLiquidityActive: saved?.lowLiquidityActive === true,
	}
}

function displayAmount(raw: string, decimals: number | null): string {
	if (decimals === null) return raw
	return formatUnits(BigInt(raw), decimals)
}

function swapBody(event: ActivityEvent): string {
	const input = event.order?.inputs[0]
	const output = event.order?.outputs[0]
	if (!input || !output) return "Simplex completed a swap."
	const inSymbol = input.symbol ?? "token"
	const outSymbol = output.symbol ?? "token"
	return `${displayAmount(input.amount, input.decimals)} ${inSymbol} → ${displayAmount(output.amount, output.decimals)} ${outSymbol}`
}

/** Durable alert rules plus Web Push delivery for one running operator. */
export class NotificationService extends EventEmitter {
	private state!: NotificationRuntimeState
	private initialization?: Promise<void>
	private initialized = false
	private stopped = false
	private readonly logger: Logger
	private readonly notifiedSwapIds = new Set<string>()
	private readonly onActivity = (event: ActivityEvent) => {
		if (event.type === "filled" && this.state.settings.swaps) {
			const id = event.txHash
				? `${event.orderId ?? "unknown-order"}:${event.txHash}`
				: (event.orderId ?? `event-${event.id}`)
			if (this.notifiedSwapIds.has(id)) return
			this.notifiedSwapIds.add(id)
			if (this.notifiedSwapIds.size > 1_000) {
				const oldest = this.notifiedSwapIds.values().next().value
				if (oldest !== undefined) this.notifiedSwapIds.delete(oldest)
			}
			void this.publish({
				title: "Swap filled",
				body: swapBody(event),
				tag: `simplex-swap-${id}`,
				url: "./orders",
			}).catch((error) => this.logger.error({ err: error }, "Could not publish swap notification"))
		}
	}
	private readonly onBalanceSnapshot = (snapshot: BalanceSnapshot) => {
		void this.evaluateLiquidity(snapshot).catch((error) =>
			this.logger.error({ err: error }, "Could not evaluate low-liquidity notification"),
		)
	}

	constructor(
		private readonly store: StateStore,
		private readonly balances: BalanceSource,
		private readonly activity: ActivitySource,
		logger: Logger = getLogger("notifications"),
	) {
		super()
		this.logger = logger
		void this.ensureReady().catch(() => undefined)
	}

	private ensureReady(): Promise<void> {
		if (this.initialized) return Promise.resolve()
		if (!this.initialization) {
			const attempt = this.initialize().then(() => {
				this.initialized = true
			})
			this.initialization = attempt
			void attempt
				.catch((error) => this.logger.error({ err: error }, "Notification service could not start"))
				.finally(() => {
					if (!this.initialized && this.initialization === attempt) this.initialization = undefined
				})
		}
		return this.initialization
	}

	private async initialize(): Promise<void> {
		const runtime = await this.store.get()
		this.state = initialState(runtime.notifications)
		if (!runtime.notifications?.vapid?.privateKey) await this.persist()
		if (this.stopped) return
		this.activity.on("event", this.onActivity)
		this.balances.on("snapshot", this.onBalanceSnapshot)
		void this.evaluateLiquidity().catch((error) =>
			this.logger.error({ err: error }, "Could not evaluate initial low-liquidity notification"),
		)
	}

	async status(): Promise<NotificationStatus> {
		await this.ensureReady()
		return {
			settings: { ...this.state.settings },
			vapidPublicKey: this.state.vapid.publicKey,
			subscriptionCount: this.state.subscriptions.length,
		}
	}

	async updateSettings(settings: NotificationSettings): Promise<NotificationStatus> {
		await this.ensureReady()
		this.state.settings = settings
		if (settings.lowLiquidityThresholdUsd === null) this.state.lowLiquidityActive = false
		await this.persist()
		await this.evaluateLiquidity()
		return this.status()
	}

	async subscribe(subscription: StoredPushSubscription): Promise<NotificationStatus> {
		await this.ensureReady()
		if (!validSubscription(subscription)) throw new Error("Invalid push subscription")
		this.state.subscriptions = [
			...this.state.subscriptions.filter((item) => item.endpoint !== subscription.endpoint),
			subscription,
		]
		await this.persist()
		return this.status()
	}

	async unsubscribe(endpoint: string): Promise<NotificationStatus> {
		await this.ensureReady()
		this.state.subscriptions = this.state.subscriptions.filter((item) => item.endpoint !== endpoint)
		await this.persist()
		return this.status()
	}

	async test(options?: {
		endpoint?: string
		native?: boolean
		push?: boolean
		receiptId?: string
	}): Promise<PushDelivery> {
		await this.ensureReady()
		return this.publish(
			{
				title: "Simplex notifications are working",
				body: "This device will receive liquidity and swap alerts.",
				tag: `simplex-test-${Date.now()}`,
				url: "./",
				...(options?.receiptId ? { receiptId: options.receiptId } : {}),
			},
			options,
		)
	}

	stop(): void {
		this.stopped = true
		this.activity.off("event", this.onActivity)
		this.balances.off("snapshot", this.onBalanceSnapshot)
	}

	private async evaluateLiquidity(snapshot = this.balances.getSnapshot()): Promise<void> {
		const threshold = this.state.settings.lowLiquidityThresholdUsd
		if (threshold === null) return
		const liquidity = availableStablecoinLiquidity(snapshot)
		if (liquidity === null) return
		const low = liquidity < threshold
		if (low === this.state.lowLiquidityActive) return
		if (low) {
			await this.publish({
				title: "Simplex liquidity is low",
				body: `$${liquidity.toLocaleString(undefined, { maximumFractionDigits: 2 })} is available to fill, below your $${threshold.toLocaleString()} alert threshold.`,
				tag: "simplex-low-liquidity",
				url: "./",
			})
		}
		this.state.lowLiquidityActive = low
		await this.persist()
	}

	private async publish(
		notification: OperatorNotification,
		options?: { endpoint?: string; native?: boolean; push?: boolean },
	): Promise<PushDelivery> {
		if (options?.native !== false) this.emit("notification", notification)
		const subscriptions =
			options?.push === false
				? []
				: options?.endpoint
					? this.state.subscriptions.filter((subscription) => subscription.endpoint === options.endpoint)
					: this.state.subscriptions
		if (subscriptions.length === 0) return { pushSent: 0, pushFailed: 0 }
		const expired = new Set<string>()
		let pushSent = 0
		let pushFailed = 0
		await Promise.all(
			subscriptions.map(async (subscription) => {
				try {
					await webPush.sendNotification(subscription, JSON.stringify(notification), {
						vapidDetails: {
							subject: "https://hyperbridge.network",
							publicKey: this.state.vapid.publicKey,
							privateKey: this.state.vapid.privateKey,
						},
						TTL: 60 * 60,
						timeout: 10_000,
						urgency: notification.tag === "simplex-low-liquidity" ? "high" : "normal",
					})
					pushSent++
				} catch (error) {
					pushFailed++
					const statusCode = (error as { statusCode?: number }).statusCode
					if (statusCode === 404 || statusCode === 410) expired.add(subscription.endpoint)
					else
						this.logger.warn(
							{ err: error, endpoint: subscription.endpoint },
							"Failed to send push notification",
						)
				}
			}),
		)
		if (expired.size > 0) {
			this.state.subscriptions = this.state.subscriptions.filter((item) => !expired.has(item.endpoint))
			await this.persist()
		}
		return { pushSent, pushFailed }
	}

	private async persist(): Promise<void> {
		await patchRuntimeState(this.store, { notifications: this.state })
	}
}
