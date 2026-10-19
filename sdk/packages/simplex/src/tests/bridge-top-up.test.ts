import { describe, expect, it, vi } from "vitest"
import {
	type AbiEvent,
	decodeAbiParameters,
	decodeFunctionData,
	encodeAbiParameters,
	encodeEventTopics,
	encodeFunctionData,
	fromHex,
	toHex,
	TransactionNotFoundError,
	TransactionReceiptNotFoundError,
} from "viem"
import { privateKeyToAccount } from "viem/accounts"
import {
	bytes20ToBytes32,
	getRequestCommitment,
	HyperFungibleTokenABI,
	IntentGatewayABI,
	transformOrderForContract,
	type HexString,
} from "@hyperbridge/sdk"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { EVM_HOST } from "@/config/abis/EvmHost"
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
import { SubmittedUnconfirmedError } from "@/services/submission"

const BRIDGE = "0x5b0c50fDd52ECC0d4c682c441eaBaD41FfDEABBB" as HexString
const RECIPIENT = `0x${"ab".repeat(32)}` as HexString
/** The session key a placement uses; its address is what `OrderPlaced.session` names. */
const SESSION_KEY = `0x${"55".repeat(32)}` as HexString
const SESSION_ADDRESS = privateKeyToAccount(SESSION_KEY).address

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

const SOLVER = "0xce319986ca4D5d0893751a628D0db3Dc8fc91d62" as HexString

