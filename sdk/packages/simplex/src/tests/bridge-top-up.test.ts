import { describe, expect, it, vi } from "vitest"
import { type AbiEvent, decodeAbiParameters, decodeFunctionData, encodeAbiParameters, encodeEventTopics, encodeFunctionData, fromHex, toHex } from "viem"
import { HyperFungibleTokenABI, IntentGatewayABI, transformOrderForContract, type HexString } from "@hyperbridge/sdk"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { MemoryDataStore } from "@/data/memory"
import type { StoredBridgeTopUp } from "@/data/types"
import {
	BridgeTopUp,
	type BridgeTopUpDeps,
	bridgeTopUpOutput,
	deserializeOrder,
	encodeBridgeTopUpCall,
	SIMPLEX_REFERRER,
	serializeOrder,
} from "@/services/BridgeTopUp"

const BRIDGE = "0x5b0c50fDd52ECC0d4c682c441eaBaD41FfDEABBB" as HexString
const RECIPIENT = `0x${"ab".repeat(32)}` as HexString

describe("BRIDGE top-up output call", () => {
	it("is one CallDispatcher call that sends the whole output to Hyperbridge with no timeout or fee", () => {
		const amount = 123_000_000n * 10n ** 12n
		const [calls] = decodeAbiParameters(
			[
				{
					type: "tuple[]",
					components: [
						{ name: "to", type: "address" },
						{ name: "value", type: "uint256" },
						{ name: "data", type: "bytes" },
					],
				},
			],
			encodeBridgeTopUpCall(BRIDGE, RECIPIENT, amount),
		)

		expect(calls).toHaveLength(1)
		expect(calls[0].to).toBe(BRIDGE)
		expect(calls[0].value).toBe(0n)

		const { functionName, args } = decodeFunctionData({ abi: HyperFungibleTokenABI, data: calls[0].data })
		expect(functionName).toBe("send")
		const [params] = args as [
			{ dest: HexString; to: HexString; amount: bigint; timeout: bigint; relayerFee: bigint; data: HexString },
		]
		expect(fromHex(params.dest, "string")).toBe("POLKADOT-3367")
		expect(params.to).toBe(RECIPIENT)
		expect(params.amount).toBe(amount)
		// A timeout would refund to the CallDispatcher, which anyone can drain.
		expect(params.timeout).toBe(0n)
		expect(params.relayerFee).toBe(0n)
		expect(params.data).toBe("0x")
	})
})

describe("SIMPLEX_REFERRER", () => {
	it("is Simplex as UTF-8, right-padded to bytes32", () => {
		expect(SIMPLEX_REFERRER).toBe(`0x53696d706c6578${"00".repeat(25)}`)
		expect(fromHex(SIMPLEX_REFERRER, { to: "string", size: 32 }).replace(/\0+$/, "")).toBe("Simplex")
	})
})

describe("bridgeTopUpOutput", () => {
	it("takes 1% off the quote and rounds down to a whole 12-decimal nexus unit", () => {
		const quoted = 100n * 10n ** 18n + 123_456_789n
		const out = bridgeTopUpOutput(quoted)
		expect(out % 10n ** 6n).toBe(0n)
		expect(out).toBeLessThanOrEqual((quoted * 99n) / 100n)
		expect((quoted * 99n) / 100n - out).toBeLessThan(10n ** 6n)
	})

	it("is zero for a quote below one nexus unit", () => {
		expect(bridgeTopUpOutput(999_999n)).toBe(0n)
	})
})

const ORDER = {
	user: `0x${"00".repeat(12)}${"11".repeat(20)}`,
	source: "EVM-8453",
	destination: "EVM-56",
	deadline: 123_456_789n,
	nonce: 0n,
	fees: 42n,
	session: "0x0000000000000000000000000000000000000000",
	predispatch: { assets: [], call: "0x" },
	inputs: [{ token: `0x${"22".repeat(32)}`, amount: 5_000_000n }],
	output: {
		beneficiary: `0x${"33".repeat(32)}`,
		assets: [{ token: `0x${"44".repeat(32)}`, amount: 99n * 10n ** 18n }],
		call: "0xabcd",
	},
} as const

