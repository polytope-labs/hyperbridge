import {
	BandwidthApp,
	BandwidthAppDailyConsumption,
	BandwidthAppTierStats,
	BandwidthSubscription,
	BandwidthTier,
} from "@/configs/src/types"
import { timestampToDate } from "@/utils/date.helpers"

/**
 * Upserts and roll-ups for `pallet-bandwidth` indexing.
 *
 * Entity ID conventions:
 *   - `BandwidthApp.id`             = `${chain}-${appHex}`
 *   - `BandwidthAppTierStats.id`    = `${chain}-${appHex}-${tier}`
 *   - `BandwidthSubscription.id`    = `${chain}-${appHex}-${blockNumber}-${eventIdx}`
 *   - `BandwidthAppDailyConsumption.id` = `${chain}-${appHex}-YYYY-MM-DD`
 *   - `BandwidthTier.id`            = `${tier}`
 *
 * `appHex` is the lowercase hex of the `AppKey` bytes, no `0x` prefix.
 */
export class BandwidthService {
	static appId(chain: string, appHex: string): string {
		return `${chain}-${appHex}`
	}

	static tierStatsId(chain: string, appHex: string, tier: number): string {
		return `${chain}-${appHex}-${tier}`
	}

	static subscriptionId(
		chain: string,
		appHex: string,
		blockNumber: bigint,
		eventIdx: number,
	): string {
		return `${chain}-${appHex}-${blockNumber.toString()}-${eventIdx}`
	}

	static dailyConsumptionId(chain: string, appHex: string, isoDate: string): string {
		return `${chain}-${appHex}-${isoDate}`
	}

	/** Strip `0x`, lowercase. Defensive against differing event encodings. */
	static normalizeAppHex(raw: string): string {
		return raw.toLowerCase().replace(/^0x/, "")
	}

	/**
	 * Get-or-create `BandwidthApp`. Caller still has to `.save()` after
	 * mutating any counters.
	 */
	static async getOrCreateApp(
		chain: string,
		appHex: string,
		blockTimestampMs: bigint,
	): Promise<BandwidthApp> {
		const id = this.appId(chain, appHex)
		let app = await BandwidthApp.get(id)
		if (!app) {
			app = BandwidthApp.create({
				id,
				chain,
				app: appHex,
				firstSeenAt: blockTimestampMs,
				lastActivityAt: blockTimestampMs,
				lifetimeSubscriptions: BigInt(0),
				lifetimeBytesCredited: BigInt(0),
				lifetimeBytesConsumed: BigInt(0),
				lifetimeBytesEvicted: BigInt(0),
				activeSubscriptions: 0,
			})
		}
		return app
	}

	/** Get-or-create per-`(app, tier)` lifetime stats row. */
	static async getOrCreateTierStats(
		chain: string,
		appHex: string,
		tier: number,
		blockTimestampMs: bigint,
	): Promise<BandwidthAppTierStats> {
		const id = this.tierStatsId(chain, appHex, tier)
		let stats = await BandwidthAppTierStats.get(id)
		if (!stats) {
			stats = BandwidthAppTierStats.create({
				id,
				appId: this.appId(chain, appHex),
				tier,
				lifetimeSubscriptions: BigInt(0),
				lifetimeBytesCredited: BigInt(0),
				lifetimeBytesEvicted: BigInt(0),
				activeSubscriptions: 0,
				lastPurchasedAt: blockTimestampMs,
			})
		}
		return stats
	}

