import { OrderCanceller } from "@/protocols/intents/OrderCanceller"
import { LEGACY_STORAGE_KEYS, STORAGE_KEYS, createCancellationStorage } from "@/storage"
import type { HexString, Order } from "@/types"
import { MissingConsensusUpdateTimeError } from "@/utils/exceptions"
import { describe, expect, it } from "vitest"

const ADDR_20 = "0xEa4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString

function makeOrder(overrides: Partial<Order> = {}): Order {
	return {
		id: "0xdeadbeef",
		user: ADDR_20,
		source: "EVM-1",
		destination: "EVM-42161",
		deadline: 100n,
		nonce: 0n,
		fees: 0n,
		session: "0x0000000000000000000000000000000000000000",
		predispatch: { assets: [], call: "0x" },
		inputs: [{ token: ADDR_20, amount: 1000n }],
		output: { beneficiary: ADDR_20, assets: [{ token: ADDR_20, amount: 990n }], call: "0x" },
		...overrides,
	}
}

describe("OrderCanceller recovery", () => {
	it("normalizes state-machine IDs in cancellation storage keys", () => {
		expect(STORAGE_KEYS.getRequest("0xdeadbeef", "EVM-1", "EVM-42161")).toBe(
			STORAGE_KEYS.getRequest("0xdeadbeef", "0x45564d2d31", "0x45564d2d3432313631"),
		)
	})

	it("persists destination cancellation checkpoints", async () => {
		const storage = createCancellationStorage({ env: "memory" })
		const key = STORAGE_KEYS.postCommitment("0xdeadbeef", "EVM-1", "EVM-42161")

		await storage.setItem(key, "0x1234")
		expect(await storage.getItem<string>(key)).toBe("0x1234")
	})

	it("migrates a legacy recovery checkpoint to the normalized key", async () => {
		const order = makeOrder({ source: "0x45564d2d31", destination: "0x45564d2d3432313631" })
		const orderId = "0xdeadbeef"
		const currentKey = STORAGE_KEYS.getRequest(orderId, order.source, order.destination)
		const legacyKey = LEGACY_STORAGE_KEYS.getRequest(orderId, order.source, order.destination)
		const values = new Map([[legacyKey, "checkpoint"]])
		const canceller = new OrderCanceller({
			cancellationStorage: {
				getItem: async (key: string) => values.get(key) ?? null,
				setItem: async (key: string, value: string) => void values.set(key, value),
				removeItem: async (key: string) => void values.delete(key),
			},
		} as never)
		const getRecoveryItem = (canceller as unknown as {
			getRecoveryItem<T>(key: string, legacyKeys: string[]): Promise<T | null>
		}).getRecoveryItem.bind(canceller)

		expect(await getRecoveryItem<string>(currentKey, [legacyKey])).toBe("checkpoint")
		expect(values.get(currentKey)).toBe("checkpoint")
		expect(values.has(legacyKey)).toBe(false)
	})

	it("clears GET recovery state but leaves POST state intact", async () => {
		const order = makeOrder()
		const orderId = "0xdeadbeef"
		const keys = {
			destProof: STORAGE_KEYS.destProof(orderId, order.source, order.destination),
			sourceProof: STORAGE_KEYS.sourceProof(orderId, order.source, order.destination),
			getRequest: STORAGE_KEYS.getRequest(orderId, order.source, order.destination),
			postCommitment: STORAGE_KEYS.postCommitment(orderId, order.source, order.destination),
		}
		const values = new Map(Object.values(keys).map((key) => [key, "cached"]))
		const canceller = new OrderCanceller({
			cancellationStorage: { removeItem: async (key: string) => void values.delete(key) },
		} as never)
		const clearGetRecoveryCache = (canceller as unknown as {
			clearGetRecoveryCache(recoveryOrder: Order): Promise<void>
		}).clearGetRecoveryCache.bind(canceller)

		await clearGetRecoveryCache(order)

		expect(values.has(keys.destProof)).toBe(false)
		expect(values.has(keys.sourceProof)).toBe(false)
		expect(values.has(keys.getRequest)).toBe(false)
		expect(values.has(keys.postCommitment)).toBe(true)
	})

	it("reports one bounded recovery restart before continuing", async () => {
		const canceller = new OrderCanceller({ cancellationStorage: { removeItem: async () => undefined } } as never)
		let attempts = 0
		const hooks = canceller as unknown as {
			cancelOrderFromSource(order: Order, indexerClient: unknown): AsyncGenerator<unknown>
		}
		hooks.cancelOrderFromSource = async function* () {
			attempts += 1
			if (attempts === 1) throw new MissingConsensusUpdateTimeError()
			yield { status: "AWAITING_CANCEL_TRANSACTION", data: "0x", to: ADDR_20, value: 0n }
		}

		const stream = canceller.cancelOrder(makeOrder(), {} as never)
		expect((await stream.next()).value).toMatchObject({ status: "RECOVERY_RESTARTED", attempt: 1 })
		expect((await stream.next()).value).toMatchObject({ status: "AWAITING_CANCEL_TRANSACTION" })
		expect(attempts).toBe(2)
	})

	it("stops after the configured number of recovery restarts", async () => {
		const canceller = new OrderCanceller({ cancellationStorage: { removeItem: async () => undefined } } as never)
		const hooks = canceller as unknown as {
			cancelOrderFromSource(order: Order, indexerClient: unknown): AsyncGenerator<unknown>
		}
		hooks.cancelOrderFromSource = async function* () {
			yield* []
			throw new MissingConsensusUpdateTimeError()
		}

		const stream = canceller.cancelOrder(makeOrder(), {} as never, { maxRecoveryRestarts: 1 })
		expect((await stream.next()).value).toMatchObject({ status: "RECOVERY_RESTARTED", attempt: 1 })
		await expect(stream.next()).rejects.toThrow("Cancellation recovery stopped after 1 restart")
	})

	it("uses the same-chain path without requiring an order ID", async () => {
		const canceller = new OrderCanceller({} as never)
		const hooks = canceller as unknown as {
			cancelOrderFromSource(order: Order, indexerClient: unknown): AsyncGenerator<unknown>
			cancelOrderFromDest(order: Order, indexerClient: unknown): AsyncGenerator<unknown>
		}
		hooks.cancelOrderFromSource = async function* () {
			yield { status: "AWAITING_CANCEL_TRANSACTION", data: "0x", to: ADDR_20, value: 0n }
		}
			hooks.cancelOrderFromDest = async function* () {
			yield* []
			throw new Error("destination path should not be selected")
		}

		const order = makeOrder({ id: undefined, destination: "0x45564d2d31" })
		expect((await canceller.cancelOrder(order, {} as never, { from: "destination" }).next()).value).toMatchObject({
			status: "AWAITING_CANCEL_TRANSACTION",
		})
	})
})
