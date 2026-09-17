import { BidManager } from "@/protocols/intents/BidManager"
import { BidExecutionPendingError, BidImpl } from "@/protocols/intents/Bid"
import { CryptoUtils } from "@/protocols/intents/CryptoUtils"
import { OrderExecutor } from "@/protocols/intents/OrderExecutor"
import { encodeFillOrderAtRate } from "@/protocols/intents/fillOrderCodec"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import type { Bid, FillerBid, HexString, Order, PackedUserOperation, SelectBidResult } from "@/types"
import { encodeAbiParameters, encodeEventTopics } from "viem"
import { describe, expect, it, vi } from "vitest"

const SOLVER_ONE = "0x1111111111111111111111111111111111111111" as HexString
const SOLVER_TWO = "0x2222222222222222222222222222222222222222" as HexString
const TOKEN = "0x3333333333333333333333333333333333333333" as HexString
const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString
const COMMITMENT = `0x${"ab".repeat(32)}` as HexString
const SESSION = "0x5555555555555555555555555555555555555555" as HexString

const USER_OPERATION_EVENT_ABI = [
	{
		type: "event",
		name: "UserOperationEvent",
		inputs: [
			{ name: "userOpHash", type: "bytes32", indexed: true },
			{ name: "sender", type: "address", indexed: true },
			{ name: "paymaster", type: "address", indexed: true },
			{ name: "nonce", type: "uint256", indexed: false },
			{ name: "success", type: "bool", indexed: false },
			{ name: "actualGasCost", type: "uint256", indexed: false },
			{ name: "actualGasUsed", type: "uint256", indexed: false },
		],
	},
] as const

function makeUserOp(sender = SOLVER_ONE): PackedUserOperation {
	return {
		sender,
		nonce: 1n,
		initCode: "0x",
		callData: "0x1234",
		accountGasLimits: `0x${"00".repeat(32)}`,
		preVerificationGas: 50_000n,
		gasFees: `0x${"00".repeat(32)}`,
		paymasterAndData: "0x",
		signature: "0x12",
	}
}

function makeOrder(): Order {
	return {
		id: COMMITMENT,
		user: SOLVER_ONE,
		source: "EVM-1",
		destination: "EVM-8453",
		deadline: 100n,
		nonce: 0n,
		fees: 0n,
		session: SESSION,
		predispatch: { assets: [], call: "0x" },
		inputs: [{ token: TOKEN, amount: 100n }],
		output: {
			beneficiary: SOLVER_ONE,
			assets: [{ token: TOKEN, amount: 100n }],
			call: "0x",
		},
	}
}

function makeResult(userOp: PackedUserOperation, solverAddress: HexString): SelectBidResult {
	return {
		userOp,
		userOpHash: `0x${"ef".repeat(32)}`,
		solverAddress,
		commitment: COMMITMENT,
	}
}

function userOperationEventLog(address: HexString, success: boolean) {
	const userOp = makeUserOp(SOLVER_ONE)
	const userOpHash = CryptoUtils.computeUserOpHash(userOp, ENTRY_POINT, 8453n)
	return {
		address,
		topics: encodeEventTopics({
			abi: USER_OPERATION_EVENT_ABI,
			eventName: "UserOperationEvent",
			args: { userOpHash, sender: userOp.sender, paymaster: "0x0000000000000000000000000000000000000000" },
		}),
		data: encodeAbiParameters(
			[{ type: "uint256" }, { type: "bool" }, { type: "uint256" }, { type: "uint256" }],
			[userOp.nonce, success, 1n, 1n],
		),
	}
}

function orderFilledLog(address: HexString) {
	return {
		address,
		topics: encodeEventTopics({
			abi: IntentGatewayV2ABI,
			eventName: "OrderFilled",
			args: { commitment: COMMITMENT },
		}),
		data: encodeAbiParameters(
			[
				{ type: "address" },
				{
					type: "tuple[]",
					components: [
						{ name: "token", type: "bytes32" },
						{ name: "amount", type: "uint256" },
					],
				},
				{
					type: "tuple[]",
					components: [
						{ name: "token", type: "bytes32" },
						{ name: "amount", type: "uint256" },
					],
				},
			],
			[SOLVER_ONE, [], []],
		),
	}
}

