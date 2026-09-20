import { describe, expect, it } from "vitest"
import { recoverTypedDataAddress, type Hex } from "viem"
import { BASE_CHAIN, BASE_CNGN, BASE_USDC, postingRig } from "../helpers/posting"
import {
	ADDRESS_ZERO,
	CryptoUtils,
	decodeERC7821ExecuteBatch,
	decodeFillOrder,
	decodePhantomBidDeclaration,
	decodeUserOpScale,
	type HexString,
	type PackedUserOperation,
} from "@hyperbridge/sdk"

/**
 * What the orderbook reads out of a posted op.
 *
 * The op is decoded the way the orderbook decodes it — `decodeUserOpScale`,
 * `decodeERC7821ExecuteBatch`, `decodeFillOrder`, `decodePhantomBidDeclaration`
 * — and every field it reads is checked where it expects to find it. A posting
 * is a signed message to a service that answers a misplaced field with a
 * rejection code and nothing else, so what is pinned here is the wire format
 * rather than the code path that produced it.
 *
 * There is one `fillOrder` shape: the one the gateways simplex fills on speak,
 * and the only one the SDK encodes or decodes.
 */

/** Base's IntentGateway and the EntryPoint the rig hands the builder. */
const GATEWAY = "0xAe041F7B0CB581876832830baeB6a2Aa2a3C9716" as HexString
const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString
const BASE_CHAIN_ID = 8453

/** The shortest TTL the orderbook takes, as `serverInfo.minOrderTtlSecs` reports it. */
const MIN_TTL_SECS = 900

/** A limit order taking in 1,500,000 cNGN and paying out 1,000.5 USDC. */
const TAKEN_IN = 1_500_000_000_000n
const PAID_OUT = 1_000_500_000n

async function postOne(overrides: { ttlSecs?: number; acceptedSourceChains?: string[] } = {}) {
	const { service, signer } = await postingRig({ gateway: GATEWAY, chainId: BASE_CHAIN_ID })
	const built = await service.prepareLimitOrderUserOp({
		fillChain: BASE_CHAIN,
		entryPointAddress: ENTRY_POINT,
		inputToken: BASE_CNGN,
		outputToken: BASE_USDC,
		inputAmount: TAKEN_IN,
		outputAmount: PAID_OUT,
		orderNonce: 7n,
		ttlSecs: overrides.ttlSecs ?? MIN_TTL_SECS,
		acceptedSourceChains: overrides.acceptedSourceChains ?? ["EVM-1", "EVM-8453"],
	})
	return { ...built, signer, op: decodeUserOpScale(built.userOp) }
}

/**
 * Who signed the op, over the bare userOpHash the EntryPoint derives. The
 * commitment prefix is not covered by that hash, so it is stripped first.
 */
async function recoverSigner(op: PackedUserOperation): Promise<string> {
	const typed = CryptoUtils.packedUserOpTypedData({ ...op, signature: "0x" }, ENTRY_POINT, BigInt(BASE_CHAIN_ID))
	return recoverTypedDataAddress({ ...typed, signature: `0x${op.signature.slice(66)}` as Hex })
}

describe("the op simplex posts", () => {
	it("quotes one leg, approved to the gateway first", async () => {
		const { op } = await postOne()
		const calls = decodeERC7821ExecuteBatch(op.callData)!

		expect(calls.map((call) => call.target.toLowerCase())).toEqual([BASE_USDC.toLowerCase(), GATEWAY.toLowerCase()])
		const { order, options } = decodeFillOrder(calls[1].data)!
		// The rate: the whole input taken for the output paid, one quote for the
		// order's one leg. The order's own output amount is zero, as the orderbook
		// requires, so this pair is where the price it reads actually lives.
		expect(options.outputs).toHaveLength(1)
		expect(BigInt(options.outputs[0].amount)).toBe(PAID_OUT)
		expect(options.inputs).toHaveLength(1)
		expect(BigInt(options.inputs[0].amount)).toBe(TAKEN_IN)
		expect(order.inputs).toHaveLength(1)
		expect(BigInt(order.inputs[0].amount)).toBe(TAKEN_IN)
		// Nothing is asked for and no session is set: a standing quote, not a fill.
		expect(order.session).toBe(ADDRESS_ZERO)
		expect(order.output.assets.every((asset) => BigInt(asset.amount) === 0n)).toBe(true)
	})

	it("carries the ttl where the orderbook reads it", async () => {
		// Seconds from the orderbook's receipt, in `validUntil`. A ttl written
		// anywhere else reads to the server as no bound at all.
		const { op } = await postOne({ ttlSecs: 1_800 })
		const calls = decodeERC7821ExecuteBatch(op.callData)!

		expect(BigInt(decodeFillOrder(calls[1].data)!.options.validUntil)).toBe(1_800n)
	})

	it("declares the source chains the order accepts", async () => {
		const { op } = await postOne({ acceptedSourceChains: ["EVM-1", "EVM-42161"] })
		expect(decodePhantomBidDeclaration(op.paymasterAndData).acceptedSources).toEqual(["EVM-1", "EVM-42161"])
	})

	it("refuses to build one that declares no source chain", async () => {
		// The orderbook answers an undeclared posting with `EMPTY_DECLARATION`, and
		// the encoder itself takes an empty list, so the guard has to be here.
		await expect(postOne({ acceptedSourceChains: [] })).rejects.toThrow(/accepted source chain/)
	})

	it("binds the nonce to the commitment and signs as the sender", async () => {
		const { commitment, op, signer } = await postOne()

		expect(op.sender.toLowerCase()).toBe(signer.address.toLowerCase())
		expect(op.signature.slice(0, 66)).toBe(commitment)
		expect(await recoverSigner(op)).toBe(signer.address)
		// The nonce is covered by the userOpHash and the signature prefix is not, so
		// the binding that matters is the key, exactly as SolverAccount reads it.
		expect(BigInt(op.nonce) >> 64n).toBe(CryptoUtils.bidNonceKey(commitment, ADDRESS_ZERO))
		// The low 64 bits are the sequence the EntryPoint counts within that key.
		expect(BigInt(op.nonce) & ((1n << 64n) - 1n)).toBe(0n)
	})
})
