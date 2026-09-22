import { EventEmitter } from "node:events"
import * as webPush from "web-push"
import { USD_STABLE_SYMBOLS } from "@/config/asset-registry"
import { patchRuntimeState } from "@/data/state"
import type {
	ActivityEvent,
	NotificationRuntimeState,
	NotificationSettings,
	OperatorNotification,
	StateStore,
	StoredPushSubscription,
} from "@/data/types"
import type { BalanceSnapshot } from "@/services/BalanceProvider"
import { getLogger, type Logger } from "@/services/Logger"

const BALANCE_CHECK_INTERVAL_MS = 15_000
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
	const negative = raw.startsWith("-")
	const digits = negative ? raw.slice(1) : raw
	const padded = digits.padStart(decimals + 1, "0")
	const whole = decimals === 0 ? padded : padded.slice(0, -decimals) || "0"
	const fraction = decimals === 0 ? "" : padded.slice(-decimals).replace(/0+$/, "").slice(0, 4)
	return `${negative ? "-" : ""}${whole}${fraction ? `.${fraction}` : ""}`
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
	private ready: Promise<void>
	private interval?: NodeJS.Timeout
	private stopped = false
	private readonly logger: Logger
	private readonly notifiedSwapIds = new Set<string>()
	private readonly onActivity = (event: ActivityEvent) => {
		if (event.type === "filled" && this.state.settings.swaps) {
			const id = event.orderId ?? `event-${event.id}`
			if (this.notifiedSwapIds.has(id)) return
			this.notifiedSwapIds.add(id)
			if (this.notifiedSwapIds.size > 1_000) {
				const oldest = this.notifiedSwapIds.values().next().value
				if (oldest !== undefined) this.notifiedSwapIds.delete(oldest)
			}
			void this.publish({
				title: "Swap filled",
				body: swapBody(event),
				tag: `simplex-swap-${event.orderId ?? event.id}`,
				url: "./orders",
			}).catch((error) => this.logger.error({ err: error }, "Could not publish swap notification"))
		}
	}

	constructor(
		private readonly store: StateStore,
		private readonly balances: { getSnapshot(): BalanceSnapshot },
		private readonly activity: ActivitySource,
		logger: Logger = getLogger("notifications"),
	) {
		super()
		this.logger = logger
		this.ready = this.initialize()
		void this.ready.catch((error) => this.logger.error({ err: error }, "Notification service could not start"))
	}

	private async initialize(): Promise<void> {
		const runtime = await this.store.get()
		this.state = initialState(runtime.notifications)
		if (!runtime.notifications?.vapid?.privateKey) await this.persist()
		if (this.stopped) return
		this.activity.on("event", this.onActivity)
		this.interval = setInterval(() => {
			void this.evaluateLiquidity().catch((error) =>
				this.logger.error({ err: error }, "Could not evaluate low-liquidity notification"),
			)
		}, BALANCE_CHECK_INTERVAL_MS)
		this.interval.unref()
	}

	async status(): Promise<NotificationStatus> {
		await this.ready
		return {
			settings: { ...this.state.settings },
			vapidPublicKey: this.state.vapid.publicKey,
			subscriptionCount: this.state.subscriptions.length,
		}
	}

	async updateSettings(settings: NotificationSettings): Promise<NotificationStatus> {
		await this.ready
		this.state.settings = settings
		if (settings.lowLiquidityThresholdUsd === null) this.state.lowLiquidityActive = false
		await this.persist()
		await this.evaluateLiquidity()
		return this.status()
	}

	async subscribe(subscription: StoredPushSubscription): Promise<NotificationStatus> {
		await this.ready
		if (!validSubscription(subscription)) throw new Error("Invalid push subscription")
		this.state.subscriptions = [
			...this.state.subscriptions.filter((item) => item.endpoint !== subscription.endpoint),
			subscription,
		]
		await this.persist()
		return this.status()
	}

	async unsubscribe(endpoint: string): Promise<NotificationStatus> {
		await this.ready
		this.state.subscriptions = this.state.subscriptions.filter((item) => item.endpoint !== endpoint)
		await this.persist()
		return this.status()
	}

	async test(options?: { endpoint?: string; native?: boolean; push?: boolean }): Promise<PushDelivery> {
		await this.ready
		return this.publish(
			{
				title: "Simplex notifications are working",
				body: "This device will receive liquidity and swap alerts.",
				tag: `simplex-test-${Date.now()}`,
				url: "./",
			},
			options,
		)
	}

	stop(): void {
		this.stopped = true
		this.activity.off("event", this.onActivity)
		if (this.interval) clearInterval(this.interval)
		this.interval = undefined
	}

	private async evaluateLiquidity(): Promise<void> {
		const threshold = this.state.settings.lowLiquidityThresholdUsd
		if (threshold === null) return
		const snapshot = this.balances.getSnapshot()
		// A partial snapshot can omit an entire failed chain, which would make the
		// known subtotal look lower than the operator's actual liquidity.
		if (snapshot.status !== "fresh") return
		const stableAssets = snapshot.chains.flatMap((chain) =>
			chain.assets.filter((asset) => USD_STABLE_SYMBOLS.has(asset.symbol.trim().toUpperCase())),
		)
		if (stableAssets.length === 0 || stableAssets.some((asset) => asset.available === null)) return
		const liquidity = stableAssets.reduce((total, asset) => total + (asset.available ?? 0), 0)
		const low = liquidity < threshold
		if (low === this.state.lowLiquidityActive) return
		this.state.lowLiquidityActive = low
		await this.persist()
		if (!low) return
		await this.publish({
			title: "Simplex liquidity is low",
			body: `$${liquidity.toLocaleString(undefined, { maximumFractionDigits: 2 })} is available to fill, below your $${threshold.toLocaleString()} alert threshold.`,
			tag: "simplex-low-liquidity",
			url: "./",
		})
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