function makeBid(params: { solverAddress: HexString; amount: bigint; take?: bigint; execute: Bid["execute"] }): Bid {
	return {
		solverAddress: params.solverAddress,
		outputs: [{ token: TOKEN, amount: params.amount }],
		inputs: params.take === undefined ? [] : [{ token: TOKEN, amount: params.take }],
		relayerFee: 0n,
		nativeDispatchFee: 0n,
		userOp: makeUserOp(params.solverAddress),
		simulate: vi.fn(async () => undefined),
		execute: params.execute,
		outputUsdValue: vi.fn(async () => null),
	}
}

function concreteBidWithReceipt(params: {
	userOpReceipt: { success?: boolean; receipt: { transactionHash: HexString } }
	chainReceipt: { status: "success" | "reverted"; logs: readonly unknown[] }
}): { ctx: any; bid: BidImpl } {
	const op = makeUserOp(SOLVER_ONE)
	const sendBundler = vi
		.fn()
		.mockResolvedValueOnce(`0x${"44".repeat(32)}`)
		.mockResolvedValueOnce(params.userOpReceipt)
	const ctx = {
		bundlerUrl: "http://bundler.test",
		intentsCoprocessor: {},
		dest: {
			config: { stateMachineId: "EVM-8453" },
			configService: {
				getIntentGatewayAddress: () => SOLVER_TWO,
				getEntryPointV08Address: () => ENTRY_POINT,
			},
			client: {
				chain: { id: 8453 },
				waitForTransactionReceipt: vi.fn(async () => params.chainReceipt),
			},
		},
		sessionKeyStorage: {
			getSessionKeyByAddress: vi.fn(async () => ({ privateKey: `0x${"01".repeat(32)}` })),
		},
	}
	const bid = new BidImpl({
		ctx: ctx as never,
		crypto: { sendBundler } as never,
		order: makeOrder(),
		fillerBid: { filler: "solver", userOp: op, deposit: 0n },
		fillOptions: {
			relayerFee: 0n,
			nativeDispatchFee: 0n,
			validUntil: 0n,
			outputs: [{ token: TOKEN, amount: 120n }],
		},
		priceOutputs: vi.fn(async () => null),
	})
	return { ctx, bid }
}