const ORDER = {
	// The filler's own account, which identifies our placement when no session address is stored.
	user: bytes20ToBytes32(SOLVER),
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
function orderPlacedLog(patch: Record<string, unknown> = {}) {
	const event = IntentGatewayABI.find((item) => item.type === "event" && item.name === "OrderPlaced") as AbiEvent
	const args: Record<string, unknown> = {
		user: ORDER.user,
		source: toHex(ORDER.source),
		destination: toHex(ORDER.destination),
		deadline: ORDER.deadline,
		nonce: 7n,
		fees: ORDER.fees,
		// This filler's placement names the session key of `place()`'s placement.
		session: SESSION_ADDRESS,
		beneficiary: ORDER.output.beneficiary,
		predispatch: [],
		inputs: ORDER.inputs,
		outputs: ORDER.output.assets,
		predispatchCall: "0x",
		outputCall: ORDER.output.call,
		graffiti: `0x${"00".repeat(32)}`,
		...patch,
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
const APPROVE_ZERO_DATA = encodeFunctionData({ abi: ERC20_ABI, functionName: "approve", args: [GATEWAY, 0n] })

/** The IntentGateway and host the test config returns for every chain. */
const INTENT_GATEWAY = "0xAe041F7B0CB581876832830baeB6a2Aa2a3C9716" as HexString
const HOST = "0x620128E2B19193d6Bd244a3AC8D3bBa0541B19c3" as HexString

/** The commitment of the order the tests' records track. */
const ORDER_COMMITMENT = `0x${"77".repeat(32)}` as HexString
/** A cancel's `context` starts with the order's commitment (`abi.encode(commitment, user, ...)`). */
const cancelContext = (commitment: HexString) => `${commitment}${"00".repeat(32)}` as HexString

/** The GET request the source host dispatches for a cross-chain cancel, sent by the IntentGateway. */
const CANCEL_GET_REQUEST = {
	source: "EVM-8453",
	dest: "EVM-56",
	from: INTENT_GATEWAY.toLowerCase() as HexString,
	keys: ["0x1234"],
	height: 9_999n,
	nonce: 3n,
	timeoutTimestamp: 0n,
	context: cancelContext(ORDER_COMMITMENT),
} as const
const GET_COMMITMENT = getRequestCommitment({ ...CANCEL_GET_REQUEST, keys: [...CANCEL_GET_REQUEST.keys] })

/** The `GetRequestEvent` log in a cross-chain cancel's receipt. */
function getRequestLog(
	patch: { from?: HexString; address?: HexString; nonce?: bigint; context?: HexString } = {},
) {
	const event = EVM_HOST.find((item) => item.type === "event" && item.name === "GetRequestEvent") as AbiEvent
	const args: Record<string, unknown> = { ...CANCEL_GET_REQUEST, fee: 0n, ...patch }
	return {
		address: patch.address ?? HOST,
		topics: encodeEventTopics({ abi: [event], eventName: "GetRequestEvent" } as never),
		data: encodeAbiParameters(
			event.inputs,
			event.inputs.map((input) => args[input.name!]),
		),
	}
}

/** The HyperFungibleToken `Sent` log a fill's output call emits. */
function sentLog(patch: Record<string, unknown> = {}) {
	const event = HyperFungibleTokenABI.find((item) => item.type === "event" && item.name === "Sent") as AbiEvent
	const args: Record<string, unknown> = {
		from: "0xE2C7e576E26E0bE7aC97c6fE925bcDAbD87c4bEd",
		to: `0x${"ab".repeat(32)}`,
		dest: "POLKADOT-3367",
		amount: 99n * 10n ** 18n,
		commitment: DELIVERY_COMMITMENT,
		...patch,
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
				getTransaction: async () => ({ hash: FILL_TX }),
				getBlockNumber: async () => 100n,
			}),
		},
		configService: {
			getConfiguredChainIds: () => [8453, 56],
			getHostAddress: () => "0x620128E2B19193d6Bd244a3AC8D3bBa0541B19c3",
			getRpcUrl: () => "http://localhost:1",
			getRpcUrls: () => [],
			getBundlerUrl: () => undefined,
			getEntryPointAddress: () => "0x4337843378433784337843378433784337843378",
			getAssetBySymbol: (_chain: string, symbol: string) => (symbol === "BRIDGE" ? BRIDGE : undefined),
			getIntentGatewayAddress: () => "0xAe041F7B0CB581876832830baeB6a2Aa2a3C9716",
			getUsdcAsset: () => FEE_TOKEN,
			getUsdcDecimals: () => 6,
			getUsdtAsset: () => "0x",
			getUsdtDecimals: () => 6,
		},
		// Base is delegated; BSC still points at an older SolverAccount.
		delegation: { isDelegated: async (chain: string) => chain === "EVM-8453" },
		solver: SOLVER,
		// sentLog() addresses its `Sent` to this key, so a read receipt reads as this filler's.
		substratePublicKey: RECIPIENT,
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
		// Every saved snapshot, to check what a restart could have found.
		const snapshots: StoredBridgeTopUp[] = []
		const set = data.state.set.bind(data.state)
		data.state.set = async (state) => {
			if (state.bridgeTopUp) snapshots.push(structuredClone(state.bridgeTopUp))
			await set(state)
		}
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
					yield { status: "CANCEL_STARTED", receipt: { transactionHash: cancelTx, logs: [getRequestLog()] } }
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
		// The cancel resets the allowance, approves the gateway for the fee, then calls it with no value; the native quote is dropped.
		expect(sent[0]).toEqual([
			{ target: FEE_TOKEN, value: 0n, data: APPROVE_ZERO_DATA },
			{ target: FEE_TOKEN, value: 0n, data: APPROVE_DATA },
			{ target: GATEWAY, value: 0n, data: CANCEL_DATA },
		])
		// Then Hyperbridge's response is submitted to the source host's handler.
		expect(sent[1]).toEqual([{ target: handler, value: 0n, data: "0xresponse" }])
		// The cancel's GET request is kept for rebuilding the claim.
		expect(record.getCommitment).toBe(GET_COMMITMENT)
		// The finalized step and its calldata reach the store together, never one without the other.
		const finalized = snapshots.filter((snapshot) =>
			snapshot.recoverySteps?.some((step) => step.step === "HYPERBRIDGE_FINALIZED"),
		)
		expect(finalized.length).toBeGreaterThan(0)
		expect(finalized.every((snapshot) => snapshot.claimCalldata === "0xresponse")).toBe(true)
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

			// Each retry waits twice as long as the last: 2, 4, 8 and 16 min after the first minute.
			for (const minutes of [2, 4, 8]) {
				await vi.advanceTimersByTimeAsync(minutes * 60_000 - 1)
				expect((await data.state.get()).bridgeTopUp?.stage).toBe("reclaiming")
				await vi.advanceTimersByTimeAsync(1)
			}
			// The sixth attempt comes 16 min after the fifth, not before.
			const attemptsBeforeLast = cancelOrderCalls
			await vi.advanceTimersByTimeAsync(15 * 60_000)
			expect(cancelOrderCalls).toBe(attemptsBeforeLast)
			await vi.advanceTimersByTimeAsync(60_000)
			record = (await data.state.get()).bridgeTopUp!
			expect(record.stage).toBe("unfilled")
			expect(record.error).toContain("Recovery stopped after 6 failed attempts")
			expect(record.error).toContain(
				"Not enough USDC on EVM-8453 to pay the cancel's relayer fee: have 0.1, need 0.31",
			)
			expect(submitted).toHaveLength(0)
			// The cap counted six attempts over 31 minutes; none ran without a scheduled retry.
			expect(cancelOrderCalls).toBe(6)

			// The cap stops the recovery: no retry is scheduled, so no further attempt runs.
			await vi.advanceTimersByTimeAsync(600_000)
			expect((await data.state.get()).bridgeTopUp?.stage).toBe("unfilled")
			expect(cancelOrderCalls).toBe(6)

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

	/** An unfilled order whose Hyperbridge finalization is recorded, so only the refund claim is left. */
	function finalizedRecovery(patch: Partial<StoredBridgeTopUp>) {
		const finalizedAt = Date.now() - 5 * 60_000
		const { topUp, data } = topUpWith(
			stored("unfilled", {
				commitment: `0x${"77".repeat(32)}`,
				recoverySteps: [
					{ step: "DESTINATION_FINALIZED", timestamp: finalizedAt - 900_000 },
					{ step: "ORDER_CANCELED", txHash: "0xcanceltx", timestamp: finalizedAt - 700_000 },
					{ step: "SOURCE_FINALIZED", txHash: "0xsf", timestamp: finalizedAt - 300_000 },
					{ step: "HYPERBRIDGE_DELIVERED", txHash: "0xhd", timestamp: finalizedAt - 120_000 },
					{ step: "HYPERBRIDGE_FINALIZED", txHash: "0xhf", timestamp: finalizedAt },
				],
				...patch,
			}),
		)
		const handler = "0x0000000000000000000000000000000000000abc"
		const internals = topUp as unknown as {
			gateways: Map<string, Promise<unknown>>
			deps: BridgeTopUpDeps
			cancelClient: () => Promise<unknown>
		}
		const claims: { target: string; value: bigint; data: string }[] = []
		const rebuilt: string[] = []
		const state = { refunded: false, failClaims: 0, receiptLogs: [getRequestLog()] as unknown[] }
		internals.gateways.set(
			"EVM-8453",
			Promise.resolve({
				isOrderRefunded: async () => state.refunded,
				async *cancelOrder() {
					throw new Error("the cancel must not run again")
				},
			}),
		)
		;(internals.deps as unknown as Record<string, unknown>).tokenSender = {
			submitCalls: async (_chain: string, calls: { target: string; value: bigint; data: string }[]) => {
				claims.push(calls[0])
				if (state.failClaims > 0) {
					state.failClaims -= 1
					throw new Error("Send tx reverted: StateCommitmentNotFound")
				}
				state.refunded = true
				return { txHash: "0xclaimtx", sponsored: true }
			},
		}
		;(internals.deps.clientManager as unknown as Record<string, unknown>).getPublicClient = () => ({
			readContract: async () => ({ handler }),
			getTransactionReceipt: async () => ({ transactionHash: "0xcanceltx", logs: state.receiptLogs }),
		})
		// The SDK's snapshot of the GET request, proven at whatever height Hyperbridge has now.
		internals.cancelClient = async () => ({
			queryGetRequestWithStatus: async (commitment: string) => {
				rebuilt.push(commitment)
				return {
					statuses: [
						{ status: "SOURCE_FINALIZED", metadata: {} },
						{ status: "HYPERBRIDGE_FINALIZED", metadata: { calldata: `0xfresh${rebuilt.length}` } },
					],
				}
			},
		})
		return { topUp, data, handler, claims, rebuilt, state }
	}

	it("rebuilds the refund claim when a claim reverts, instead of replaying the stale calldata", async () => {
		vi.useFakeTimers()
		const fetchSpy = vi
			.spyOn(globalThis, "fetch")
			.mockImplementation(
				async () => new Response(JSON.stringify({ data: { iOrderV3s: { nodes: [{ statusMetadata: { nodes: [] } }] } } })),
			)
		try {
			const { topUp, data, handler, claims, rebuilt, state } = finalizedRecovery({
				getCommitment: GET_COMMITMENT,
				claimCalldata: "0xstale",
			})
			state.failClaims = 1

			await topUp.reclaim()
			// The recorded bytes go out first; they revert, as when the host's Hyperbridge client moved on.
			await vi.advanceTimersByTimeAsync(0)
			expect(claims).toEqual([{ target: handler, value: 0n, data: "0xstale" }])
			expect(rebuilt).toEqual([])
			expect((await data.state.get()).bridgeTopUp?.claimCalldata).toBeUndefined()

			// The retry asks the SDK for the response again and submits that.
			await vi.advanceTimersByTimeAsync(60_000)
			await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("reclaimed"))
			expect(rebuilt).toEqual([GET_COMMITMENT])
			expect(claims.map((claim) => claim.data)).toEqual(["0xstale", "0xfresh1"])
			expect((await data.state.get()).bridgeTopUp?.claimTxHash).toBe("0xclaimtx")
		} finally {
			vi.clearAllTimers()
			fetchSpy.mockRestore()
			vi.useRealTimers()
		}
	})

	it("rebuilds a refund claim whose calldata was never saved, as after a restart", async () => {
		const fetchSpy = vi
			.spyOn(globalThis, "fetch")
			.mockImplementation(
				async () => new Response(JSON.stringify({ data: { iOrderV3s: { nodes: [{ statusMetadata: { nodes: [] } }] } } })),
			)
		try {
			// Finalized is recorded, its calldata is not: a crash fell between the two writes.
			const { topUp, data, handler, claims, rebuilt } = finalizedRecovery({ getCommitment: GET_COMMITMENT })

			await topUp.current()
			await topUp.reclaim()
			await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("reclaimed"))
			expect(rebuilt).toEqual([GET_COMMITMENT])
			expect(claims).toEqual([{ target: handler, value: 0n, data: "0xfresh1" }])
			expect((await data.state.get()).bridgeTopUp?.claimCalldata).toBe("0xfresh1")
		} finally {
			fetchSpy.mockRestore()
		}
	})

	it("reads the cancel's GET request back from its receipt when it was not recorded", async () => {
		const fetchSpy = vi
			.spyOn(globalThis, "fetch")
			.mockImplementation(
				async () => new Response(JSON.stringify({ data: { iOrderV3s: { nodes: [{ statusMetadata: { nodes: [] } }] } } })),
			)
		try {
			const { topUp, data, rebuilt } = finalizedRecovery({})

			await topUp.reclaim()
			await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("reclaimed"))
			expect(rebuilt).toEqual([GET_COMMITMENT])
			expect((await data.state.get()).bridgeTopUp?.getCommitment).toBe(GET_COMMITMENT)
		} finally {
			fetchSpy.mockRestore()
		}
	})

	it("takes the IntentGateway's GET request from a bundle receipt, not another op's that comes first", async () => {
		const fetchSpy = vi
			.spyOn(globalThis, "fetch")
			.mockImplementation(
				async () => new Response(JSON.stringify({ data: { iOrderV3s: { nodes: [{ statusMetadata: { nodes: [] } }] } } })),
			)
		try {
			const { topUp, data, rebuilt, state } = finalizedRecovery({})
			state.receiptLogs = [
				// Another sender's request through this host, then the gateway's from a contract that is not the host.
				getRequestLog({ from: "0x00000000000000000000000000000000000000ee", nonce: 1n }),
				getRequestLog({ address: "0x00000000000000000000000000000000000000dd", nonce: 2n }),
				getRequestLog(),
			]

			await topUp.reclaim()
			await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("reclaimed"))
			expect(rebuilt).toEqual([GET_COMMITMENT])
			expect((await data.state.get()).bridgeTopUp?.getCommitment).toBe(GET_COMMITMENT)
		} finally {
			fetchSpy.mockRestore()
		}
	})

	it("takes this order's cancel from a bundle where another order's cancel through the gateway comes first", async () => {
		const fetchSpy = vi
			.spyOn(globalThis, "fetch")
			.mockImplementation(
				async () => new Response(JSON.stringify({ data: { iOrderV3s: { nodes: [{ statusMetadata: { nodes: [] } }] } } })),
			)
		try {
			const { topUp, data, rebuilt, state } = finalizedRecovery({})
			state.receiptLogs = [
				getRequestLog({ nonce: 1n, context: cancelContext(`0x${"55".repeat(32)}`) }),
				getRequestLog(),
			]

			await topUp.reclaim()
			await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("reclaimed"))
			expect(rebuilt).toEqual([GET_COMMITMENT])
			expect((await data.state.get()).bridgeTopUp?.getCommitment).toBe(GET_COMMITMENT)
		} finally {
			fetchSpy.mockRestore()
		}
	})

	it("fails to read the cancel's GET request when the receipt has none from the IntentGateway", async () => {
		const { topUp } = finalizedRecovery({})
		const internals = topUp as unknown as {
			deps: BridgeTopUpDeps
			cancelCommitment: (chain: string) => Promise<string>
		}
		;(internals.deps.clientManager as unknown as Record<string, unknown>).getPublicClient = () => ({
			getTransactionReceipt: async () => ({
				transactionHash: "0xcanceltx",
				logs: [getRequestLog({ from: "0x00000000000000000000000000000000000000ee" })],
			}),
		})

		await topUp.current()
		await expect(internals.cancelCommitment("EVM-8453")).rejects.toThrow("dispatched no GET request")
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

const USER_OP_HASH = `0x${"aa".repeat(32)}` as HexString
const PLACE_TX = `0x${"bb".repeat(32)}` as HexString
const SCAN_TX = `0x${"cc".repeat(32)}` as HexString
/** A session key that is not this filler's, for a receipt carrying someone else's placement. */
const OTHER_SESSION = `0x${"77".repeat(20)}` as HexString

/** The internals these recovery tests reach into, as the file's other tests do. */
type TopUpInternals = {
	record: StoredBridgeTopUp
	gateways: Map<string, Promise<unknown>>
	deps: BridgeTopUpDeps
	bsc: () => unknown
	place: (order: unknown, tokenIn: HexString) => Promise<void>
	deliveryRequest: (fillTxHash: HexString) => Promise<HexString>
	resumeDelivery: () => Promise<void>
}

/** A gateway that yields a placement transaction, reports no fill, and resumes into nothing. */
function placementGateway(overrides: Record<string, unknown> = {}) {
	return {
		executeBest: () =>
			(async function* () {
				yield {
					status: "AWAITING_PLACE_ORDER",
					sessionPrivateKey: SESSION_KEY,
					to: GATEWAY,
					data: "0xdead",
					value: 0n,
					feeTokenAmount: 0n,
					feeTokenAddress: FEE_TOKEN,
				}
			})(),
		isOrderFilled: async () => false,
		async *resumeBest() {},
		...overrides,
	}
}

/** A paying-chain client the recovery reads: its receipt, its scan, its EntryPoint nonce and tx. */
function payingClient(
	opts: {
		receipt?: { status?: string; logs: unknown[] }
		receiptError?: unknown
		logs?: unknown[]
		getLogsError?: unknown
		latestBlock?: bigint
		nonce?: bigint
		/** The EntryPoint nonce, as it depends on the height the read was pinned to (undefined = latest). */
		nonceAt?: (blockNumber: bigint | undefined) => bigint
		getTransaction?: () => Promise<unknown>
		accountNonce?: number
		/** The wallet's nonce, as it depends on the height the read was pinned to (undefined = latest). */
		accountNonceAt?: (blockNumber: bigint | undefined) => number
		/** Records each scan page, for tests that assert where a scan starts. */
		onGetLogs?: (range: { fromBlock: bigint; toBlock: bigint }) => void
		/** The chain's clock in seconds at the block read; defaults to this host's. */
		blockTimestamp?: (blockNumber: bigint) => number
	} = {},
) {
	return {
		getBlock: async ({ blockNumber }: { blockNumber: bigint }) => ({
			timestamp: BigInt(opts.blockTimestamp ? opts.blockTimestamp(blockNumber) : Math.floor(Date.now() / 1000)),
		}),
		waitForTransactionReceipt: async () => {
			if (opts.receiptError !== undefined) throw opts.receiptError
			return opts.receipt ?? { logs: [] }
		},
		getBlockNumber: async () => opts.latestBlock ?? 100n,
		getLogs: async (args: { fromBlock: bigint; toBlock: bigint }) => {
			opts.onGetLogs?.(args)
			if (opts.getLogsError !== undefined) throw opts.getLogsError
			return opts.logs ?? []
		},
		// A pending tx: `blockNumber` is null until it is mined, as viem reports it.
		getTransaction: opts.getTransaction ?? (async () => ({ hash: PLACE_TX, nonce: 0, blockNumber: null })),
		getTransactionCount: async (args: { blockNumber?: bigint }) =>
			opts.accountNonceAt ? opts.accountNonceAt(args.blockNumber) : opts.accountNonce ?? 0,
		readContract: async (args: { blockNumber?: bigint }) =>
			opts.nonceAt ? opts.nonceAt(args.blockNumber) : opts.nonce ?? 0n,
	}
}

/** Installs a paying-chain client built from `opts`. */
function installPayingClient(internals: TopUpInternals, opts: Parameters<typeof payingClient>[0] = {}) {
	;(internals.deps.clientManager as unknown as Record<string, unknown>).getPublicClient = () => payingClient(opts)
}

/** A placing record for a UserOp placement that never got a receipt. */
function userOpPlacement(patch: Record<string, unknown> = {}) {
	return stored("placing", {
		placeUserOpHash: USER_OP_HASH,
		placeUserOpNonce: "3",
		placeUserOpNonceKey: "0",
		placeUserOpExpires: true,
		placeSubmittedAt: Date.now() - 2 * 60 * 60_000,
		placeSession: SESSION_ADDRESS,
		placeFromBlock: "10",
		sessionPrivateKey: SESSION_KEY,
		...patch,
	})
}

/** The `OrderPlaced` log a scan returns for this filler's session. */
function scannedPlacement() {
	return { transactionHash: SCAN_TX, args: { session: SESSION_ADDRESS } }
}

/** A placing record for a native placement tx that got no receipt. */
function nativePlacement(patch: Record<string, unknown> = {}) {
	return stored("placing", {
		placeTxHash: PLACE_TX,
		placeSession: SESSION_ADDRESS,
		placeFromBlock: "10",
		placeSubmittedAt: Date.now() - 2 * 60 * 60_000,
		...patch,
	})
}

/** A paying-chain node that cannot read the receipt and has no record of the tx. */
function forgottenTx() {
	return {
		receiptError: new Error("receipt not readable"),
		logs: [],
		getTransaction: async () => {
			throw new TransactionNotFoundError({ hash: PLACE_TX })
		},
	}
}

describe("BRIDGE top-up placement recovery", () => {
	it("keeps a top-up pending while a sponsored placement has no receipt, then recovers it", async () => {
		vi.useFakeTimers()
		try {
			const record = stored("placing")
			const { topUp, data } = topUpWith(record)
			const internals = topUp as unknown as TopUpInternals
			internals.record = record
			internals.gateways.set("EVM-8453", Promise.resolve(placementGateway()))
			const userOpReceipt = vi.fn(async () => ({ txHash: PLACE_TX, success: true }))
			internals.deps.tokenSender = {
				submitCalls: async () => {
					throw new SubmittedUnconfirmedError(
						`Sponsored UserOp ${USER_OP_HASH} submitted but no receipt within timeout`,
						"EVM-8453",
						{ kind: "userOp", userOpHash: USER_OP_HASH, nonce: 0n, nonceKey: 0n, expires: true, submittedAt: Date.now() },
					)
				},
				userOpReceipt,
			} as never
			installPayingClient(internals, { receipt: { logs: [orderPlacedLog()] } })

			await internals.place(deserializeOrder(record.order), FEE_TOKEN)

			let saved = (await data.state.get()).bridgeTopUp!
			expect(saved.stage).toBe("placing")
			expect(saved.placeUserOpHash).toBe(USER_OP_HASH)
			expect(saved.placeUserOpNonce).toBe("0")
			expect(saved.placeUserOpExpires).toBe(true)
			// The session address names this placement's log, and the block bounds the scan.
			expect(saved.placeSession).toBe(SESSION_ADDRESS)
			expect(saved.placeFromBlock).toBe("100")
			expect(saved.error).toContain("Placement is unconfirmed")
			// Pending, so another top-up is refused without a restart.
			await expect(topUp.start({ chain: "EVM-8453", token: "USDC", amount: "5" })).rejects.toThrow(
				"A BRIDGE top-up is already pending",
			)

			// The retry finds the op's receipt and tracks the order it placed.
			await vi.advanceTimersByTimeAsync(60_000)
			saved = (await data.state.get()).bridgeTopUp!
			expect(saved.stage).toBe("awaiting_bids")
			expect(saved.placeTxHash).toBe(PLACE_TX)
			expect(userOpReceipt).toHaveBeenCalled()
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("keeps a top-up pending while a native placement has no receipt, then recovers it", async () => {
		vi.useFakeTimers()
		try {
			const record = stored("placing")
			const { topUp, data } = topUpWith(record)
			const internals = topUp as unknown as TopUpInternals
			internals.record = record
			internals.gateways.set("EVM-8453", Promise.resolve(placementGateway()))
			internals.deps.tokenSender = {
				submitCalls: async () => {
					throw new SubmittedUnconfirmedError(`Timed out waiting for ${PLACE_TX}`, "EVM-8453", {
						kind: "tx",
						txHash: PLACE_TX,
						nonce: 4,
						submittedAt: Date.now(),
					})
				},
				userOpReceipt: async () => null,
			} as never
			installPayingClient(internals, { receipt: { logs: [orderPlacedLog()] } })

			await internals.place(deserializeOrder(record.order), FEE_TOKEN)

			let saved = (await data.state.get()).bridgeTopUp!
			expect(saved.stage).toBe("placing")
			expect(saved.placeTxHash).toBe(PLACE_TX)
			// The wallet nonce it was sent with is what later shows the tx can no longer be mined.
			expect(saved.placeTxNonce).toBe(4)
			expect(saved.error).toContain("Placement is unconfirmed")

			await vi.advanceTimersByTimeAsync(60_000)
			saved = (await data.state.get()).bridgeTopUp!
			expect(saved.stage).toBe("awaiting_bids")
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("picks this filler's OrderPlaced out of a receipt that carries another user's first", async () => {
		const placeTxHash = `0x${"66".repeat(32)}` as HexString
		const { topUp, data } = topUpWith(
			stored("placing", { placeTxHash, placeSession: SESSION_ADDRESS, placeSubmittedAt: Date.now() }),
		)
		const internals = topUp as unknown as TopUpInternals
		installPayingClient(internals, {
			receipt: { logs: [orderPlacedLog({ session: OTHER_SESSION, nonce: 9n }), orderPlacedLog()] },
		})

		await topUp.current()

		await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("awaiting_bids"))
		const saved = (await data.state.get()).bridgeTopUp!
		// The second log is ours: its nonce, not the other user's 9, and its session.
		expect(deserializeOrder(saved.order).nonce).toBe(7n)
		expect(deserializeOrder(saved.order).session).toBe(SESSION_ADDRESS)
	})

	it("settles a mined placement that placed no order, and does not retry", async () => {
		vi.useFakeTimers()
		try {
			const placeTxHash = `0x${"66".repeat(32)}` as HexString
			const { topUp, data } = topUpWith(
				stored("placing", { placeTxHash, placeSession: OTHER_SESSION, placeSubmittedAt: Date.now() }),
			)
			const internals = topUp as unknown as TopUpInternals
			let receipts = 0
			;(internals.deps.clientManager as unknown as Record<string, unknown>).getPublicClient = () => ({
				...payingClient(),
				waitForTransactionReceipt: async () => {
					receipts += 1
					return { logs: [orderPlacedLog()] }
				},
			})

			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)

			let saved = (await data.state.get()).bridgeTopUp!
			expect(saved.stage).toBe("failed")
			expect(saved.error).toContain("was mined but placed no order")
			expect(saved.error).toContain("Nothing was escrowed")

			// Settled, not retried: no timer fires another receipt read.
			await vi.advanceTimersByTimeAsync(120_000)
			saved = (await data.state.get()).bridgeTopUp!
			expect(saved.stage).toBe("failed")
			expect(receipts).toBe(1)
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("settles a reverted placement as failed", async () => {
		const placeTxHash = `0x${"66".repeat(32)}` as HexString
		const { topUp, data } = topUpWith(
			stored("placing", { placeTxHash, placeSession: SESSION_ADDRESS, placeSubmittedAt: Date.now() }),
		)
		const internals = topUp as unknown as TopUpInternals
		installPayingClient(internals, { receipt: { status: "reverted", logs: [] } })

		await topUp.current()

		await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("failed"))
		expect((await data.state.get()).bridgeTopUp?.error).toContain("was mined but placed no order")
	})

	it("recovers from the scan when the bundler does not know the placement op", async () => {
		const { topUp, data } = topUpWith(userOpPlacement())
		const internals = topUp as unknown as TopUpInternals
		internals.gateways.set("EVM-8453", Promise.resolve(placementGateway()))
		internals.deps.tokenSender = { userOpReceipt: async () => null } as never
		installPayingClient(internals, { logs: [scannedPlacement()], receipt: { logs: [orderPlacedLog()] } })

		await topUp.current()

		await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("awaiting_bids"))
		expect((await data.state.get()).bridgeTopUp?.placeTxHash).toBe(SCAN_TX)
	})

	it("fails a placement with no log once its EntryPoint nonce has moved on", async () => {
		const { topUp, data } = topUpWith(userOpPlacement({ placeUserOpExpires: false }))
		const internals = topUp as unknown as TopUpInternals
		internals.deps.tokenSender = { userOpReceipt: async () => null } as never
		installPayingClient(internals, { logs: [], nonce: 4n })

		await topUp.current()

		await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("failed"))
		expect((await data.state.get()).bridgeTopUp?.error).toContain("nonce has moved on")
		expect((await data.state.get()).bridgeTopUp?.error).toContain("Nothing was escrowed")
	})

	it("fails a Permit2 op with no log once its permit window has passed", async () => {
		const { topUp, data } = topUpWith(userOpPlacement())
		const internals = topUp as unknown as TopUpInternals
		internals.deps.tokenSender = { userOpReceipt: async () => null } as never
		installPayingClient(internals, { logs: [], nonce: 3n })

		await topUp.current()

		await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("failed"))
		expect((await data.state.get()).bridgeTopUp?.error).toContain("permit expired")
	})

	it("keeps a bootstrap op pending: it has no expiry and can still land", async () => {
		vi.useFakeTimers()
		try {
			const { topUp, data } = topUpWith(userOpPlacement({ placeUserOpExpires: false }))
			const internals = topUp as unknown as TopUpInternals
			internals.deps.tokenSender = { userOpReceipt: async () => null } as never
			installPayingClient(internals, { logs: [], nonce: 3n })

			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)

			const saved = (await data.state.get()).bridgeTopUp!
			expect(saved.stage).toBe("placing")
			expect(saved.error).toContain("Placement is unconfirmed")
			expect((await topUp.current())?.pending).toBe(true)
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("keeps retrying while the placement scan cannot be read", async () => {
		vi.useFakeTimers()
		try {
			const { topUp, data } = topUpWith(userOpPlacement())
			const internals = topUp as unknown as TopUpInternals
			internals.deps.tokenSender = { userOpReceipt: async () => null } as never
			installPayingClient(internals, { getLogsError: new Error("range too wide") })

			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)

			const saved = (await data.state.get()).bridgeTopUp!
			expect(saved.stage).toBe("placing")
			expect(saved.error).toContain("Placement is unconfirmed")
			expect(saved.error).toContain("range too wide")
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("keeps waiting on a native placement the node has forgotten while the wallet's nonce has not moved", async () => {
		vi.useFakeTimers()
		try {
			const { topUp, data } = topUpWith(nativePlacement({ placeTxNonce: 4 }))
			const internals = topUp as unknown as TopUpInternals
			// One backend or mempool lacks the tx, but its nonce is still free, so it may yet be mined.
			installPayingClient(internals, { ...forgottenTx(), accountNonce: 4 })

			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)

			const saved = (await data.state.get()).bridgeTopUp!
			expect(saved.stage).toBe("placing")
			expect(saved.error).toContain("Placement is unconfirmed")
			expect(saved.error).not.toContain("Nothing was escrowed")
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("fails a forgotten native placement once its nonce is consumed and the scan found nothing", async () => {
		const { topUp, data } = topUpWith(nativePlacement({ placeTxNonce: 4 }))
		const internals = topUp as unknown as TopUpInternals
		installPayingClient(internals, { ...forgottenTx(), accountNonce: 5 })

		await topUp.current()

		await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("failed"))
		expect((await data.state.get()).bridgeTopUp?.error).toContain("nonce has moved past it")
		expect((await data.state.get()).bridgeTopUp?.error).toContain("Nothing was escrowed")
	})

	it("settles a legacy native placement the node does not know as failed, unverified, so it cannot block new top-ups", async () => {
		// A record from before nonces were kept: no nonce, session or start block to judge it by.
		const { topUp, data } = topUpWith(stored("placing", { placeTxHash: PLACE_TX }))
		const internals = topUp as unknown as TopUpInternals
		installPayingClient(internals, { ...forgottenTx(), accountNonce: 99 })

		await topUp.current()

		await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("failed"))
		const { error } = (await data.state.get()).bridgeTopUp!
		expect(error).toContain(PLACE_TX)
		expect(error).toContain("predates nonce tracking")
		expect(error).toContain("could not be verified")
		expect(error).toContain("Check the tx on an explorer before starting another top-up")
		expect(error).not.toContain("Nothing was escrowed")
		expect((await topUp.current())?.pending).toBe(false)
	})

	it("takes a legacy native placement's nonce from the tx the node still knows, then applies the nonce rule", async () => {
		const legacy = () => stored("placing", { placeTxHash: PLACE_TX })
		const pending = (nonce: number) => async () => ({ hash: PLACE_TX, nonce, blockNumber: null })

		// The wallet's nonce is still the tx's: it may yet be mined, and the learned nonce is kept.
		vi.useFakeTimers()
		try {
			const { topUp, data } = topUpWith(legacy())
			installPayingClient(topUp as unknown as TopUpInternals, {
				receiptError: new Error("receipt not readable"),
				getTransaction: pending(4),
				accountNonce: 4,
			})
			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)
			const waiting = (await data.state.get()).bridgeTopUp!
			expect(waiting.stage).toBe("placing")
			expect(waiting.placeTxNonce).toBe(4)
			expect(waiting.error).toContain("is not mined yet")
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}

		// The wallet's nonce is past it and nothing was mined: replaced or dropped.
		const { topUp, data } = topUpWith(legacy())
		installPayingClient(topUp as unknown as TopUpInternals, {
			receiptError: new Error("receipt not readable"),
			getTransaction: pending(4),
			accountNonce: 5,
		})
		await topUp.current()
		await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("failed"))
		const settled = (await data.state.get()).bridgeTopUp!
		expect(settled.placeTxNonce).toBe(4)
		expect(settled.error).toContain("nonce has moved past it")
	})

	it("keeps waiting on a forgotten native placement that has a recorded nonce, whatever its age", async () => {
		vi.useFakeTimers()
		try {
			const { topUp, data } = topUpWith(
				nativePlacement({ placeTxNonce: 4, placeSubmittedAt: Date.now() - 5 * 60 * 60_000 }),
			)
			installPayingClient(topUp as unknown as TopUpInternals, { ...forgottenTx(), accountNonce: 4 })

			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)

			const saved = (await data.state.get()).bridgeTopUp!
			expect(saved.stage).toBe("placing")
			expect(saved.error).toContain("Placement is unconfirmed")
			expect(saved.error).not.toContain("predates nonce tracking")
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("waits, never judging against nonce 0, for a placement UserOp that has no recorded nonce", async () => {
		vi.useFakeTimers()
		try {
			const { topUp, data } = topUpWith(userOpPlacement({ placeUserOpExpires: false, placeUserOpNonce: undefined }))
			const internals = topUp as unknown as TopUpInternals
			internals.deps.tokenSender = { userOpReceipt: async () => null } as never
			// Any nonce above 0 would have settled the op as consumed had it defaulted to 0.
			installPayingClient(internals, { logs: [], nonce: 5n })

			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)

			const saved = (await data.state.get()).bridgeTopUp!
			expect(saved.stage).toBe("placing")
			expect(saved.error).toContain("no recorded nonce")
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("retries a native placement when the node read itself fails", async () => {
		vi.useFakeTimers()
		try {
			const placeTxHash = `0x${"66".repeat(32)}` as HexString
			const { topUp, data } = topUpWith(
				stored("placing", {
					placeTxHash,
					placeSession: SESSION_ADDRESS,
					placeFromBlock: "10",
					placeSubmittedAt: Date.now() - 2 * 60 * 60_000,
				}),
			)
			const internals = topUp as unknown as TopUpInternals
			installPayingClient(internals, {
				receiptError: new Error("receipt not readable"),
				logs: [],
				getTransaction: async () => {
					throw new Error("rpc exploded")
				},
			})

			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)

			const saved = (await data.state.get()).bridgeTopUp!
			expect(saved.stage).toBe("placing")
			expect(saved.error).toContain("rpc exploded")
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("keeps a legacy placement failure failed when it mined no order, never unfilled", async () => {
		const placeTxHash = `0x${"66".repeat(32)}` as HexString
		const { topUp, data } = topUpWith(
			stored("failed", { placeTxHash, sessionPrivateKey: SESSION_KEY, error: "receipt read failed" }),
		)
		const internals = topUp as unknown as TopUpInternals
		installPayingClient(internals, { receipt: { logs: [] } })

		await topUp.current()

		await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("failed"))
		// A reclaim of a never-placed order would mark it reclaimed (X1); it must stay failed.
		expect((await data.state.get()).bridgeTopUp?.error).toContain("was mined but placed no order")
	})

	it("does not drive a new top-up from a legacy record's retry timer", async () => {
		vi.useFakeTimers()
		try {
			const placeTxHash = `0x${"66".repeat(32)}` as HexString
			const { topUp, data } = topUpWith(
				stored("failed", { placeTxHash, sessionPrivateKey: SESSION_KEY, error: "receipt read failed" }),
			)
			const internals = topUp as unknown as TopUpInternals
			// The receipt read keeps failing, so the record arms a retry timer.
			installPayingClient(internals, { receiptError: new Error("receipt read failed") })

			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)
			expect((await data.state.get()).bridgeTopUp?.stage).toBe("failed")
			expect((await data.state.get()).bridgeTopUp?.error).toContain("Placement is unconfirmed")

			// The operator starts a new top-up, which start() records in place of the old one.
			const next = stored("placing", { id: "next" })
			internals.record = next
			await data.state.set({ bridgeTopUp: next })

			// The old record's timer fires, but must leave the new record alone.
			await vi.advanceTimersByTimeAsync(60_000)
			expect(internals.record.id).toBe("next")
			expect(internals.record.stage).toBe("placing")
			expect((await data.state.get()).bridgeTopUp?.id).toBe("next")
			expect((await data.state.get()).bridgeTopUp?.stage).toBe("placing")
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	/**
	 * A reclaim whose first cancel is sent without a receipt, and then a retry a minute later.
	 * `client` is what the paying chain says about that cancel by then.
	 */
	async function retriedCancel(
		submission: SubmittedUnconfirmedError["submission"] | undefined,
		client: {
			receipt?: { status: "success" | "reverted" }
			accountNonce?: number
			entryPointNonce?: bigint
			userOpReceipt?: { txHash: string; success: boolean } | null
			/** The bundler cannot be asked: the lookup throws instead of answering. */
			userOpReceiptError?: Error
			/** The chain's clock in seconds at the evidence block. */
			blockTimestamp?: number
			/** A pending tx the node still lists; any other hash is unknown to it. */
			pendingTx?: { hash: string; nonce: number }
		},
		/** The record at the start; a cancel hash from older code sits here instead of being sent. */
		patch: Partial<StoredBridgeTopUp> = {},
	) {
		vi.useFakeTimers()
		const { topUp, data } = topUpWith(stored("unfilled", { commitment: `0x${"77".repeat(32)}`, ...patch }))
		const internals = topUp as unknown as TopUpInternals
		const handedBack: string[] = []
		internals.gateways.set(
			"EVM-8453",
			Promise.resolve({
				isOrderRefunded: async () => false,
				async *cancelOrder() {
					yield { status: "DESTINATION_FINALIZED", proof: {} }
					const input: string = yield { status: "AWAITING_CANCEL_TRANSACTION", to: GATEWAY, data: CANCEL_DATA, value: 5n }
					handedBack.push(input)
				},
			}),
		)
		let submits = 0
		;(internals.deps as unknown as Record<string, unknown>).tokenSender = {
			submitCalls: async () => {
				submits += 1
				if (submits === 1 && submission) throw new SubmittedUnconfirmedError("no receipt in time", "EVM-8453", submission)
				return { txHash: "0xnewcancel", sponsored: true }
			},
			userOpReceipt: async () => {
				if (client.userOpReceiptError) throw client.userOpReceiptError
				return client.userOpReceipt ?? null
			},
		}
		;(internals.deps.clientManager as unknown as Record<string, unknown>).getPublicClient = () => ({
			getBlockNumber: async () => 1_000n,
			getBlock: async () => ({ timestamp: BigInt(client.blockTimestamp ?? 0) }),
			getTransactionReceipt: async () => {
				if (!client.receipt) throw new TransactionReceiptNotFoundError({ hash: PLACE_TX })
				return client.receipt
			},
			getTransaction: async ({ hash }: { hash: string }) => {
				if (client.pendingTx?.hash !== hash) throw new TransactionNotFoundError({ hash: hash as HexString })
				return { ...client.pendingTx, blockNumber: null }
			},
			getTransactionCount: async () => client.accountNonce ?? 0,
			readContract: async ({ functionName }: { functionName: string }) => {
				if (functionName === "feeToken") return FEE_TOKEN
				if (functionName === "balanceOf") return 1_000_000n
				if (functionName === "getNonce") return client.entryPointNonce ?? 0n
				return {}
			},
		})
		;(internals.deps.configService as unknown as Record<string, unknown>).getEntryPointAddress = () => GATEWAY
		await topUp.reclaim()
		// The first attempt stores the unconfirmed cancel and fails; the retry is a minute out.
		await vi.advanceTimersByTimeAsync(0)
		const afterFirst = (await data.state.get()).bridgeTopUp!
		await vi.advanceTimersByTimeAsync(60_000)
		return { afterFirst, record: (await data.state.get()).bridgeTopUp!, submits: () => submits, handedBack }
	}

	const cleanup = () => {
		vi.clearAllTimers()
		vi.useRealTimers()
	}
	const unconfirmedTx = { kind: "tx", txHash: "0xcanceltx", nonce: 4, submittedAt: 1 } as const
	const unconfirmedOp = {
		kind: "userOp",
		userOpHash: USER_OP_HASH,
		nonce: 6n,
		nonceKey: 0n,
		expires: true,
		submittedAt: 1,
	} as const
	const withFetch = async (run: () => Promise<void>) => {
		const fetchSpy = vi
			.spyOn(globalThis, "fetch")
			.mockImplementation(
				async () => new Response(JSON.stringify({ data: { iOrderV3s: { nodes: [{ statusMetadata: { nodes: [] } }] } } })),
			)
		try {
			await run()
		} finally {
			cleanup()
			fetchSpy.mockRestore()
		}
	}

	it("stores a cancel sent without a receipt as unconfirmed, with its nonce, and hands it back once it succeeded", () =>
		withFetch(async () => {
			const { afterFirst, record, submits, handedBack } = await retriedCancel(unconfirmedTx, {
				receipt: { status: "success" },
			})

			expect(afterFirst.reclaimSubmission).toEqual({ kind: "tx", txHash: "0xcanceltx", nonce: 4, submittedAt: 1 })
			// Not a confirmed cancel yet: the hash is not offered to the SDK until a check shows it landed.
			expect(afterFirst.reclaimTxHash).toBeUndefined()
			expect(submits()).toBe(1)
			expect(handedBack).toEqual(["0xcanceltx"])
			expect(record.reclaimTxHash).toBe("0xcanceltx")
			expect(record.reclaimSubmission).toBeUndefined()
		}))

	it("sends a new cancel when the unconfirmed one reverted", () =>
		withFetch(async () => {
			const { record, submits, handedBack } = await retriedCancel(unconfirmedTx, { receipt: { status: "reverted" } })

			expect(submits()).toBe(2)
			expect(handedBack).toEqual(["0xnewcancel"])
			expect(record.reclaimTxHash).toBe("0xnewcancel")
			expect(record.reclaimSubmission).toBeUndefined()
		}))

	it("sends a new cancel when the unconfirmed one was dropped and its nonce reused", () =>
		withFetch(async () => {
			// No receipt, and the wallet's nonce is past the cancel's: another tx took it.
			const { submits, handedBack } = await retriedCancel(unconfirmedTx, { accountNonce: 5 })

			expect(submits()).toBe(2)
			expect(handedBack).toEqual(["0xnewcancel"])
		}))

	it("waits, sending nothing, while the unconfirmed cancel could still be mined", () =>
		withFetch(async () => {
			const { record, submits, handedBack } = await retriedCancel(unconfirmedTx, { accountNonce: 4 })

			expect(submits()).toBe(1)
			expect(handedBack).toEqual([])
			expect(record.stage).toBe("reclaiming")
			expect(record.error).toContain("is not mined yet")
			expect(record.reclaimSubmission).toMatchObject({ txHash: "0xcanceltx" })
		}))

	it("does not resend a UserOp cancel that timed out: it is checked by hash, nonce and key", () =>
		withFetch(async () => {
			const waiting = await retriedCancel(unconfirmedOp, { entryPointNonce: 6n })
			expect(waiting.afterFirst.reclaimSubmission).toEqual({
				kind: "userOp",
				userOpHash: USER_OP_HASH,
				nonce: "6",
				nonceKey: "0",
				expires: true,
				submittedAt: 1,
			})
			expect(waiting.submits()).toBe(1)
			expect(waiting.record.error).toContain("can still land")
			cleanup()

			const landed = await retriedCancel(unconfirmedOp, { userOpReceipt: { txHash: "0xlanded", success: true } })
			expect(landed.submits()).toBe(1)
			expect(landed.handedBack).toEqual(["0xlanded"])
			cleanup()

			// The bundler has no receipt and the op's EntryPoint nonce was consumed: it is not ours.
			const dropped = await retriedCancel(unconfirmedOp, { entryPointNonce: 7n })
			expect(dropped.submits()).toBe(2)
			expect(dropped.handedBack).toEqual(["0xnewcancel"])
		}))

	it("waits, sending nothing, when the bundler cannot be asked about a cancel UserOp whose nonce moved", () =>
		withFetch(async () => {
			// The lookup failed, so it says nothing about the op: a moved nonce may be our own landed op.
			const unanswered = await retriedCancel(unconfirmedOp, {
				entryPointNonce: 7n,
				userOpReceiptError: new Error("bundler unreachable"),
			})
			expect(unanswered.submits()).toBe(1)
			expect(unanswered.handedBack).toEqual([])
			expect(unanswered.record.stage).toBe("reclaiming")
			expect(unanswered.record.error).toContain("Could not ask the bundler")
			expect(unanswered.record.reclaimSubmission).toMatchObject({ kind: "userOp", userOpHash: USER_OP_HASH })
		}))

	it("clears and resends when the bundler answers that it has no receipt and the nonce moved", () =>
		withFetch(async () => {
			const answered = await retriedCancel(unconfirmedOp, { entryPointNonce: 7n, userOpReceipt: null })
			expect(answered.submits()).toBe(2)
			expect(answered.handedBack).toEqual(["0xnewcancel"])
			expect(answered.record.reclaimSubmission).toBeUndefined()
		}))

	it("clears a Permit2 cancel UserOp whose permit expired on a quiet chain, and sends a new cancel", () =>
		withFetch(async () => {
			// No receipt and the EntryPoint nonce never moved, but the chain's clock at the evidence block is
			// past the op's deadline: it can never validate, so waiting on it would hold the escrow for good.
			const expired = await retriedCancel(unconfirmedOp, { entryPointNonce: 6n, blockTimestamp: 10_000_000_000 })
			expect(expired.afterFirst.reclaimSubmission).toMatchObject({ kind: "userOp", expires: true })
			expect(expired.submits()).toBe(2)
			expect(expired.handedBack).toEqual(["0xnewcancel"])
			expect(expired.record.reclaimSubmission).toBeUndefined()
			cleanup()

			// A permit still inside its window can land, so the attempt waits.
			const live = await retriedCancel(unconfirmedOp, { entryPointNonce: 6n, blockTimestamp: 2 })
			expect(live.submits()).toBe(1)
			expect(live.record.error).toContain("can still land")
			cleanup()

			// A PERMIT bootstrap op has no deadline, however late the chain's clock is.
			const bootstrap = await retriedCancel(
				{ ...unconfirmedOp, expires: false },
				{ entryPointNonce: 6n, blockTimestamp: 10_000_000_000 },
			)
			expect(bootstrap.submits()).toBe(1)
			expect(bootstrap.handedBack).toEqual([])
			expect(bootstrap.record.error).toContain("can still land")
		}))

	it("hands back a legacy reclaimTxHash only when its receipt succeeded", () =>
		withFetch(async () => {
			const legacy = { reclaimTxHash: "0xlegacy" } as const
			const landed = await retriedCancel(undefined, { receipt: { status: "success" } }, legacy)
			expect(landed.submits()).toBe(0)
			expect(landed.handedBack).toEqual(["0xlegacy"])
			expect(landed.record.reclaimTxHash).toBe("0xlegacy")
		}))

	it("clears a legacy reclaimTxHash that reverted and sends a new cancel", () =>
		withFetch(async () => {
			const { record, submits, handedBack } = await retriedCancel(
				undefined,
				{ receipt: { status: "reverted" } },
				{ reclaimTxHash: "0xlegacy" },
			)
			expect(submits()).toBe(1)
			expect(handedBack).toEqual(["0xnewcancel"])
			expect(record.reclaimTxHash).toBe("0xnewcancel")
		}))

	it("converts a legacy reclaimTxHash the node still lists to a submission, and applies the nonce rule", () =>
		withFetch(async () => {
			const pendingTx = { hash: "0xlegacy", nonce: 4 }
			const waiting = await retriedCancel(undefined, { pendingTx, accountNonce: 4 }, { reclaimTxHash: "0xlegacy" })
			expect(waiting.submits()).toBe(0)
			expect(waiting.handedBack).toEqual([])
			expect(waiting.record.reclaimTxHash).toBeUndefined()
			expect(waiting.record.reclaimSubmission).toMatchObject({ kind: "tx", txHash: "0xlegacy", nonce: 4 })
			expect(waiting.record.error).toContain("is not mined yet")
			cleanup()

			// Its nonce taken by another tx: it can never land, so a new cancel goes out.
			const dropped = await retriedCancel(undefined, { pendingTx, accountNonce: 5 }, { reclaimTxHash: "0xlegacy" })
			expect(dropped.submits()).toBe(1)
			expect(dropped.handedBack).toEqual(["0xnewcancel"])
			expect(dropped.record.reclaimSubmission).toBeUndefined()
		}))

	it("clears a legacy reclaimTxHash the node does not know and sends a new cancel", () =>
		withFetch(async () => {
			const { record, submits, handedBack } = await retriedCancel(undefined, {}, { reclaimTxHash: "0xlegacy" })
			expect(submits()).toBe(1)
			expect(handedBack).toEqual(["0xnewcancel"])
			expect(record.reclaimTxHash).toBe("0xnewcancel")
		}))

	it("keeps a bundler-confirmed placement pending when its receipt cannot be read", async () => {
		vi.useFakeTimers()
		try {
			const { topUp, data } = topUpWith(userOpPlacement())
			const internals = topUp as unknown as TopUpInternals
			internals.deps.tokenSender = { userOpReceipt: async () => ({ txHash: PLACE_TX, success: true }) } as never
			installPayingClient(internals, { receiptError: new Error("receipt not readable"), logs: [] })

			await topUp.current()
			await vi.advanceTimersByTimeAsync(60_000)
			await vi.advanceTimersByTimeAsync(60_000)

			const saved = (await data.state.get()).bridgeTopUp!
			// The known tx decides by the tx rules: it is not settled by the op's permit window.
			expect(saved.stage).toBe("placing")
			expect(saved.placeTxHash).toBe(PLACE_TX)
			expect(saved.error).not.toContain("Nothing was escrowed")
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("does not treat a paying chain behind the placement range as not found", async () => {
		vi.useFakeTimers()
		try {
			const { topUp, data } = topUpWith(userOpPlacement({ placeFromBlock: "500" }))
			const internals = topUp as unknown as TopUpInternals
			internals.deps.tokenSender = { userOpReceipt: async () => null } as never
			installPayingClient(internals, { latestBlock: 100n, logs: [] })

			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)

			const saved = (await data.state.get()).bridgeTopUp!
			expect(saved.stage).toBe("placing")
			expect(saved.error).toContain("behind the placement's range")
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("reads the EntryPoint nonce a reorg margin below the height the scan covered", async () => {
		vi.useFakeTimers()
		try {
			const { topUp, data } = topUpWith(userOpPlacement({ placeUserOpExpires: false }))
			const internals = topUp as unknown as TopUpInternals
			internals.deps.tokenSender = { userOpReceipt: async () => null } as never
			// The nonce has moved at the tip, but not as of the reorg margin below the height the scan reached.
			installPayingClient(internals, { logs: [], nonceAt: (block) => (block === 36n ? 3n : 4n) })

			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)

			expect((await data.state.get()).bridgeTopUp?.stage).toBe("placing")
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("continues a placement scan from the stored cursor", async () => {
		vi.useFakeTimers()
		try {
			const { topUp, data } = topUpWith(userOpPlacement({ placeUserOpExpires: false, placeFromBlock: "300" }))
			const internals = topUp as unknown as TopUpInternals
			internals.deps.tokenSender = { userOpReceipt: async () => null } as never
			const ranges: bigint[] = []
			installPayingClient(internals, {
				logs: [],
				latestBlock: 400n,
				nonce: 3n,
				onGetLogs: ({ fromBlock }) => ranges.push(fromBlock),
			})

			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)
			// The block recorded before submission came from one backend; the scan starts a margin below it.
			expect(ranges).toEqual([236n])
			// A complete scan leaves the cursor a reorg margin below the tip.
			expect((await data.state.get()).bridgeTopUp?.placeScannedTo).toBe("336")

			await vi.advanceTimersByTimeAsync(60_000)
			// The next attempt starts at the cursor, not at the placement's first block.
			expect(ranges[ranges.length - 1]).toBe(336n)
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("picks our placement by user when no session address is stored", async () => {
		const placeTxHash = `0x${"66".repeat(32)}` as HexString
		const { topUp, data } = topUpWith(stored("placing", { placeTxHash, placeSubmittedAt: Date.now() }))
		const internals = topUp as unknown as TopUpInternals
		// Another user's placement comes first; ours is identified by the order's user.
		installPayingClient(internals, {
			receipt: {
				logs: [orderPlacedLog({ user: bytes20ToBytes32(`0x${"22".repeat(20)}`), nonce: 9n }), orderPlacedLog()],
			},
		})

		await topUp.current()

		await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("awaiting_bids"))
		expect(deserializeOrder((await data.state.get()).bridgeTopUp!.order).nonce).toBe(7n)
	})

	it("fails a native placement the wallet's nonce has moved past", async () => {
		const placeTxHash = `0x${"66".repeat(32)}` as HexString
		const { topUp, data } = topUpWith(
			stored("placing", {
				placeTxHash,
				placeSession: SESSION_ADDRESS,
				placeFromBlock: "10",
				placeSubmittedAt: Date.now() - 2 * 60 * 60_000,
			}),
		)
		const internals = topUp as unknown as TopUpInternals
		installPayingClient(internals, {
			receiptError: new Error("receipt not readable"),
			logs: [],
			// The node still has the pending tx, but the wallet has sent another since: it can never be mined.
			getTransaction: async () => ({ hash: placeTxHash, nonce: 1, blockNumber: null }),
			accountNonce: 2,
		})

		await topUp.current()

		await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("failed"))
		expect((await data.state.get()).bridgeTopUp?.error).toContain("nonce has moved past it")
		expect((await data.state.get()).bridgeTopUp?.error).toContain("Nothing was escrowed")
	})

	it("does not settle a bundler-sourced tx the node has forgotten", async () => {
		vi.useFakeTimers()
		try {
			const placeTxHash = `0x${"66".repeat(32)}` as HexString
			const { topUp, data } = topUpWith(
				userOpPlacement({
					placeTxHash,
					// Past the dropped-tx window, inside the Permit2 permit window.
					placeSubmittedAt: Date.now() - 40 * 60_000,
				}),
			)
			const internals = topUp as unknown as TopUpInternals
			installPayingClient(internals, {
				receiptError: new Error("receipt not readable"),
				logs: [],
				getTransaction: async () => {
					throw new TransactionNotFoundError({ hash: placeTxHash })
				},
				nonce: 3n,
			})

			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)

			const saved = (await data.state.get()).bridgeTopUp!
			// The op could be re-bundled, so the UserOp rules decide, not "dropped".
			expect(saved.stage).toBe("placing")
			expect(saved.error).not.toContain("Nothing was escrowed")
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("recovers a re-bundled placement the scan finds under a new tx hash", async () => {
		const placeTxHash = `0x${"66".repeat(32)}` as HexString
		const { topUp, data } = topUpWith(userOpPlacement({ placeTxHash, placeSubmittedAt: Date.now() - 40 * 60_000 }))
		const internals = topUp as unknown as TopUpInternals
		internals.gateways.set("EVM-8453", Promise.resolve(placementGateway()))
		// The old tx's receipt is unreadable; the scan finds the placement under a new one.
		;(internals.deps.clientManager as unknown as Record<string, unknown>).getPublicClient = () => ({
			...payingClient({ logs: [scannedPlacement()], nonce: 3n }),
			waitForTransactionReceipt: async ({ hash }: { hash: string }) => {
				if (hash.toLowerCase() === placeTxHash.toLowerCase()) throw new Error("receipt not readable")
				return { logs: [orderPlacedLog()] }
			},
		})

		await topUp.current()

		await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("awaiting_bids"))
		expect((await data.state.get()).bridgeTopUp?.placeTxHash).toBe(SCAN_TX)
	})

	it("retries a native placement the node reports as mined", async () => {
		vi.useFakeTimers()
		try {
			const placeTxHash = `0x${"66".repeat(32)}` as HexString
			const { topUp, data } = topUpWith(
				stored("placing", {
					placeTxHash,
					placeSession: SESSION_ADDRESS,
					placeFromBlock: "10",
					placeSubmittedAt: Date.now() - 2 * 60 * 60_000,
				}),
			)
			const internals = topUp as unknown as TopUpInternals
			installPayingClient(internals, {
				receiptError: new Error("receipt not readable"),
				logs: [],
				// Mined, so its receipt will be readable; never settle it.
				getTransaction: async () => ({ hash: placeTxHash, nonce: 1, blockNumber: 42n }),
				accountNonce: 5,
			})

			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)

			const saved = (await data.state.get()).bridgeTopUp!
			expect(saved.stage).toBe("placing")
			expect(saved.error).not.toContain("Nothing was escrowed")
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("fails a native placement the wallet's nonce has moved past below the scanned height", async () => {
		const placeTxHash = `0x${"66".repeat(32)}` as HexString
		const { topUp, data } = topUpWith(
			stored("placing", {
				placeTxHash,
				placeSession: SESSION_ADDRESS,
				placeFromBlock: "10",
				placeSubmittedAt: Date.now() - 2 * 60 * 60_000,
			}),
		)
		const internals = topUp as unknown as TopUpInternals
		installPayingClient(internals, {
			receiptError: new Error("receipt not readable"),
			logs: [],
			getTransaction: async () => ({ hash: placeTxHash, nonce: 1, blockNumber: null }),
			accountNonceAt: (block) => (block === 36n ? 2 : 0),
		})

		await topUp.current()

		await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("failed"))
		expect((await data.state.get()).bridgeTopUp?.error).toContain("nonce has moved past it")
	})

	it("reads the wallet's nonce below the scanned height, not at the tip", async () => {
		vi.useFakeTimers()
		try {
			const placeTxHash = `0x${"66".repeat(32)}` as HexString
			const { topUp, data } = topUpWith(
				stored("placing", {
					placeTxHash,
					placeSession: SESSION_ADDRESS,
					placeFromBlock: "10",
					placeSubmittedAt: Date.now() - 2 * 60 * 60_000,
				}),
			)
			const internals = topUp as unknown as TopUpInternals
			installPayingClient(internals, {
				receiptError: new Error("receipt not readable"),
				logs: [],
				getTransaction: async () => ({ hash: placeTxHash, nonce: 1, blockNumber: null }),
				// Past the tx only at the tip, not as of a reorg margin below the height the scan reached.
				accountNonceAt: (block) => (block === 36n ? 1 : 2),
			})

			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)

			expect((await data.state.get()).bridgeTopUp?.stage).toBe("placing")
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("ignores an OrderPlaced for our session that another contract in the bundle emitted", async () => {
		const placeTxHash = `0x${"66".repeat(32)}` as HexString
		// Same shape and same session, but not the gateway: any op in the bundle can emit it.
		const forged = { ...orderPlacedLog({ nonce: 99n }), address: "0x00000000000000000000000000000000000000ff" }

		const onlyForged = topUpWith(stored("placing", { placeTxHash, placeSession: SESSION_ADDRESS, placeSubmittedAt: Date.now() }))
		installPayingClient(onlyForged.topUp as unknown as TopUpInternals, { receipt: { logs: [forged] } })
		await onlyForged.topUp.current()
		await vi.waitFor(async () => expect((await onlyForged.data.state.get()).bridgeTopUp?.stage).toBe("failed"))
		expect((await onlyForged.data.state.get()).bridgeTopUp?.error).toContain("was mined but placed no order")

		const bothLogs = topUpWith(stored("placing", { placeTxHash, placeSession: SESSION_ADDRESS, placeSubmittedAt: Date.now() }))
		installPayingClient(bothLogs.topUp as unknown as TopUpInternals, { receipt: { logs: [forged, orderPlacedLog()] } })
		await bothLogs.topUp.current()
		await vi.waitFor(async () => expect((await bothLogs.data.state.get()).bridgeTopUp?.stage).toBe("awaiting_bids"))
		expect(deserializeOrder((await bothLogs.data.state.get()).bridgeTopUp!.order).nonce).toBe(7n)
	})

	it("records when a placement was submitted and which op it was even when its receipt arrives", async () => {
		vi.useFakeTimers()
		try {
			const record = stored("placing")
			const { topUp, data } = topUpWith(record)
			const internals = topUp as unknown as TopUpInternals
			internals.record = record
			internals.gateways.set("EVM-8453", Promise.resolve(placementGateway()))
			const submittedAt = Date.now()
			internals.deps.tokenSender = {
				submitCalls: async () => ({
					txHash: PLACE_TX,
					sponsored: true,
					submission: { kind: "userOp", userOpHash: USER_OP_HASH, nonce: 5n, nonceKey: 0n, expires: true, submittedAt },
				}),
				userOpReceipt: async () => null,
			} as never
			// The bundler confirmed the op, but this node cannot read the bundle's receipt.
			installPayingClient(internals, { receiptError: new Error("receipt not readable"), logs: [], nonce: 6n })

			await internals.place(deserializeOrder(record.order), FEE_TOKEN)

			let saved = (await data.state.get()).bridgeTopUp!
			expect(saved).toMatchObject({
				placeTxHash: PLACE_TX,
				placeUserOpHash: USER_OP_HASH,
				placeUserOpNonce: "5",
				placeUserOpNonceKey: "0",
				placeUserOpExpires: true,
				placeSubmittedAt: submittedAt,
			})

			// The bundle's hash is later forgotten by the node: the op's own nonce settles it, not "dropped".
			;(internals.deps.clientManager as unknown as Record<string, unknown>).getPublicClient = () =>
				payingClient({
					receiptError: new Error("receipt not readable"),
					logs: [],
					nonce: 6n,
					getTransaction: async () => {
						throw new TransactionNotFoundError({ hash: PLACE_TX })
					},
				})
			await vi.advanceTimersByTimeAsync(60_000)
			saved = (await data.state.get()).bridgeTopUp!
			expect(saved.stage).toBe("failed")
			expect(saved.error).toContain("EntryPoint nonce has moved on")
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("records a native placement's submission time and nonce when it is confirmed", async () => {
		const record = stored("placing")
		const { topUp, data } = topUpWith(record)
		const internals = topUp as unknown as TopUpInternals
		internals.record = record
		internals.gateways.set("EVM-8453", Promise.resolve(placementGateway()))
		internals.deps.tokenSender = {
			submitCalls: async () => ({
				txHash: PLACE_TX,
				sponsored: false,
				submission: { kind: "tx", txHash: PLACE_TX, nonce: 8, submittedAt: 1_234 },
			}),
		} as never
		installPayingClient(internals, { receipt: { logs: [orderPlacedLog()] } })

		await internals.place(deserializeOrder(record.order), FEE_TOKEN)

		expect((await data.state.get()).bridgeTopUp).toMatchObject({
			stage: "awaiting_bids",
			placeTxHash: PLACE_TX,
			placeTxNonce: 8,
			placeSubmittedAt: 1_234,
		})
	})

	it("dates a legacy record by when it began placing, never by its last save", async () => {
		const hoursAgo = (hours: number) => Date.now() - hours * 60 * 60_000
		// Every save rewrites `updatedAt`, so it is always recent; the permit window must still age.
		for (const legacy of [
			{ stageTimes: { placing: hoursAgo(2) }, createdAt: Date.now() },
			{ stageTimes: undefined, createdAt: hoursAgo(2) },
		]) {
			const { topUp, data } = topUpWith(
				userOpPlacement({ ...legacy, placeSubmittedAt: undefined, updatedAt: Date.now() }),
			)
			const internals = topUp as unknown as TopUpInternals
			internals.deps.tokenSender = { userOpReceipt: async () => null } as never
			installPayingClient(internals, { logs: [], nonce: 3n })

			await topUp.current()

			await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("failed"))
			expect((await data.state.get()).bridgeTopUp?.error).toContain("permit expired")
		}
	})

	it("judges a permit's expiry by the chain's clock at the block its nonce is read at, not the host's", async () => {
		vi.useFakeTimers()
		try {
			const { topUp, data } = topUpWith(userOpPlacement())
			const internals = topUp as unknown as TopUpInternals
			internals.deps.tokenSender = { userOpReceipt: async () => null } as never
			const blocks: bigint[] = []
			// By this host's clock the op was submitted two hours ago; the chain is only 20 minutes past it.
			installPayingClient(internals, {
				logs: [],
				nonce: 3n,
				blockTimestamp: (block) => {
					blocks.push(block)
					return Math.floor((Date.now() - 100 * 60_000) / 1000)
				},
			})

			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)

			expect((await data.state.get()).bridgeTopUp?.stage).toBe("placing")
			// The scan reached block 100; the clock is read where the nonce is, a reorg margin below.
			expect(blocks).toEqual([36n])
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("saves the scan cursor after each chunk, so a scan that fails late resumes where it got to", async () => {
		vi.useFakeTimers()
		try {
			const { topUp, data } = topUpWith(userOpPlacement({ placeUserOpExpires: false }))
			const internals = topUp as unknown as TopUpInternals
			internals.deps.tokenSender = { userOpReceipt: async () => null } as never
			installPayingClient(internals, {
				logs: [],
				latestBlock: 5_000n,
				// The first 2000-block chunk reads; every later one is refused.
				onGetLogs: ({ fromBlock }) => {
					if (fromBlock >= 2_000n) throw new Error("range too wide")
				},
			})

			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)

			const saved = (await data.state.get()).bridgeTopUp!
			expect(saved.error).toContain("range too wide")
			expect(saved.placeScannedTo).toBe("2000")
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("does not let a recovery for a replaced record write into the new top-up", async () => {
		const placeTxHash = `0x${"66".repeat(32)}` as HexString
		const { topUp, data } = topUpWith(
			stored("failed", {
				placeTxHash,
				placeSession: SESSION_ADDRESS,
				sessionPrivateKey: SESSION_KEY,
				error: "receipt read failed",
			}),
		)
		const internals = topUp as unknown as TopUpInternals
		const isOrderFilled = vi.fn(async () => false)
		internals.gateways.set("EVM-8453", Promise.resolve(placementGateway({ isOrderFilled })))
		let release!: () => void
		const gate = new Promise<void>((resolve) => {
			release = resolve
		})
		let read = false
		;(internals.deps.clientManager as unknown as Record<string, unknown>).getPublicClient = () => ({
			...payingClient(),
			waitForTransactionReceipt: async () => {
				await gate
				read = true
				return { logs: [orderPlacedLog()] }
			},
		})

		// A failed record is not pending, so restoring it re-checks its placement in the background.
		await topUp.current()
		// Meanwhile the operator starts the next top-up, which takes the record's place.
		const next = stored("placing", { id: "next", updatedAt: 5 })
		internals.record = next
		await data.state.set({ bridgeTopUp: next })

		release()
		await vi.waitFor(() => expect(read).toBe(true))
		await new Promise((resolve) => setTimeout(resolve, 20))

		// The old recovery found its order and would have stored it; the new record is untouched,
		// in memory and on disk, and nothing started tracking the old order as if it were the new one.
		expect(internals.record).toEqual(stored("placing", { id: "next", updatedAt: 5 }))
		expect((await data.state.get()).bridgeTopUp).toEqual(stored("placing", { id: "next", updatedAt: 5 }))
		expect(isOrderFilled).not.toHaveBeenCalled()
	})

	it("drops every write a flow makes, and the error it fails with, once its record was replaced", async () => {
		const { topUp, data } = topUpWith(stored("failed", { id: "old", error: "old error" }))
		const internals = topUp as unknown as TopUpInternals & {
			drive: (flow: () => Promise<void>) => Promise<void>
			update: (patch: Record<string, unknown>) => Promise<void>
		}
		await topUp.current()
		let release!: () => void
		const gate = new Promise<void>((resolve) => {
			release = resolve
		})
		const flow = internals.drive(async () => {
			await gate
			await internals.update({ stage: "unfilled", error: "from the old flow" })
			throw new Error("the old flow failed")
		})
		const next = stored("placing", { id: "next", updatedAt: 5 })
		internals.record = next
		await data.state.set({ bridgeTopUp: next })

		release()
		await flow

		expect(internals.record).toEqual(stored("placing", { id: "next", updatedAt: 5 }))
		expect((await data.state.get()).bridgeTopUp).toEqual(stored("placing", { id: "next", updatedAt: 5 }))
		// A write outside any flow, such as the operator's own, still reaches the current record.
		await internals.update({ error: "operator" })
		expect(internals.record.error).toBe("operator")
	})
})

describe("BRIDGE top-up live order tracking", () => {
	const OPEN = stored("awaiting_bids", {
		commitment: `0x${"77".repeat(32)}`,
		sessionPrivateKey: `0x${"55".repeat(32)}`,
	})

	it("keeps retrying a live order whose destination read fails, and resumes it", async () => {
		vi.useFakeTimers()
		try {
			const { topUp, data } = topUpWith(OPEN)
			const internals = topUp as unknown as TopUpInternals
			const isOrderFilled = vi
				.fn()
				.mockRejectedValueOnce(new Error("Could not read the order on its destination chain"))
				.mockResolvedValue(false)
			internals.gateways.set(
				"EVM-8453",
				Promise.resolve({ isOrderFilled, async *resumeBest() {} }),
			)
			// The order's deadline is a BSC height far above this; only then is it settled.
			internals.bsc = () => ({ getBlockNumber: async () => 1n })

			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)

			let saved = (await data.state.get()).bridgeTopUp!
			expect(saved.stage).toBe("awaiting_bids")
			expect(saved.error).toContain("Tracking is retrying")

			await vi.advanceTimersByTimeAsync(60_000)
			saved = (await data.state.get()).bridgeTopUp!
			expect(saved.stage).toBe("awaiting_bids")
			expect(isOrderFilled).toHaveBeenCalledTimes(2)
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("does not settle a live order whose executor reports a transient failure", async () => {
		vi.useFakeTimers()
		try {
			const { topUp, data } = topUpWith(OPEN)
			const internals = topUp as unknown as TopUpInternals
			internals.gateways.set(
				"EVM-8453",
				Promise.resolve({
					isOrderFilled: async () => false,
					async *resumeBest() {
						yield { status: "FAILED", error: "Unexpected error: destination read failed" }
					},
				}),
			)
			internals.bsc = () => ({ getBlockNumber: async () => 1n })

			await topUp.current()
			await vi.advanceTimersByTimeAsync(0)

			const saved = (await data.state.get()).bridgeTopUp!
			expect(saved.stage).toBe("awaiting_bids")
			expect(saved.error).toContain("Unexpected error: destination read failed")
			expect(saved.error).toContain("Tracking is retrying")
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})

	it("settles a live order whose executor fails once its deadline has passed", async () => {
		const { topUp, data } = topUpWith(OPEN)
		const internals = topUp as unknown as TopUpInternals
		internals.gateways.set(
			"EVM-8453",
			Promise.resolve({
				isOrderFilled: async () => false,
				async *resumeBest() {
					yield { status: "FAILED", error: "Unexpected error: destination read failed" }
				},
			}),
		)
		internals.bsc = () => ({ getBlockNumber: async () => 999_999_999n })

		await topUp.current()

		await vi.waitFor(async () => expect((await data.state.get()).bridgeTopUp?.stage).toBe("unfilled"))
		expect((await data.state.get()).bridgeTopUp?.error).toContain("Unexpected error: destination read failed")
	})

	it("picks up an unfilled order the indexer shows as filled", async () => {
		const { topUp, data } = topUpWith(stored("unfilled", { commitment: `0x${"77".repeat(32)}` }))
		const internals = topUp as unknown as TopUpInternals
		const resumeDelivery = vi.fn(async () => {})
		internals.resumeDelivery = resumeDelivery
		internals.gateways.set("EVM-8453", Promise.resolve({ isOrderFilled: async () => false, isOrderRefunded: async () => false }))
		const fetchSpy = vi.spyOn(globalThis, "fetch").mockImplementation(
			async () =>
				new Response(
					JSON.stringify({
						data: {
							iOrderV3s: {
								nodes: [
									{
										statusMetadata: {
											nodes: [{ status: "FILLED", timestamp: "100", transactionHash: FILL_TX, filler: SOLVER }],
										},
									},
								],
							},
						},
					}),
				),
		)

		await topUp.sync()

		const saved = (await data.state.get()).bridgeTopUp!
		expect(saved.stage).toBe("delivering")
		expect(saved.fillTxHash).toBe(FILL_TX)
		expect(saved.filler).toBe(SOLVER)
		expect(saved.stageTimes?.delivering).toBeTypeOf("number")
		expect(resumeDelivery).toHaveBeenCalledTimes(1)
		fetchSpy.mockRestore()
	})

	it("picks up an unfilled order the gateway itself shows as filled", async () => {
		const { topUp, data } = topUpWith(stored("unfilled", { commitment: `0x${"77".repeat(32)}` }))
		const internals = topUp as unknown as TopUpInternals
		const resumeDelivery = vi.fn(async () => {})
		internals.resumeDelivery = resumeDelivery
		internals.gateways.set("EVM-8453", Promise.resolve({ isOrderFilled: async () => true, isOrderRefunded: async () => false }))
		const fetchSpy = vi
			.spyOn(globalThis, "fetch")
			.mockImplementation(
				async () => new Response(JSON.stringify({ data: { iOrderV3s: { nodes: [{ statusMetadata: { nodes: [] } }] } } })),
			)

		await topUp.sync()

		expect((await data.state.get()).bridgeTopUp?.stage).toBe("delivering")
		expect(resumeDelivery).toHaveBeenCalledTimes(1)
		fetchSpy.mockRestore()
	})
})

describe("BRIDGE top-up delivery request", () => {
	it("takes this order's send from a fill that also paid another account", async () => {
		const record = stored("delivering", { commitment: `0x${"77".repeat(32)}`, fillTxHash: FILL_TX })
		const { topUp } = topUpWith(record)
		const internals = topUp as unknown as TopUpInternals
		internals.record = record
		// Another recipient is paid first; the indexer also returns two requests.
		internals.bsc = () => ({
			getTransactionReceipt: async () => ({
				logs: [sentLog({ to: `0x${"cd".repeat(32)}`, amount: 1n, commitment: `0x${"11".repeat(32)}` }), sentLog()],
			}),
		})
		const fetchSpy = vi.spyOn(globalThis, "fetch")

		expect(await internals.deliveryRequest(FILL_TX)).toBe(DELIVERY_COMMITMENT)
		// The readable receipt decides; the indexer is never asked.
		expect(fetchSpy).not.toHaveBeenCalled()
		fetchSpy.mockRestore()
	})

	it("uses the order's output amount to pick its send when the account is paid twice", async () => {
		const record = stored("delivering", { commitment: `0x${"77".repeat(32)}`, fillTxHash: FILL_TX })
		const { topUp } = topUpWith(record)
		const internals = topUp as unknown as TopUpInternals
		internals.record = record
		internals.bsc = () => ({
			getTransactionReceipt: async () => ({ logs: [sentLog({ amount: 1n, commitment: `0x${"11".repeat(32)}` }), sentLog()] }),
		})

		expect(await internals.deliveryRequest(FILL_TX)).toBe(DELIVERY_COMMITMENT)
	})

	it("throws when a readable fill sent no BRIDGE to this filler", async () => {
		const record = stored("delivering", { commitment: `0x${"77".repeat(32)}`, fillTxHash: FILL_TX })
		const { topUp } = topUpWith(record)
		const internals = topUp as unknown as TopUpInternals
		internals.record = record
		internals.bsc = () => ({
			getTransactionReceipt: async () => ({ logs: [sentLog({ to: `0x${"cd".repeat(32)}` })] }),
		})

		await expect(internals.deliveryRequest(FILL_TX)).rejects.toThrow(
			`The fill ${FILL_TX} sent no BRIDGE to this filler's Hyperbridge account`,
		)
	})
})
