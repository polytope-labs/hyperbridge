import { beforeEach, describe, expect, it, vi } from "vitest"
import type { HexString } from "@/types"

// Storage-slot discovery traces calls on a live node; here every token answers with one slot.
const TOKEN_BALANCE_SLOT = `0x${"ab".repeat(32)}` as HexString
vi.mock("@/utils", async (importOriginal) => ({
	...(await importOriginal<typeof import("@/utils")>()),
	getOrFetchStorageSlot: vi.fn(async () => TOKEN_BALANCE_SLOT),
}))

const { GasEstimator } = await import("@/protocols/intents/GasEstimator")

const GATEWAY = "0xAe041F7B0CB581876832830baeB6a2Aa2a3C9716" as HexString
const CNGN = "0x46c85152bfe9f96829aa94755d9f915f9b10ef5f" as HexString
const SOLVER = "0x9C7054b429f6b1dd35FD03e4fDC4f875Bc19931f" as HexString
// A mainnet order, and the slot that held its 999,500 escrow: `_orders[commitment][0]`.
const COMMITMENT = "0x0cfe2884884e8fd6752b0eab75b100f4111a2cc19018efb02891f16575c4d844" as HexString
const LEG0_ESCROW_SLOT = "0x7eae9e13a21d616d177f4e2573da33a918bfa4f615c8d57173b436ebd9797750"

const PARAMS_SLOT = `0x${"0".repeat(63)}5`

/** The gateway's params slot 5 with `solverSelection` set in byte 20, above `dispatcher`. */
const paramsWord = (dispatcher: HexString) => `0x${"00".repeat(11)}01${dispatcher.slice(2)}` as HexString

function estimator(options: { client?: unknown } = {}) {
	const ctx = {
		dest: {
			client: options.client ?? { getStorageAt: async () => paramsWord(`0x${"d1".repeat(20)}`) },
			configService: {
				getCalldispatcherAddress: () => "0xc71251c8b3e7b02697a84363eef6dce8dfbdf333",
				getSolverAccountAddress: () => undefined,
			},
		},
		solverCodeCache: new Map(),
	}
	return new GasEstimator(ctx as never, {} as never)
}

describe("GasEstimator.buildStateOverride escrow", () => {
	beforeEach(() => vi.clearAllMocks())

	it("gives the gateway the order's escrow and the input tokens to release it", async () => {
		const { bundler } = await estimator().buildStateOverride({
			accountAddress: SOLVER,
			chain: "EVM-8453",
			outputAssets: [],
			spenderAddress: GATEWAY,
			intentGatewayV2Address: GATEWAY,
			escrow: {
				commitment: COMMITMENT,
				inputs: [{ token: `0x000000000000000000000000${CNGN.slice(2)}` as HexString, amount: 1_000_000_000n }],
			},
		})

		expect(bundler[GATEWAY].stateDiff?.[LEG0_ESCROW_SLOT]).toBe(
			`0x${1_000_000_000n.toString(16).padStart(64, "0")}`,
		)
		// Params slot 5 is still rewritten with solver selection cleared.
		expect(bundler[GATEWAY].stateDiff?.[`0x${"0".repeat(63)}5`]).toBeDefined()
		expect(bundler[CNGN].stateDiff?.[TOKEN_BALANCE_SLOT]).toBe(`0x${(2n ** 128n).toString(16).padStart(64, "0")}`)
	})

	it("leaves escrow alone when none is given", async () => {
		const { bundler } = await estimator().buildStateOverride({
			accountAddress: SOLVER,
			chain: "EVM-8453",
			outputAssets: [],
			spenderAddress: GATEWAY,
			intentGatewayV2Address: GATEWAY,
		})

		expect(Object.keys(bundler[GATEWAY].stateDiff ?? {})).toEqual([`0x${"0".repeat(63)}5`])
		expect(bundler[CNGN]).toBeUndefined()
	})
})

describe("GasEstimator.buildStateOverride solver selection", () => {
	const override = (gasEstimator: InstanceType<typeof GasEstimator>) =>
		gasEstimator.buildStateOverride({
			accountAddress: SOLVER,
			chain: "EVM-97",
			outputAssets: [],
			spenderAddress: GATEWAY,
			intentGatewayV2Address: GATEWAY,
		})

	it("clears only the solverSelection byte of the gateway's params slot 5", async () => {
		// Bytes 21-31 are unused by the layout; they are set here to show they are kept as read.
		const word = `0x${"ee".repeat(11)}01${"d1".repeat(20)}` as HexString
		const getStorageAt = vi.fn(async (_request: unknown) => word)

		const { bundler } = await override(estimator({ client: { getStorageAt } }))

		expect(getStorageAt).toHaveBeenCalledWith({ address: GATEWAY, slot: PARAMS_SLOT })
		expect(bundler[GATEWAY].stateDiff?.[PARAMS_SLOT]).toBe(`0x${"ee".repeat(11)}00${"d1".repeat(20)}`)
	})

	it("keeps the gateway's own dispatcher, read on every estimate", async () => {
		const dispatchers = [`0x${"e2".repeat(20)}`, `0x${"2b".repeat(20)}`] as HexString[]
		const getStorageAt = vi
			.fn()
			.mockResolvedValueOnce(paramsWord(dispatchers[0]))
			.mockResolvedValueOnce(paramsWord(dispatchers[1]))
		const gasEstimator = estimator({ client: { getStorageAt } })

		for (const dispatcher of dispatchers) {
			const { bundler } = await override(gasEstimator)
			expect(bundler[GATEWAY].stateDiff?.[PARAMS_SLOT]).toBe(`0x${"00".repeat(12)}${dispatcher.slice(2)}`)
		}
		expect(getStorageAt).toHaveBeenCalledTimes(2)
	})
})