function stored(stage: StoredBridgeTopUp["stage"], patch: Partial<StoredBridgeTopUp> = {}): StoredBridgeTopUp {
	return {
		id: "top-up",
		chain: "EVM-8453",
		token: "USDC",
		amountIn: "5",
		bridgeOut: "99",
		stage,
		createdAt: 1,
		updatedAt: 1,
		order: serializeOrder(ORDER as never),
		...patch,
	}
}

/** An `OrderPlaced` log for {@link ORDER}, as the gateway emits it with its own nonce. */
function orderPlacedLog() {
	const event = IntentGatewayABI.find((item) => item.type === "event" && item.name === "OrderPlaced") as AbiEvent
	const args: Record<string, unknown> = {
		user: ORDER.user,
		source: toHex(ORDER.source),
		destination: toHex(ORDER.destination),
		deadline: ORDER.deadline,
		nonce: 7n,
		fees: ORDER.fees,
		session: "0x0000000000000000000000000000000000000077",
		beneficiary: ORDER.output.beneficiary,
		predispatch: [],
		inputs: ORDER.inputs,
		outputs: ORDER.output.assets,
		predispatchCall: "0x",
		outputCall: ORDER.output.call,
		graffiti: `0x${"00".repeat(32)}`,
	}
	const indexed = event.inputs.filter((input) => input.indexed)
	const plain = event.inputs.filter((input) => !input.indexed)
	return {
		address: "0xAe041F7B0CB581876832830baeB6a2Aa2a3C9716",
		topics: encodeEventTopics({
			abi: [event],
			eventName: "OrderPlaced",
			args: Object.fromEntries(indexed.map((input) => [input.name!, args[input.name!]])),
		} as never),
		data: encodeAbiParameters(
			plain,
			plain.map((input) => args[input.name!]),
		),
	}
}

const FILL_TX = `0x${"88".repeat(32)}` as HexString
const SOLVER = "0xce319986ca4D5d0893751a628D0db3Dc8fc91d62" as HexString
const DELIVERY_COMMITMENT = `0x${"99".repeat(32)}` as HexString
/** The source gateway the SDK's cancel calldata targets, and the source host's fee token. */
const GATEWAY = "0x00000000000000000000000000000000000000ab" as HexString
const FEE_TOKEN = "0x00000000000000000000000000000000000000cd" as HexString
/** The relayer fee the SDK encodes into a cross-chain cancel's calldata. */
const RELAYER_FEE = 310_000n
const CANCEL_DATA = encodeFunctionData({
	abi: IntentGatewayABI,
	functionName: "cancelOrder",
	args: [transformOrderForContract(ORDER as never), { relayerFee: RELAYER_FEE, height: 12_345n }],
}) as HexString
const APPROVE_DATA = encodeFunctionData({ abi: ERC20_ABI, functionName: "approve", args: [GATEWAY, RELAYER_FEE] })

/** The HyperFungibleToken `Sent` log a fill's output call emits. */
function sentLog() {
	const event = HyperFungibleTokenABI.find((item) => item.type === "event" && item.name === "Sent") as AbiEvent
	const args: Record<string, unknown> = {
		from: "0xE2C7e576E26E0bE7aC97c6fE925bcDAbD87c4bEd",
		to: `0x${"ab".repeat(32)}`,
		dest: "POLKADOT-3367",
		amount: 99n * 10n ** 18n,
		commitment: DELIVERY_COMMITMENT,
	}
	const plain = event.inputs.filter((input) => !input.indexed)
	return {
		address: "0x5b0c50fDd52ECC0d4c682c441eaBaD41FfDEABBB",
		topics: encodeEventTopics({
			abi: [event],
			eventName: "Sent",
			args: Object.fromEntries(event.inputs.filter((i) => i.indexed).map((i) => [i.name!, args[i.name!]])),
		} as never),
		data: encodeAbiParameters(
			plain,
			plain.map((input) => args[input.name!]),
		),
	}
}