	/**
	 * Record a new subscription created by either `BandwidthCredited`
	 * (purchase) or `ForceCredited` (admin). Bumps both app-level and
	 * per-tier lifetime counters and writes the `BandwidthSubscription`
	 * row. Caller passes `paidFrom` as the chain that paid (= the app's
	 * own chain for `ForceCredited`).
	 */
	static async recordCredit(params: {
		chain: string
		appHex: string
		tier: number
		bytes: bigint
		expiresAtSecs: bigint
		paidFrom: string
		forced: boolean
		blockNumber: bigint
		blockTimestampMs: bigint
		eventIdx: number
		extrinsicHash?: string
	}): Promise<void> {
		const app = await this.getOrCreateApp(params.chain, params.appHex, params.blockTimestampMs)
		const tierStats = await this.getOrCreateTierStats(
			params.chain,
			params.appHex,
			params.tier,
			params.blockTimestampMs,
		)

		const sub = BandwidthSubscription.create({
			id: this.subscriptionId(
				params.chain,
				params.appHex,
				params.blockNumber,
				params.eventIdx,
			),
			appId: app.id,
			tier: params.tier,
			bytes: params.bytes,
			expiresAt: params.expiresAtSecs,
			purchasedAt: params.blockTimestampMs,
			paidFrom: params.paidFrom,
			forced: params.forced,
			evictedAt: undefined,
			evictedLostBytes: undefined,
			blockNumber: params.blockNumber,
			extrinsicHash: params.extrinsicHash,
		})
		await sub.save()

		app.lastActivityAt = params.blockTimestampMs
		app.lifetimeSubscriptions += BigInt(1)
		app.lifetimeBytesCredited += params.bytes
		await app.save()

		tierStats.lifetimeSubscriptions += BigInt(1)
		tierStats.lifetimeBytesCredited += params.bytes
		tierStats.lastPurchasedAt = params.blockTimestampMs
		await tierStats.save()
	}

	/**
	 * Record a gate deduction. Bumps the app's `lifetimeBytesConsumed`
	 * and upserts the per-app daily bucket. Monthly/yearly views are
	 * computed over a date range on the daily entity.
	 */
	static async recordConsumption(params: {
		chain: string
		appHex: string
		bytes: bigint
		blockTimestampMs: bigint
	}): Promise<void> {
		const app = await this.getOrCreateApp(params.chain, params.appHex, params.blockTimestampMs)
		app.lastActivityAt = params.blockTimestampMs
		app.lifetimeBytesConsumed += params.bytes
		await app.save()

		const day = timestampToDate(params.blockTimestampMs)
		day.setUTCHours(0, 0, 0, 0)
		const dateString = day.toISOString().slice(0, 10)
		const dayId = this.dailyConsumptionId(params.chain, params.appHex, dateString)

		let bucket = await BandwidthAppDailyConsumption.get(dayId)
		if (!bucket) {
			bucket = BandwidthAppDailyConsumption.create({
				id: dayId,
				appId: app.id,
				date: day,
				bytesConsumed: BigInt(0),
			})
		}
		bucket.bytesConsumed += params.bytes
		await bucket.save()
	}

	/**
	 * Attribute a `SubscriptionEvicted` event to the oldest unevicted
	 * `BandwidthSubscription` of matching tier for this `(chain, app)`.
	 *
	 * Heuristic: the on-chain FIFO drops the front of the list; the
	 * event payload tells us the evicted tier. We pick the lowest-
	 * `purchasedAt` row for this `(app, tier)` that hasn't been marked
	 * evicted yet. This is approximate — if the oldest sub of that tier
	 * naturally drained or expired before being evicted, the heuristic
	 * mis-attributes. Aggregate counters (`lifetimeBytesEvicted`) are
	 * always correct; only the per-sub closure is best-effort.
	 */
	static async recordEviction(params: {
		chain: string
		appHex: string
		tier: number
		lostBytes: bigint
		blockTimestampMs: bigint
	}): Promise<void> {
		const app = await this.getOrCreateApp(params.chain, params.appHex, params.blockTimestampMs)
		app.lastActivityAt = params.blockTimestampMs
		app.lifetimeBytesEvicted += params.lostBytes
		await app.save()

		const tierStats = await this.getOrCreateTierStats(
			params.chain,
			params.appHex,
			params.tier,
			params.blockTimestampMs,
		)
		tierStats.lifetimeBytesEvicted += params.lostBytes
		await tierStats.save()

		const candidates = await BandwidthSubscription.getByFields(
			[
				["appId", "=", app.id],
				["tier", "=", params.tier],
			],
			{ limit: 100 },
		)
		const target = candidates
			.filter((s) => s.evictedAt === undefined || s.evictedAt === null)
			.sort((a, b) => Number(a.purchasedAt - b.purchasedAt))[0]
		if (target) {
			target.evictedAt = params.blockTimestampMs
			target.evictedLostBytes = params.lostBytes
			await target.save()
		} else {
			logger.warn(
				`BandwidthService.recordEviction: no unevicted sub found for app=${app.id} tier=${params.tier}`,
			)
		}
	}

