import { EventEmitter } from "node:events"
import { afterEach, describe, expect, it, vi } from "vitest"
import { MemoryDataStore } from "@/data/memory"
import type { ActivityEvent } from "@/data/types"
import type { BalanceSnapshot } from "@/services/BalanceProvider"
import { NotificationService, type OperatorNotification } from "@/services/server/NotificationService"

const webPush = vi.hoisted(() => ({
	generateVAPIDKeys: vi.fn(() => ({ publicKey: "vapid-public", privateKey: "vapid-private" })),
	sendNotification: vi.fn().mockResolvedValue({}),
}))
vi.mock("web-push", () => webPush)

function snapshot(available: number): BalanceSnapshot {
	return {
		updatedAt: Date.now(),
		status: "fresh",
		issues: [],
		chains: [
			{
				chainId: 1,
				assets: [
					{
						address: "0x1",
						symbol: "USDC",
						wallet: available,
						walletReserve: 0,
						vaultPosition: 0,
						vaultAvailable: 0,
						total: available,
						available,
						vaults: [],
						status: "fresh",
					},
				],
			},
		],
	}
}

const services: NotificationService[] = []
afterEach(() => {
	for (const service of services.splice(0)) service.stop()
	webPush.sendNotification.mockReset().mockResolvedValue({})
})

function serviceAt(available: number) {
	const activity = new EventEmitter()
	const state = new MemoryDataStore().state
	const balance = { current: snapshot(available) }
	const service = new NotificationService(state, { getSnapshot: () => balance.current }, activity)
	services.push(service)
	return { service, state, activity, balance }
}

describe("operator notifications", () => {
	it("alerts only on a low-liquidity crossing and persists the armed state", async () => {
		const { service, state } = serviceAt(50)
		const alerts: OperatorNotification[] = []
		service.on("notification", (alert) => alerts.push(alert))

		await service.updateSettings({ lowLiquidityThresholdUsd: 100, swaps: false })
		await service.updateSettings({ lowLiquidityThresholdUsd: 100, swaps: false })
		expect(alerts).toHaveLength(1)
		expect(alerts[0]).toMatchObject({ title: "Simplex liquidity is low", tag: "simplex-low-liquidity" })
		expect((await state.get()).notifications?.lowLiquidityActive).toBe(true)

		await service.updateSettings({ lowLiquidityThresholdUsd: 25, swaps: false })
		await service.updateSettings({ lowLiquidityThresholdUsd: 100, swaps: false })
		expect(alerts).toHaveLength(2)
	})

	it("emits a completed-swap alert with the recorded amounts", async () => {
		const { service, activity } = serviceAt(500)
		await service.updateSettings({ lowLiquidityThresholdUsd: null, swaps: true })
		const alerts: OperatorNotification[] = []
		service.on("notification", (notification) => alerts.push(notification))
		const event = {
			id: 7,
			ts: Date.now(),
			type: "filled",
			orderId: "0xorder",
			chainId: 1,
			strategy: null,
			success: true,
			reason: null,
			volumeUsd: 12,
			profitUsd: 1,
			txHash: "0xtx",
			order: {
				user: "0xuser",
				source: "EVM-1",
				destination: "EVM-2",
				placedTxHash: null,
				referrer: null,
				deadline: "1",
				inputs: [{ token: "0x1", amount: "12500000", symbol: "USDC", decimals: 6 }],
				outputs: [{ token: "0x2", amount: "25000000000000000000", symbol: "DAI", decimals: 18 }],
			},
		} satisfies ActivityEvent
		activity.emit("event", event)
		activity.emit("event", { ...event, id: 8, txHash: "0xobserved" })

		expect(alerts).toHaveLength(1)
		expect(alerts[0]).toMatchObject({
			title: "Swap filled",
			body: "12.5 USDC → 25 DAI",
			tag: "simplex-swap-0xorder",
		})
	})

	it("does not infer low liquidity from a partial stablecoin snapshot", async () => {
		const { service, balance } = serviceAt(50)
		balance.current.status = "partial"
		balance.current.chains[0].assets[0].available = null
		const notify = vi.fn()
		service.on("notification", notify)
		await service.updateSettings({ lowLiquidityThresholdUsd: 100, swaps: false })
		expect(notify).not.toHaveBeenCalled()
	})

	it("does not infer low liquidity when a partial snapshot omits a chain", async () => {
		const { service, balance } = serviceAt(50)
		balance.current.status = "partial"
		balance.current.chains.push({ chainId: 2, assets: [] })
		const notify = vi.fn()
		service.on("notification", notify)
		await service.updateSettings({ lowLiquidityThresholdUsd: 100, swaps: false })
		expect(notify).not.toHaveBeenCalled()
	})

	it("sends encrypted Web Push payloads and forgets expired endpoints", async () => {
		const { service } = serviceAt(500)
		const subscription = {
			endpoint: "https://push.example/device",
			keys: { p256dh: "public-key", auth: "auth-secret" },
		}
		await service.subscribe(subscription)
		await expect(service.test()).resolves.toEqual({ pushSent: 1, pushFailed: 0 })
		expect(webPush.sendNotification).toHaveBeenCalledWith(
			subscription,
			expect.stringContaining("Simplex notifications are working"),
			expect.objectContaining({
				vapidDetails: {
					subject: "https://hyperbridge.network",
					publicKey: "vapid-public",
					privateKey: "vapid-private",
				},
			}),
		)

		webPush.sendNotification.mockRejectedValueOnce({ statusCode: 410 })
		await expect(service.test()).resolves.toEqual({ pushSent: 0, pushFailed: 1 })
		expect((await service.status()).subscriptionCount).toBe(0)
	})
})