function topUpWith(record: StoredBridgeTopUp, chain: { receipt?: { logs: unknown[] } } = {}) {
	const data = new MemoryDataStore()
	void data.state.set({ bridgeTopUp: record })
	const refreshBridgeBalance = vi.fn(async () => {})
	const topUp = new BridgeTopUp({
		state: data.state,
		clientManager: {
			getPublicClient: () => ({
				waitForTransactionReceipt: async () => chain.receipt,
				getTransactionReceipt: async () => ({ transactionHash: FILL_TX, blockNumber: 100n, logs: [sentLog()] }),
			}),
		},
		configService: {
			getConfiguredChainIds: () => [8453, 56],
			getHostAddress: () => "0x620128E2B19193d6Bd244a3AC8D3bBa0541B19c3",
			getRpcUrl: () => "http://localhost:1",
			getBundlerUrl: () => undefined,
			getIntentGatewayAddress: () => "0xAe041F7B0CB581876832830baeB6a2Aa2a3C9716",
			getUsdcAsset: () => FEE_TOKEN,
			getUsdcDecimals: () => 6,
			getUsdtAsset: () => "0x",
			getUsdtDecimals: () => 6,
		},
		// Base is delegated; BSC still points at an older SolverAccount.
		delegation: { isDelegated: async (chain: string) => chain === "EVM-8453" },
		refreshBridgeBalance,
	} as unknown as BridgeTopUpDeps)
	// Hyperbridge already holds the request, as when another relayer delivered it.
	;(topUp as unknown as { hyperbridgeChain: Promise<unknown> }).hyperbridgeChain = Promise.resolve({
		queryRequestReceipt: async () => "0x01",
	})
	;(topUp as unknown as { ismpClient: () => unknown }).ismpClient = () => ({
		queryRequestWithStatus: async () => ({
			statuses: [
				{ status: "SOURCE", metadata: { timestamp: 1_791_898_135, transactionHash: FILL_TX } },
				{ status: "SOURCE_FINALIZED", metadata: { timestamp: 1_791_898_190, transactionHash: "0x01" } },
				{ status: "DESTINATION", metadata: { timestamp: 1_791_898_206_000, transactionHash: "0x02" } },
			],
		}),
	})
	// Stands in for the SDK gateway: the recovered order reads as already filled.
	;(topUp as unknown as { gateways: Map<string, Promise<unknown>> }).gateways.set(
		"EVM-8453",
		Promise.resolve({ isOrderFilled: async () => true }),
	)
	return { topUp, data, refreshBridgeBalance }
}

