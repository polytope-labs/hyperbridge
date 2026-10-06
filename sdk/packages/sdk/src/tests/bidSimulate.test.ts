import { ENTRY_POINT_V08, ENTRY_POINT_V09 } from "@/configs/chain"
import { BidImpl } from "@/protocols/intents/Bid"
import { CryptoUtils, LEGACY_SELECT_SOLVER_TYPEHASH, SELECT_SOLVER_TYPEHASH } from "@/protocols/intents/CryptoUtils"
import type { HexString, Order, PackedUserOperation } from "@/types"
import { concat, pad } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { describe, expect, it, vi } from "vitest"

const CHAIN_ID = 8453
const COMMITMENT = `0x${"66".repeat(32)}` as HexString
const TOKEN = pad("0x55", { size: 32 }) as HexString
const NATIVE = pad("0x", { size: 32 }) as HexString
const DISPATCHER = `0x${"d1".repeat(20)}` as HexString
const PARAMS_SLOT = `0x${"0".repeat(63)}5`
// The gateway's params slot 5: `solverSelection` on in byte 20, above the dispatcher.
const PARAMS_WORD = `0x${"00".repeat(11)}01${DISPATCHER.slice(2)}` as HexString
const solver = privateKeyToAccount("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")

const order: Order = {
	id: COMMITMENT,
	user: TOKEN,
	source: "EVM-1",
	destination: `EVM-${CHAIN_ID}`,
	deadline: 100n,
	nonce: 1n,
	fees: 0n,
	session: "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
	predispatch: { assets: [], call: "0x" },
	inputs: [{ token: TOKEN, amount: 100n }],
	output: { beneficiary: TOKEN, assets: [{ token: NATIVE, amount: 7n }], call: "0x" },
}

/** A bid for `entryPoint` on the gateway at `gateway`, which selects in the format `typehash` names. */
async function bidOn(entryPoint: HexString, gateway: HexString, typehash: HexString) {
	const unsigned: PackedUserOperation = {
		sender: solver.address,
		nonce: 1n,
		initCode: "0x",
		callData: "0x1234",
		accountGasLimits: `0x${"00".repeat(32)}`,
		preVerificationGas: 1n,
		gasFees: `0x${"00".repeat(32)}`,
		paymasterAndData: "0x",
		signature: "0x",
	}
	const solverSignature = await solver.signTypedData(
		CryptoUtils.packedUserOpTypedData(unsigned, entryPoint, BigInt(CHAIN_ID)),
	)
	const userOp = { ...unsigned, signature: concat([COMMITMENT, solverSignature]) }

	const client = {
		chain: { id: CHAIN_ID },
		readContract: vi.fn(async () => typehash),
		getStorageAt: vi.fn(async (_request: Record<string, unknown>) => PARAMS_WORD),
		call: vi.fn(async (_request: Record<string, unknown>) => ({ data: "0x" })),
	}
	// No session key anywhere: simulating a bid does not sign a selection.
	const ctx = {
		dest: {
			config: { stateMachineId: `EVM-${CHAIN_ID}` },
			configService: { getIntentGatewayAddress: () => gateway },
			client,
		},
	} as never
	const bid = new BidImpl({
		ctx,
		crypto: new CryptoUtils(ctx),
		order,
		fillerBid: { filler: "solver", bid: CryptoUtils.bidId(userOp.callData), userOp, deposit: 0n },
		fillOptions: {
			outputs: [{ token: NATIVE, amount: 7n }],
			inputs: [{ token: TOKEN, amount: 100n }],
			relayerFee: 0n,
			nativeDispatchFee: 3n,
			validUntil: 0n,
		},
		priceOutputs: async () => null,
	})
	return { bid, client, userOp }
}

describe("BidImpl.simulate", () => {
	it.each([
		["v0.9", "userOpHash", ENTRY_POINT_V09, SELECT_SOLVER_TYPEHASH, "0x0000000000000000000000000000000000000b01"],
		[
			"v0.8",
			"address",
			ENTRY_POINT_V08,
			LEGACY_SELECT_SOLVER_TYPEHASH,
			"0x0000000000000000000000000000000000000b02",
		],
	] as const)(
		"runs a %s bid's calldata with %s selection turned off and no select",
		async (_version, _format, entryPoint, typehash, gateway) => {
			const { bid, client, userOp } = await bidOn(entryPoint, gateway, typehash)

			await bid.simulate()

			expect(client.getStorageAt).toHaveBeenCalledWith({ address: gateway, slot: PARAMS_SLOT })
			// One call of the bid's own calldata: no batch that runs `select` first.
			expect(client.call).toHaveBeenCalledOnce()
			expect(client.call.mock.calls[0][0]).toEqual({
				account: solver.address,
				to: solver.address,
				data: userOp.callData,
				value: 10n,
				// The gateway's own dispatcher kept, the `solverSelection` byte above it cleared.
				stateOverride: [
					{
						address: gateway,
						stateDiff: [{ slot: PARAMS_SLOT, value: `0x${"0".repeat(24)}${DISPATCHER.slice(2)}` }],
					},
				],
			})
		},
	)

	it("fails when the call reverts", async () => {
		const { bid, client } = await bidOn(
			ENTRY_POINT_V09,
			"0x0000000000000000000000000000000000000b03",
			SELECT_SOLVER_TYPEHASH,
		)
		client.call.mockRejectedValue(new Error("execution reverted: Unauthorized()"))

		await expect(bid.simulate()).rejects.toThrow("Simulation failed: execution reverted: Unauthorized()")
	})
})