	/**
	 * Apply a `TierSet` governance change. `None` config (revoke)
	 * leaves the row with `isActive: false` and nulled-out limits.
	 */
	static async applyTierSet(params: {
		tier: number
		bytes?: bigint
		durationSecs?: bigint
		blockTimestampMs: bigint
	}): Promise<void> {
		const id = params.tier.toString()
		let tier = await BandwidthTier.get(id)
		if (!tier) {
			tier = BandwidthTier.create({
				id,
				tier: params.tier,
				bytes: undefined,
				durationSecs: undefined,
				isActive: false,
				lastUpdatedAt: params.blockTimestampMs,
			})
		}
		if (params.bytes !== undefined && params.durationSecs !== undefined) {
			tier.bytes = params.bytes
			tier.durationSecs = params.durationSecs
			tier.isActive = true
		} else {
			tier.bytes = undefined
			tier.durationSecs = undefined
			tier.isActive = false
		}
		tier.lastUpdatedAt = params.blockTimestampMs
		await tier.save()
	}

	/**
	 * Re-derive `activeSubscriptions` for an `(app_chain, app_key)` pair
	 * from on-chain `pallet-bandwidth :: Allowance` storage at
	 * `blockHash`. Two paths inside `try_consume` — natural drain
	 * (`remaining_bytes → 0`) and expiry sweep — mutate the list
	 * silently with no event, so delta-based counting from events
	 * drifts strictly upward over time. This is the source-of-truth
	 * refresh called after every bandwidth event.
	 *
	 * Primary path uses the typed `api.at(blockHash).query` which
	 * decodes via runtime metadata. On failure (most commonly the
	 * archive node pruned the historical state) it falls back to a
	 * tip read; counts become eventually-consistent rather than
	 * block-exact, which is still strictly better than the previous
	 * unbounded drift.
	 */
	static async syncActiveCounts(params: {
		chain: string
		appHex: string
		palletAppChain: unknown
		palletAppKey: unknown
		blockHash: string
		blockTimestampMs: bigint
	}): Promise<void> {
		let raw: unknown
		try {
			const apiAt = await api.at(params.blockHash)
			raw = await apiAt.query.bandwidth.allowance(
				params.palletAppChain,
				params.palletAppKey,
			)
		} catch (err) {
			logger.warn(
				`syncActiveCounts: at-block read failed (${(err as Error).message}); falling back to tip`,
			)
			try {
				raw = await api.query.bandwidth.allowance(
					params.palletAppChain,
					params.palletAppKey,
				)
			} catch (err2) {
				logger.warn(
					`syncActiveCounts: tip fallback failed (${(err2 as Error).message}); leaving counts unchanged`,
				)
				return
			}
		}

		const list = raw as unknown as Iterable<{ tier: { toNumber(): number } }>
		const tierCounts = new Map<number, number>()
		let total = 0
		for (const sub of list) {
			total++
			const tier = sub.tier.toNumber()
			tierCounts.set(tier, (tierCounts.get(tier) ?? 0) + 1)
		}

		const app = await this.getOrCreateApp(
			params.chain,
			params.appHex,
			params.blockTimestampMs,
		)
		app.activeSubscriptions = total
		await app.save()

		// Walk every per-tier stats row for this app and overwrite from
		// the live count. Zeroes out tiers that had active subs before
		// but don't now — the silent-drain / expiry case Seun flagged.
		const existing = await BandwidthAppTierStats.getByFields(
			[["appId", "=", app.id]],
			{ limit: 100 },
		)
		const updated = new Set<number>()
		for (const row of existing) {
			row.activeSubscriptions = tierCounts.get(row.tier) ?? 0
			await row.save()
			updated.add(row.tier)
		}
		// Defensive: storage shows a tier with no prior stats row — can
		// happen on genesis state or if we missed an earlier event.
		for (const [tier, count] of tierCounts) {
			if (updated.has(tier)) continue
			const stats = await this.getOrCreateTierStats(
				params.chain,
				params.appHex,
				tier,
				params.blockTimestampMs,
			)
			stats.activeSubscriptions = count
			await stats.save()
		}
	}
}
