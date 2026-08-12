import { BidImpl, BidSubmissionRejectedError } from "@/protocols/intents/Bid"
import { BidManager } from "@/protocols/intents/BidManager"
import { BundlerRpcError, CryptoUtils } from "@/protocols/intents/CryptoUtils"
import { OrderExecutor } from "@/protocols/intents/OrderExecutor"
import { BundlerMethod } from "@/protocols/intents/types"
import type { Bid, FillerBid, HexString, Order, PackedUserOperation, SelectBidResult } from "@/types"
import { describe, expect, it, vi } from "vitest"

const SOLVER_ONE = "0x1111111111111111111111111111111111111111" as HexString
const SOLVER_TWO = "0x2222222222222222222222222222222222222222" as HexString
const TOKEN = "0x3333333333333333333333333333333333333333" as HexString
const GATEWAY = "0x4444444444444444444444444444444444444444" as HexString
const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString
const COMMITMENT = `0x${"ab".repeat(32)}` as HexString
const SESSION = "0x5555555555555555555555555555555555555555" as HexString
const TX_HASH = `0x${"cd".repeat(32)}` as HexString
const SESSION_PRIVATE_KEY = `0x${"01".repeat(32)}` as HexString

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

function makeBid(params: { solverAddress: HexString; amount: bigint; execute: Bid["execute"] }): Bid {
	return {
		solverAddress: params.solverAddress,
		outputs: [{ token: TOKEN, amount: params.amount }],
		relayerFee: 0n,
		nativeDispatchFee: 0n,
		userOp: makeUserOp(params.solverAddress),
		simulate: vi.fn(async () => undefined),
		execute: params.execute,
		outputUsdValue: vi.fn(async () => null),
	}
}

describe("Order execution bid-selection integration", () => {
	it("selects the next ranked bid in the same polling round after a bundler rejection", async () => {
		const firstExecute = vi.fn(async () => {
			throw new BidSubmissionRejectedError(
				`0x${"01".repeat(32)}`,
				new BundlerRpcError(BundlerMethod.ETH_SEND_USER_OPERATION, {
					code: -32500,
					message: "AA23 reverted",
				}),
			)
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

describe("Bid execution retry integration", () => {
	it("resumes receipt tracking when the bundler reports an already-known UserOperation", async () => {
		const sendBundler = vi.fn(async (method: string) => {
			if (method === BundlerMethod.ETH_SEND_USER_OPERATION) {
				throw new BundlerRpcError(BundlerMethod.ETH_SEND_USER_OPERATION, {
					code: -32500,
					message: "already known",
				})
			}
			return { receipt: { transactionHash: TX_HASH } }
		})
		const userOp = makeUserOp()
		const ctx = {
			bundlerUrl: "http://bundler.test",
			dest: {
				config: { stateMachineId: "EVM-8453" },
				configService: {
					getIntentGatewayAddress: () => GATEWAY,
					getEntryPointV08Address: () => ENTRY_POINT,
				},
				client: {
					chain: { id: 8453 },
					waitForTransactionReceipt: vi.fn(async () => ({ logs: [] })),
				},
			},
			sessionKeyStorage: {
				getSessionKeyByAddress: vi.fn(async () => ({ privateKey: SESSION_PRIVATE_KEY })),
			},
		} as never
		const bid = new BidImpl({
			ctx,
			crypto: { sendBundler } as unknown as CryptoUtils,
			order: makeOrder(),
			fillerBid: { filler: "solver", userOp, deposit: 0n },
			fillOptions: { relayerFee: 0n, nativeDispatchFee: 0n, outputs: [{ token: TOKEN, amount: 100n }] },
			priceOutputs: async () => null,
		})

		const result = await bid.execute()
		const expectedHash = CryptoUtils.computeUserOpHash(result.userOp, ENTRY_POINT, 8453n)

		expect(result.userOpHash).toBe(expectedHash)
		expect(result.txnHash).toBe(TX_HASH)
		expect(sendBundler).toHaveBeenCalledTimes(2)
		expect(sendBundler.mock.calls[1]?.[0]).toBe(BundlerMethod.ETH_GET_USER_OPERATION_RECEIPT)
	})
})
