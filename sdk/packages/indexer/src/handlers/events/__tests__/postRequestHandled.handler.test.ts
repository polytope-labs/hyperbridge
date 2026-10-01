;(global as any).logger = { debug: jest.fn(), info: jest.fn(), warn: jest.fn(), error: jest.fn() }
;(global as any).chainId = "137"
;(global as any).store = { get: jest.fn(async () => undefined), set: jest.fn() }

jest.mock("@/utils/rpc.helpers", () => ({ getBlockTimestamp: async () => 1_700n }))
jest.mock("@/utils/substrate.helpers", () => ({ getHostStateMachine: () => "EVM-137" }))
jest.mock("@/services/request.service", () => ({ RequestService: { updateStatus: jest.fn() } }))
jest.mock("@/services/hyperbridge.service", () => ({ HyperBridgeService: { handleRequestHandledEvent: jest.fn() } }))
jest.mock("@/services/transfer.service", () => ({ TransferService: { storeTransfer: jest.fn() } }))
jest.mock("@/services/volume.service", () => ({ VolumeService: { updateVolume: jest.fn() } }))
jest.mock("@/utils/transfer.helpers", () => ({
	...jest.requireActual("@/utils/transfer.helpers"),
	getPriceDataFromEthereumLog: jest.fn(async () => ({ symbol: "USDC", decimals: 6, amountValueInUSD: "25" })),
}))

import { Interface } from "@ethersproject/abi"

import HandlerV2Abi from "@/configs/abis/HandlerV2.abi.json"
import { handlePostRequestHandledEvent } from "@/handlers/events/evmHost/postRequestHandled.event.handler"
import { RequestService } from "@/services/request.service"
import { TransferService } from "@/services/transfer.service"
import { VolumeService } from "@/services/volume.service"

const handler = new Interface(HandlerV2Abi)

const HOST = "0x620128e2b19193d6bd244a3ac8d3bba0541b19c3"
const GATEWAY = "0xae041f7b0cb581876832830baeb6a2aa2a3c9716"
const TOKEN = "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359"
const BENEFICIARY = "0x3333333333333333333333333333333333333333"
const ERC20_TRANSFER_TOPIC = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
const transactionHash = `0x${"22".repeat(32)}`

const pad32 = (address: string) => `0x${address.slice(2).padStart(64, "0")}`

/** What a relayer sends today: a consensus update and the requests it proves, in one batchCall. */
const batchedDelivery = handler.encodeFunctionData("batchCall", [
	[
		handler.encodeFunctionData("handleConsensus", [HOST, "0xdeadbeef"]),
		handler.encodeFunctionData("handlePostRequests", [
			HOST,
			{
				proof: { height: { stateMachineId: 3367n, height: 100n }, multiproof: [], leafCount: 1n },
				requests: [
					{
						request: {
							source: "0x504f4c4b41444f542d33333637",
							dest: "0x45564d2d313337",
							nonce: 7n,
							from: GATEWAY,
							to: GATEWAY,
							timeoutTimestamp: 0n,
							body: "0x1234",
						},
						index: 0n,
					},
				],
			},
		]),
	],
])

/** A PostRequestHandled event whose transaction moved 25 USDC out of the gateway. */
const event = (input: string) =>
	({
		args: { relayer: BENEFICIARY, commitment: `0x${"11".repeat(32)}` },
		block: { hash: `0x${"33".repeat(32)}` },
		blockHash: `0x${"33".repeat(32)}`,
		blockNumber: 99,
		transactionHash,
		transaction: {
			input,
			logs: [
				{
					address: TOKEN,
					transactionHash,
					topics: [ERC20_TRANSFER_TOPIC, pad32(GATEWAY), pad32(BENEFICIARY)],
					data: `0x${(25_000_000).toString(16).padStart(64, "0")}`,
				},
			],
		},
	}) as any

beforeEach(() => jest.clearAllMocks())

it("records the transfers of a batched delivery and attributes them to the request's recipient", async () => {
	await handlePostRequestHandledEvent(event(batchedDelivery))

	expect(RequestService.updateStatus).toHaveBeenCalledTimes(1)
	expect(TransferService.storeTransfer).toHaveBeenCalledWith(
		expect.objectContaining({ chain: "EVM-137", value: 25_000_000n, from: GATEWAY, to: BENEFICIARY }),
	)
	expect(jest.mocked(VolumeService.updateVolume).mock.calls).toEqual([
		["Transfer.USDC", "25", 1_700n],
		[`Contract.${GATEWAY}`, "25", 1_700n],
	])
	expect(logger.error).not.toHaveBeenCalled()
})

it("still records the transfers when the calldata is not a handler call it knows", async () => {
	await handlePostRequestHandledEvent(event("0xa9059cbb" + "00".repeat(64)))

	expect(TransferService.storeTransfer).toHaveBeenCalledTimes(1)
	expect(jest.mocked(VolumeService.updateVolume).mock.calls).toEqual([["Transfer.USDC", "25", 1_700n]])
	expect(logger.error).not.toHaveBeenCalled()
})