describe("BRIDGE top-up persistence", () => {
	it("round-trips an order's bigints through the JSON state store", () => {
		expect(deserializeOrder(serializeOrder(ORDER as never))).toEqual(ORDER)
	})

	it("refuses a second top-up while one is pending", async () => {
		const { topUp } = topUpWith(stored("unfilled"))
		await expect(topUp.start({ chain: "EVM-8453", token: "USDC", amount: "5" })).rejects.toThrow(
			"A BRIDGE top-up is already pending",
		)
	})

	it("hides the order and session key from the dashboard", async () => {
		const { topUp } = topUpWith(stored("unfilled", { sessionPrivateKey: `0x${"55".repeat(32)}` }))
		const current = await topUp.current()
		expect(current).toMatchObject({ stage: "unfilled", pending: true })
		expect(current).not.toHaveProperty("order")
		expect(current).not.toHaveProperty("sessionPrivateKey")
	})

	it("fails a placement interrupted before anything was sent: nothing is escrowed", async () => {
		const { topUp, data } = topUpWith(stored("placing"))
		const current = await topUp.current()
		expect(current?.stage).toBe("failed")
		expect(current?.pending).toBe(false)
		expect(current?.error).toContain("Nothing was escrowed")
		expect((await data.state.get()).bridgeTopUp?.stage).toBe("failed")
	})

	it("recovers an order whose placement landed but whose receipt read failed, then delivers it", async () => {
		const placeTxHash = `0x${"66".repeat(32)}` as HexString
		const { topUp, data, refreshBridgeBalance } = topUpWith(
			stored("failed", { placeTxHash, sessionPrivateKey: `0x${"55".repeat(32)}`, error: "receipt read failed" }),
			{ receipt: { logs: [orderPlacedLog()] } },
		)
		// The executor is stubbed out, so the fill is found through the indexer, as by a sync.
		// The indexer answers both lookups: the order's fill, and the request that fill dispatched.
		const fetchSpy = vi.spyOn(globalThis, "fetch").mockImplementation(async (_url, init) => {
			const query = String((init as RequestInit).body)
			const data = query.includes("requestV2s")
				? { requestV2s: { nodes: [{ commitment: DELIVERY_COMMITMENT }] } }
				: {
						iOrderV3s: {
							nodes: [
								{
									statusMetadata: {
										nodes: [{ status: "FILLED", timestamp: "100", transactionHash: FILL_TX, filler: SOLVER }],
									},
								},
							],
						},
					}
			return new Response(JSON.stringify({ data }))
		})
		await topUp.current()
		await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("delivered"))
		// The dashboard's BRIDGE balance is re-read the moment delivery lands, not a minute later.
		expect(refreshBridgeBalance).toHaveBeenCalledTimes(1)
		fetchSpy.mockRestore()
		const recovered = (await data.state.get()).bridgeTopUp!
		expect(recovered.commitment).toMatch(/^0x[0-9a-f]{64}$/)
		expect(deserializeOrder(recovered.order).nonce).toBe(7n)
		// Found through the gateway's OrderFilled log, then the HFT request inside that fill.
		expect(recovered.fillTxHash).toBe(FILL_TX)
		expect(recovered.deliveryCommitment).toBe(DELIVERY_COMMITMENT)
		expect(recovered.error).toBeUndefined()
		expect(recovered.stageTimes?.delivering).toBeTypeOf("number")
		expect(recovered.filler).toBe(SOLVER)
		// Statuses arrive in seconds and milliseconds; both are stored in milliseconds.
		expect(recovered.deliveryStatuses?.map((status) => [status.status, status.timestamp])).toEqual([
			["SOURCE", 1_791_898_135_000],
			["SOURCE_FINALIZED", 1_791_898_190_000],
			["DESTINATION", 1_791_898_206_000],
		])
	})

	it("finds a fill the indexer has not reached on BSC itself, near when it was seen", async () => {
		const seenAt = 1_791_900_416_000
		const { topUp } = topUpWith(
			stored("delivering", {
				commitment: `0x${"77".repeat(32)}`,
				events: [{ status: "FILLED", timestamp: seenAt }],
			}),
		)
		// The indexer has the order only as placed.
		const fetchSpy = vi
			.spyOn(globalThis, "fetch")
			.mockImplementation(
				async () => new Response(JSON.stringify({ data: { iOrderV3s: { nodes: [{ statusMetadata: { nodes: [] } }] } } })),
			)
		const ranges: [bigint, bigint][] = []
		const internals = topUp as unknown as {
			record: StoredBridgeTopUp
			bsc: () => unknown
			blockTimeSeconds: () => Promise<number>
			findFill: () => Promise<HexString | undefined>
		}
		await topUp.current()
		internals.blockTimeSeconds = async () => 0.5
		internals.bsc = () => ({
			// The latest block is 2 minutes after the fill was seen.
			getBlock: async () => ({ number: 1_000_000n, timestamp: BigInt(seenAt / 1000 + 120) }),
			getLogs: async ({ fromBlock, toBlock }: { fromBlock: bigint; toBlock: bigint }) => {
				ranges.push([fromBlock, toBlock])
				return fromBlock <= 999_700n && 999_700n <= toBlock ? [{ transactionHash: FILL_TX }] : []
			},
		})
		expect(await internals.findFill()).toBe(FILL_TX)
		// Searched from 3 minutes before the fill was seen, a few hundred blocks at a time.
		expect(ranges[0][0]).toBe(1_000_000n - 600n)
		expect(ranges.every(([from, to]) => to - from < 300n)).toBe(true)
		fetchSpy.mockRestore()
	})

	it("only pays from a network the filler fills on and is delegated on", async () => {
		const { topUp } = topUpWith(stored("filled"))
		for (const chain of ["EVM-56", "EVM-10"]) {
			await expect(topUp.start({ chain, token: "USDC", amount: "5" })).rejects.toThrow(
				`${chain} is not a network the filler fills on`,
			)
		}
	})

	it("places a new top-up while the last one is still reading its final statuses", async () => {
		const { topUp } = topUpWith(stored("placing", { id: "next", placeTxHash: undefined }))
		const internals = topUp as unknown as {
			driving: string | undefined
			drive: (flow: () => Promise<void>) => Promise<void>
		}
		await topUp.current()
		// The delivered top-up before it is still being driven, for its status backfill.
		internals.driving = "previous"
		let ran = false
		await internals.drive(async () => {
			ran = true
		})
		expect(ran).toBe(true)
	})

	it("recovers an unfilled cross-chain order and claims the refund a relayer did not deliver", async () => {
		const { topUp, data } = topUpWith(stored("unfilled", { commitment: `0x${"77".repeat(32)}` }))
		const handler = "0x0000000000000000000000000000000000000abc"
		const sent: { target: string; value: bigint; data: string }[][] = []
		let refunded = false
		const finalizedSeconds = Math.floor(Date.now() / 1000) - 600
		const internals = topUp as unknown as {
			gateways: Map<string, Promise<unknown>>
			deps: BridgeTopUpDeps
		}
		internals.gateways.set(
			"EVM-8453",
			Promise.resolve({
				isOrderRefunded: async () => refunded,
				async *cancelOrder() {
					yield { status: "DESTINATION_FINALIZED", proof: {} }
					const cancelTx: string = yield {
						status: "AWAITING_CANCEL_TRANSACTION",
						to: GATEWAY,
						data: CANCEL_DATA,
						// A non-zero native quote the cancel must not forward: the host pulls the fee token.
						value: 5n,
					}
					yield { status: "CANCEL_STARTED", receipt: { transactionHash: cancelTx } }
					yield { status: "SOURCE_FINALIZED", metadata: { transactionHash: "0xsf", timestamp: finalizedSeconds - 120 } }
					yield { status: "HYPERBRIDGE_DELIVERED", metadata: { transactionHash: "0xhd", timestamp: finalizedSeconds - 60 } }
					yield {
						status: "HYPERBRIDGE_FINALIZED",
						metadata: { transactionHash: "0xhf", timestamp: finalizedSeconds, calldata: "0xresponse" },
					}
				},
			}),
		)
		;(internals.deps as unknown as Record<string, unknown>).tokenSender = {
			submitCalls: async (_chain: string, calls: { target: string; value: bigint; data: string }[]) => {
				sent.push(calls)
				// The second batch is the claim; the escrow is refunded by it.
				if (sent.length === 2) refunded = true
				return { txHash: sent.length === 1 ? "0xcanceltx" : "0xclaimtx", sponsored: true }
			},
		}
		;(internals.deps.clientManager as unknown as Record<string, unknown>).getPublicClient = () => ({
			readContract: async ({ functionName }: { functionName: string }) => {
				if (functionName === "feeToken") return FEE_TOKEN
				if (functionName === "balanceOf") return 1_000_000n
				return { handler }
			},
		})
		const fetchSpy = vi
			.spyOn(globalThis, "fetch")
			.mockImplementation(
				async () => new Response(JSON.stringify({ data: { iOrderV3s: { nodes: [{ statusMetadata: { nodes: [] } }] } } })),
			)

		await topUp.reclaim()
		await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("reclaimed"))
		const record = (await data.state.get()).bridgeTopUp!
		expect(record.recoverySteps?.map((step) => [step.step, step.txHash])).toEqual([
			["DESTINATION_FINALIZED", undefined],
			["ORDER_CANCELED", "0xcanceltx"],
			["SOURCE_FINALIZED", "0xsf"],
			["HYPERBRIDGE_DELIVERED", "0xhd"],
			["HYPERBRIDGE_FINALIZED", "0xhf"],
			["FUNDS_CLAIMED", "0xclaimtx"],
		])
		// The cancel approves the gateway for the fee, then calls it with no value; the native quote is dropped.
		expect(sent[0]).toEqual([
			{ target: FEE_TOKEN, value: 0n, data: APPROVE_DATA },
			{ target: GATEWAY, value: 0n, data: CANCEL_DATA },
		])
		// Then Hyperbridge's response is submitted to the source host's handler.
		expect(sent[1]).toEqual([{ target: handler, value: 0n, data: "0xresponse" }])
		fetchSpy.mockRestore()
	})

	it("stops a reclaim that cannot pay the relayer fee in the fee token", async () => {
		vi.useFakeTimers()
		const fetchSpy = vi
			.spyOn(globalThis, "fetch")
			.mockImplementation(
				async () => new Response(JSON.stringify({ data: { iOrderV3s: { nodes: [{ statusMetadata: { nodes: [] } }] } } })),
			)
		try {
			const { topUp, data } = topUpWith(stored("unfilled", { commitment: `0x${"77".repeat(32)}` }))
			const submitted: unknown[] = []
			let cancelOrderCalls = 0
			const internals = topUp as unknown as {
				gateways: Map<string, Promise<unknown>>
				deps: BridgeTopUpDeps
			}
			internals.gateways.set(
				"EVM-8453",
				Promise.resolve({
					isOrderRefunded: async () => false,
					async *cancelOrder() {
						cancelOrderCalls += 1
						yield { status: "DESTINATION_FINALIZED", proof: {} }
						yield { status: "AWAITING_CANCEL_TRANSACTION", to: GATEWAY, data: CANCEL_DATA, value: 5n }
					},
				}),
			)
			;(internals.deps as unknown as Record<string, unknown>).tokenSender = {
				submitCalls: async () => {
					submitted.push(1)
					return { txHash: "0xcanceltx", sponsored: true }
				},
			}
			// The wallet holds 0.1 USDC; the cancel's relayer fee is 0.31.
			;(internals.deps.clientManager as unknown as Record<string, unknown>).getPublicClient = () => ({
				readContract: async ({ functionName }: { functionName: string }) => {
					if (functionName === "feeToken") return FEE_TOKEN
					if (functionName === "balanceOf") return 100_000n
					return {}
				},
			})

			await topUp.reclaim()
			// A short wallet fails before anything is sent; the first retry is a minute out.
			await vi.advanceTimersByTimeAsync(60_000)
			let record = (await data.state.get()).bridgeTopUp!
			expect(record.stage).toBe("reclaiming")
			expect(record.error).toContain("Recovery is retrying")
			expect(record.error).toContain("Not enough USDC on EVM-8453 to pay the cancel's relayer fee")

			for (let i = 0; i < 5; i++) await vi.advanceTimersByTimeAsync(60_000)
			record = (await data.state.get()).bridgeTopUp!
			expect(record.stage).toBe("unfilled")
			expect(record.error).toContain("Recovery stopped after 5 failed attempts")
			expect(record.error).toContain(
				"Not enough USDC on EVM-8453 to pay the cancel's relayer fee: have 0.1, need 0.31",
			)
			expect(submitted).toHaveLength(0)
			// The cap counted five attempts; no attempt was made without a scheduled retry.
			expect(cancelOrderCalls).toBe(5)

			// The cap stops the recovery: no retry is scheduled, so no further attempt runs.
			await vi.advanceTimersByTimeAsync(600_000)
			expect((await data.state.get()).bridgeTopUp?.stage).toBe("unfilled")
			expect(cancelOrderCalls).toBe(5)

			// An operator restart gets a fresh budget and resumes from the SDK's saved progress.
			await topUp.reclaim()
			await vi.waitFor(async () =>
				expect((await data.state.get()).bridgeTopUp?.error).toContain("Recovery is retrying"),
			)
			expect((await data.state.get()).bridgeTopUp?.stage).toBe("reclaiming")
		} finally {
			vi.clearAllTimers()
			fetchSpy.mockRestore()
			vi.useRealTimers()
		}
	})

	it("cancels a same-chain unfilled order in one call, with no relayer fee", async () => {
		const { topUp, data } = topUpWith(stored("unfilled", { chain: "EVM-56", commitment: `0x${"77".repeat(32)}` }))
		const cancelData = encodeFunctionData({
			abi: IntentGatewayABI,
			functionName: "cancelOrder",
			args: [transformOrderForContract(ORDER as never), { relayerFee: 0n, height: 0n }],
		}) as HexString
		const sent: { target: string; value: bigint; data: string }[][] = []
		let reads = 0
		const internals = topUp as unknown as {
			gateways: Map<string, Promise<unknown>>
			deps: BridgeTopUpDeps
		}
		internals.gateways.set(
			"EVM-56",
			Promise.resolve({
				isOrderRefunded: async () => false,
				async *cancelOrder() {
					const cancelTx: string = yield { status: "AWAITING_CANCEL_TRANSACTION", to: GATEWAY, data: cancelData, value: 0n }
					yield { status: "CANCELLATION_COMPLETE", transactionHash: cancelTx }
				},
			}),
		)
		;(internals.deps as unknown as Record<string, unknown>).tokenSender = {
			submitCalls: async (_chain: string, calls: { target: string; value: bigint; data: string }[]) => {
				sent.push(calls)
				return { txHash: "0xcanceltx", sponsored: true }
			},
		}
		// A same-chain cancel pays no relayer fee, so it reads no fee token or balance.
		;(internals.deps.clientManager as unknown as Record<string, unknown>).getPublicClient = () => ({
			readContract: async ({ functionName }: { functionName: string }) => {
				reads += 1
				throw new Error(`unexpected read: ${functionName}`)
			},
		})
		const fetchSpy = vi
			.spyOn(globalThis, "fetch")
			.mockImplementation(
				async () => new Response(JSON.stringify({ data: { iOrderV3s: { nodes: [{ statusMetadata: { nodes: [] } }] } } })),
			)

		await topUp.reclaim()
		await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("reclaimed"))
		expect(sent).toEqual([[{ target: GATEWAY, value: 0n, data: cancelData }]])
		expect(reads).toBe(0)
		fetchSpy.mockRestore()
	})

	it("does not cancel again once Hyperbridge finalized, only claims the refund", async () => {
		const finalizedAt = Date.now() - 5 * 60_000
		const claimCalldata = "0xresponse" as HexString
		const { topUp, data } = topUpWith(
			stored("unfilled", {
				commitment: `0x${"77".repeat(32)}`,
				reclaimTxHash: "0xcanceltx",
				claimCalldata,
				recoverySteps: [
					{ step: "DESTINATION_FINALIZED", timestamp: finalizedAt - 900_000 },
					{ step: "ORDER_CANCELED", txHash: "0xcanceltx", timestamp: finalizedAt - 700_000 },
					{ step: "SOURCE_FINALIZED", txHash: "0xsf", timestamp: finalizedAt - 300_000 },
					{ step: "HYPERBRIDGE_DELIVERED", txHash: "0xhd", timestamp: finalizedAt - 120_000 },
					{ step: "HYPERBRIDGE_FINALIZED", txHash: "0xhf", timestamp: finalizedAt },
				],
			}),
		)
		const handler = "0x0000000000000000000000000000000000000abc"
		const sent: { target: string; value: bigint; data: string }[][] = []
		let cancelOrderCalls = 0
		let refunded = false
		const internals = topUp as unknown as {
			gateways: Map<string, Promise<unknown>>
			deps: BridgeTopUpDeps
		}
		internals.gateways.set(
			"EVM-8453",
			Promise.resolve({
				isOrderRefunded: async () => refunded,
				async *cancelOrder() {
					cancelOrderCalls += 1
					yield { status: "AWAITING_CANCEL_TRANSACTION", to: GATEWAY, data: CANCEL_DATA, value: 5n }
				},
			}),
		)
		;(internals.deps as unknown as Record<string, unknown>).tokenSender = {
			submitCalls: async (_chain: string, calls: { target: string; value: bigint; data: string }[]) => {
				sent.push(calls)
				refunded = true
				return { txHash: "0xclaimtx", sponsored: true }
			},
		}
		;(internals.deps.clientManager as unknown as Record<string, unknown>).getPublicClient = () => ({
			readContract: async ({ functionName }: { functionName: string }) => {
				if (functionName === "hostParams") return { handler }
				throw new Error(`unexpected read: ${functionName}`)
			},
		})
		const fetchSpy = vi
			.spyOn(globalThis, "fetch")
			.mockImplementation(
				async () => new Response(JSON.stringify({ data: { iOrderV3s: { nodes: [{ statusMetadata: { nodes: [] } }] } } })),
			)

		await topUp.reclaim()
		await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("reclaimed"))
		// The cancel already went out; only Hyperbridge's response is claimed.
		expect(sent).toEqual([[{ target: handler, value: 0n, data: claimCalldata }]])
		expect(cancelOrderCalls).toBe(0)
		fetchSpy.mockRestore()
	})

	it("does not burn the reclaim cap on an attempt that waited on finality", async () => {
		vi.useFakeTimers()
		const fetchSpy = vi
			.spyOn(globalThis, "fetch")
			.mockImplementation(
				async () => new Response(JSON.stringify({ data: { iOrderV3s: { nodes: [{ statusMetadata: { nodes: [] } }] } } })),
			)
		try {
			const { topUp, data } = topUpWith(stored("unfilled", { commitment: `0x${"77".repeat(32)}` }))
			const internals = topUp as unknown as {
				gateways: Map<string, Promise<unknown>>
				deps: BridgeTopUpDeps
				reclaimFailures: number
			}
			internals.gateways.set(
				"EVM-8453",
				Promise.resolve({
					isOrderRefunded: async () => false,
					async *cancelOrder() {
						yield { status: "DESTINATION_FINALIZED", proof: {} }
						// A status stream that stalls past the long-attempt threshold, then fails.
						await new Promise((resolve) => setTimeout(resolve, 11 * 60_000))
						throw new Error("status stream stalled")
					},
				}),
			)

			await topUp.reclaim()
			// Seven attempts, each stalling 11 min then failing; a fast-failure cap would have stopped at five.
			for (let i = 0; i < 7; i++) await vi.advanceTimersByTimeAsync(12 * 60_000)
			const record = (await data.state.get()).bridgeTopUp!
			expect(record.stage).toBe("reclaiming")
			expect(record.error).toContain("Recovery is retrying")
			expect(internals.reclaimFailures).toBe(1)
		} finally {
			vi.clearAllTimers()
			fetchSpy.mockRestore()
			vi.useRealTimers()
		}
	})

	it("only reclaims an unfilled order", async () => {
		const { topUp } = topUpWith(stored("filled"))
		await expect(topUp.reclaim()).rejects.toThrow("There is no unfilled top-up to reclaim")
	})
})