describe("Order execution bid-selection integration", () => {
	it("advances to the next bid after a chain-confirmed failed UserOperation", async () => {
		const { ctx, bid: failed } = concreteBidWithReceipt({
			userOpReceipt: { success: false, receipt: { transactionHash: `0x${"55".repeat(32)}` } },
			chainReceipt: { status: "success", logs: [] },
		})
		vi.spyOn(failed, "simulate").mockResolvedValue()
		const secondResult = makeResult(makeUserOp(SOLVER_TWO), SOLVER_TWO)
		const secondExecute = vi.fn(async () => secondResult)
		const second = makeBid({ solverAddress: SOLVER_TWO, amount: 110n, execute: secondExecute })
		const manager = new BidManager(ctx as never, {} as never)

		await expect(manager.selectAndExecuteBest(makeOrder(), [failed, second])).resolves.toBe(secondResult)
		expect(secondExecute).toHaveBeenCalledTimes(1)
	})

	it("keeps a confirmed transaction with no failure flag or fill event uncertain", async () => {
		const { ctx, bid: uncertain } = concreteBidWithReceipt({
			userOpReceipt: { receipt: { transactionHash: `0x${"55".repeat(32)}` } },
			chainReceipt: { status: "success", logs: [] },
		})
		vi.spyOn(uncertain, "simulate").mockResolvedValue()
		const secondExecute = vi.fn(async () => makeResult(makeUserOp(SOLVER_TWO), SOLVER_TWO))
		const second = makeBid({ solverAddress: SOLVER_TWO, amount: 110n, execute: secondExecute })
		const manager = new BidManager(ctx as never, {} as never)

		await expect(manager.selectAndExecuteBest(makeOrder(), [uncertain, second])).rejects.toBeInstanceOf(
			BidExecutionPendingError,
		)
		expect(secondExecute).not.toHaveBeenCalled()
	})

	it("ignores a matching UserOperationEvent emitted outside the configured EntryPoint", async () => {
		const { bid } = concreteBidWithReceipt({
			userOpReceipt: { receipt: { transactionHash: `0x${"55".repeat(32)}` } },
			chainReceipt: {
				status: "success",
				logs: [
					userOperationEventLog(SOLVER_ONE, false),
					userOperationEventLog(ENTRY_POINT, true),
					orderFilledLog(SOLVER_TWO),
				],
			},
		})

		await expect(bid.execute()).resolves.toMatchObject({ fillStatus: "full" })
	})

	it("does not accept a fill event emitted outside the configured gateway", async () => {
		const { bid } = concreteBidWithReceipt({
			userOpReceipt: { success: true, receipt: { transactionHash: `0x${"55".repeat(32)}` } },
			chainReceipt: { status: "success", logs: [orderFilledLog(SOLVER_ONE)] },
		})

		await expect(bid.execute()).rejects.toBeInstanceOf(BidExecutionPendingError)
	})

	it("classifies a confirmed reverted UserOp as a definitive candidate failure", async () => {
		const op = makeUserOp(SOLVER_ONE)
		const sendBundler = vi
			.fn()
			.mockResolvedValueOnce(`0x${"44".repeat(32)}`)
			.mockResolvedValueOnce({ receipt: { transactionHash: `0x${"55".repeat(32)}` } })
		const ctx = {
			bundlerUrl: "http://bundler.test",
			intentsCoprocessor: {},
			dest: {
				config: { stateMachineId: "EVM-8453" },
				configService: {
					getIntentGatewayAddress: () => SOLVER_TWO,
					getEntryPointV08Address: () => ENTRY_POINT,
				},
				client: {
					chain: { id: 8453 },
					waitForTransactionReceipt: vi.fn(async () => ({ status: "reverted", logs: [] })),
				},
			},
			sessionKeyStorage: {
				getSessionKeyByAddress: vi.fn(async () => ({ privateKey: `0x${"01".repeat(32)}` })),
			},
		} as never
		const concrete = new BidImpl({
			ctx,
			crypto: { sendBundler } as never,
			order: makeOrder(),
			fillerBid: { filler: "solver", userOp: op, deposit: 0n },
			fillOptions: {
				relayerFee: 0n,
				nativeDispatchFee: 0n,
				validUntil: 0n,
				outputs: [{ token: TOKEN, amount: 100n }],
			},
			priceOutputs: vi.fn(async () => null),
		})

		const execution = concrete.execute()
		await expect(execution).rejects.toThrow("reverted")
		await expect(execution).rejects.not.toBeInstanceOf(BidExecutionPendingError)
	})

	it("does not advance to another bid when the bundler send times out after a durable attempt record", async () => {
		const op = makeUserOp(SOLVER_ONE)
		const ctx = {
			bundlerUrl: "http://bundler.test",
			intentsCoprocessor: {},
			dest: {
				config: { stateMachineId: "EVM-8453" },
				configService: {
					getIntentGatewayAddress: () => SOLVER_TWO,
					getEntryPointV08Address: () => ENTRY_POINT,
				},
				client: { chain: { id: 8453 } },
			},
			sessionKeyStorage: {
				getSessionKeyByAddress: vi.fn(async () => ({ privateKey: `0x${"01".repeat(32)}` })),
			},
		} as never
		const concrete = new BidImpl({
			ctx,
			crypto: { sendBundler: vi.fn(async () => Promise.reject(new Error("transport timeout"))) } as never,
			order: makeOrder(),
			fillerBid: { filler: "solver", userOp: op, deposit: 0n },
			fillOptions: {
				relayerFee: 0n,
				nativeDispatchFee: 0n,
				validUntil: 0n,
				outputs: [{ token: TOKEN, amount: 120n }],
			},
			priceOutputs: vi.fn(async () => null),
		})
		vi.spyOn(concrete, "simulate").mockResolvedValue()
		const secondExecute = vi.fn(async () => makeResult(makeUserOp(SOLVER_TWO), SOLVER_TWO))
		const second = makeBid({ solverAddress: SOLVER_TWO, amount: 110n, execute: secondExecute })
		const onSubmitted = vi.fn(async () => undefined)
		const manager = new BidManager(ctx, {} as never)

		await expect(manager.selectAndExecuteBest(makeOrder(), [concrete, second], onSubmitted)).rejects.toBeInstanceOf(
			BidExecutionPendingError,
		)
		expect(onSubmitted).toHaveBeenCalledTimes(1)
		expect(secondExecute).not.toHaveBeenCalled()
	})

	it("refuses to sign a rate bid unless the live gateway and delegated solver account support it", async () => {
		const readContract = vi.fn().mockResolvedValueOnce(true).mockResolvedValueOnce(false)
		const token32 = `0x${"00".repeat(12)}${TOKEN.slice(2)}` as HexString
		const rateOrder = {
			...makeOrder(),
			user: token32,
			source: "0x6131",
			destination: "0x6232",
			inputs: [{ token: token32, amount: 100n }],
			output: { ...makeOrder().output, beneficiary: token32, assets: [{ token: token32, amount: 100n }] },
		}
		const rateCall = encodeFillOrderAtRate(
			rateOrder,
			{ relayerFee: 0n, nativeDispatchFee: 0n, validUntil: 1n, outputs: [{ token: token32, amount: 55n }] },
			[{ token: token32, amount: 50n }],
		)
		const manager = new BidManager(
			{
				dest: {
					client: { chain: { id: 8453 }, readContract },
					config: { stateMachineId: "EVM-8453" },
					configService: { getIntentGatewayAddress: () => SOLVER_TWO },
				},
			} as never,
			{ decodeERC7821Execute: () => [{ target: SOLVER_TWO, value: 0n, data: rateCall }] } as never,
		)

		await expect(
			manager.prepareSubmitBid({
				order: makeOrder(),
				fillOptions: {
					relayerFee: 0n,
					nativeDispatchFee: 0n,
					validUntil: 1n,
					outputs: [{ token: TOKEN, amount: 55n }],
				},
				solverAccount: SOLVER_ONE,
				solverSigner: { signTypedData: vi.fn() },
				nonce: 0n,
				entryPointAddress: ENTRY_POINT,
				callGasLimit: 1n,
				verificationGasLimit: 1n,
				preVerificationGas: 1n,
				maxFeePerGas: 1n,
				maxPriorityFeePerGas: 1n,
				callData: "0x",
			}),
		).rejects.toThrow("Rate fills are not supported")
		expect(readContract).toHaveBeenCalledTimes(2)
	})

	it("ranks a better-priced small rate bid before a worse-priced larger bid", async () => {
		const manager = new BidManager({} as never, {} as never)
		const betterSmall = makeBid({
			solverAddress: SOLVER_ONE,
			amount: 55n,
			take: 50n,
			execute: vi.fn(),
		})
		const worseLarge = makeBid({
			solverAddress: SOLVER_TWO,
			amount: 105n,
			take: 100n,
			execute: vi.fn(),
		})

		await expect(manager.sortBids(makeOrder(), [worseLarge, betterSmall])).resolves.toEqual([
			betterSmall,
			worseLarge,
		])
	})

	it("keeps input order when exact rate products tie", async () => {
		const manager = new BidManager({} as never, {} as never)
		const first = makeBid({ solverAddress: SOLVER_ONE, amount: 10n, take: 5n, execute: vi.fn() })
		const second = makeBid({ solverAddress: SOLVER_TWO, amount: 20n, take: 10n, execute: vi.fn() })

		await expect(manager.sortBids(makeOrder(), [first, second])).resolves.toEqual([first, second])
	})

	it("does not submit another bid when the first submission outcome is uncertain", async () => {
		const ctx = { bundlerUrl: "x", intentsCoprocessor: {} } as never
		const manager = new BidManager(ctx, {} as never)
		const first = makeBid({
			solverAddress: SOLVER_ONE,
			amount: 120n,
			execute: vi.fn(async () => {
				throw new BidExecutionPendingError(`0x${"44".repeat(32)}` as HexString, "receipt timed out")
			}),
		})
		const secondExecute = vi.fn(async () => makeResult(makeUserOp(SOLVER_TWO), SOLVER_TWO))
		const second = makeBid({ solverAddress: SOLVER_TWO, amount: 110n, execute: secondExecute })

		await expect(manager.selectAndExecuteBest(makeOrder(), [first, second])).rejects.toBeInstanceOf(
			BidExecutionPendingError,
		)
		expect(secondExecute).not.toHaveBeenCalled()
	})

	it("reports an accepted UserOp before waiting for its uncertain receipt", async () => {
		const ctx = { bundlerUrl: "x", intentsCoprocessor: {} } as never
		const manager = new BidManager(ctx, {} as never)
		const op = makeUserOp(SOLVER_ONE)
		const accepted = makeResult(op, SOLVER_ONE)
		const first = makeBid({
			solverAddress: SOLVER_ONE,
			amount: 120n,
			execute: vi.fn(async (onSubmitted) => {
				await onSubmitted?.(accepted)
				throw new BidExecutionPendingError(accepted.userOpHash, "receipt timed out")
			}),
		})
		const onSubmitted = vi.fn(async () => undefined)

		await expect(manager.selectAndExecuteBest(makeOrder(), [first], onSubmitted)).rejects.toBeInstanceOf(
			BidExecutionPendingError,
		)
		expect(onSubmitted).toHaveBeenCalledWith(accepted)
	})

	it("selects the next ranked bid in the same polling round after an already-known error", async () => {
		const firstExecute = vi.fn(async () => {
			throw new Error("Bundler error: already known")
		})
		const first = makeBid({ solverAddress: SOLVER_ONE, amount: 120n, execute: firstExecute })
		const secondExecute = vi.fn(async () => ({
			...makeResult(makeUserOp(SOLVER_TWO), SOLVER_TWO),
			fillStatus: "full" as const,
		}))
		const second = makeBid({ solverAddress: SOLVER_TWO, amount: 110n, execute: secondExecute })
		const rawBids: FillerBid[] = [first, second].map((bid, index) => ({
			filler: `solver-${index}`,
			userOp: bid.userOp,
			deposit: 0n,
		}))
		const getBidsForOrder = vi.fn(async () => rawBids)
		const persistedUserOps = new Map<string, string>()
		const persistUsedUserOps = vi.fn(async (key: string, value: string) => void persistedUserOps.set(key, value))
		let resolveDeadlineBlock: (block: bigint) => void = () => undefined
		const deadlineBlock = new Promise<bigint>((resolve) => {
			resolveDeadlineBlock = resolve
		})
		const ctx = {
			bundlerUrl: "http://bundler.test",
			intentsCoprocessor: { getBidsForOrder },
			dest: {
				config: { stateMachineId: "EVM-8453" },
				configService: { getEntryPointV08Address: () => ENTRY_POINT },
				client: {
					chain: { id: 8453, blockTime: 1 },
					getBlockNumber: vi.fn(() => deadlineBlock),
				},
			},
			usedUserOpsStorage: {
				getItem: vi.fn(async (key: string) => persistedUserOps.get(key) ?? null),
				setItem: persistUsedUserOps,
			},
		} as never
		const bidManager = new BidManager(ctx, {} as never)
		vi.spyOn(bidManager, "buildBids").mockReturnValue([first, second])
		const executor = new OrderExecutor(ctx, bidManager)
		const stream = executor.executeOrder({ order: makeOrder(), auctionTimeMs: 0, pollIntervalMs: 0 })

		expect((await stream.next()).value).toMatchObject({ status: "AWAITING_BIDS", commitment: COMMITMENT })

		const received = await stream.next()
		expect(received.value).toMatchObject({ status: "BIDS_RECEIVED", bidCount: 2 })
		if (received.done || received.value.status !== "BIDS_RECEIVED") throw new Error("Expected bids to be received")

		const selectedResult = await bidManager.selectAndExecuteBest(makeOrder(), received.value.bids)
		expect((await stream.next(selectedResult)).value).toMatchObject({
			status: "BID_SELECTED",
			selectedSolver: SOLVER_TWO,
		})
		expect((await stream.next()).value).toMatchObject({ status: "FILLED", selectedSolver: SOLVER_TWO })

		resolveDeadlineBlock(makeOrder().deadline)
		expect((await stream.next()).done).toBe(true)

		expect(getBidsForOrder).toHaveBeenCalledOnce()
		expect(firstExecute).toHaveBeenCalledOnce()
		expect(secondExecute).toHaveBeenCalledOnce()
		expect(persistUsedUserOps).toHaveBeenCalledOnce()
	})
})
