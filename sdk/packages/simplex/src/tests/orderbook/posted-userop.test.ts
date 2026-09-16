import { readFileSync } from "node:fs"
import { describe, expect, it } from "vitest"
import { recoverTypedDataAddress, type Hex } from "viem"
import { EvmChain, IntentGateway } from "@hyperbridge/sdk"
import { ContractInteractionService } from "@/services/ContractInteractionService"
import { privateKeySigner } from "@/services/wallet"
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
 * What the orderbook reads out of a posted op, and what it refuses it for.
 *
 * `orderbook-userops.json` is the orderbook's own golden vectors, copied from
 * `fixtures/userops.json` in polytope-labs/hyperfx-orderbook. Refresh it with
 * `gh api repos/polytope-labs/hyperfx-orderbook/contents/fixtures/userops.json -q .content | base64 -d`.
 *
 * Each vector names the verdict it expects, so the vectors are what keep
 * {@link refusalFor} honest: it has to accept the two the orderbook accepts and
 * refuse the rest for the documented reason. The op simplex posts then goes
 * through the same function. Checking our op against a hand-written list of
 * rules would only ever prove we agree with ourselves.
 */

interface Vectors {
	chainId: number
	entryPoint: HexString
	gateway: HexString
	cases: VectorCase[]
}

interface VectorCase {
	name: string
	description: string
	fillOptionsVersion: number
	scale: HexString
	commitment: HexString
	nonceKey: string
	userOpHash: HexString
	recoveredSigner: HexString
}

const vectors = JSON.parse(
	readFileSync(new URL("../fixtures/orderbook-userops.json", import.meta.url), "utf8"),
) as Vectors

/** The tokens the vectors trade, both six decimals on Base. */
const ONE_TIER = 1000n * 10n ** 6n

/** The shortest TTL the orderbook takes, as `serverInfo.minOrderTtlSecs` reports it. */
const MIN_TTL_SECS = 900n

/**
 * The `fillOrder` selector the vectors marked `fillOptionsVersion: 2` carry.
 *
 * Read off the vectors rather than written out, because a v1 payload decodes as
 * a `validUntil` of zero and only the selector tells that apart from a v2 one
 * that asks for zero seconds. The two refusals differ.
 */
const V2_SELECTOR = fillSelector(vectors.cases.find((entry) => entry.name === "v2_bid")!)

function fillSelector(entry: VectorCase): string {
	const op = decodeUserOpScale(entry.scale)
	const calls = decodeERC7821ExecuteBatch(op.callData)!
	return calls[calls.length - 1].data.slice(0, 10)
}

/**
 * The orderbook's refusal for this op, or null when it would take it.
 *
 * `UNSUPPORTED_CHAIN`, `UNSUPPORTED_PAIR`, `UNSUPPORTED_SOURCE_CHAIN`,
 * `REPLAYED`, `ORDER_EXISTS` and `TOO_MANY_ORDERS` are all about the server's
 * own config or state rather than the op, so nothing here can decide them.
 */
interface Posting {
	gateway: HexString
	entryPoint: HexString
	chainId: number
}

async function refusalFor(scale: HexString, posting: Posting): Promise<string | null> {
	const { gateway, entryPoint, chainId } = posting
	const op = decodeUserOpScale(scale)
	const calls = decodeERC7821ExecuteBatch(op.callData)
	if (!calls || op.signature.length < 66) return "MALFORMED_USER_OP"

	const fill = calls
		.filter((call) => call.target.toLowerCase() === gateway.toLowerCase())
		.map((call) => ({ selector: call.data.slice(0, 10), decoded: decodeFillOrder(call.data) }))
		.find((candidate) => candidate.decoded)
	if (!fill) return "NO_FILL_ORDER"

	const { order, options } = fill.decoded!
	if (fill.selector !== V2_SELECTOR) return "MISSING_VALID_UNTIL"

	// A phantom order asks for nothing and is not tied to a session: it is a
	// standing quote, not a fill of someone's order.
	if (order.session !== ADDRESS_ZERO) return "NOT_PHANTOM"
	if (order.output.assets.some((asset) => BigInt(asset.amount) !== 0n)) return "NOT_PHANTOM"

	if (options.outputs.length !== 1) return "UNSUPPORTED_SHAPE"
	const quoted = BigInt(options.outputs[0].amount)
	if (quoted === 0n) return "UNSUPPORTED_SHAPE"

	if (BigInt(options.validUntil) < MIN_TTL_SECS) return "TTL_TOO_SHORT"

	const { acceptedSources } = decodePhantomBidDeclaration(op.paymasterAndData)
	if (!acceptedSources) return "MISSING_DECLARATION"
	if (acceptedSources.length === 0) return "EMPTY_DECLARATION"

	// The vectors call this one BELOW_MIN_TIER in prose; the wire enum has always
	// said MIN_ORDER_SIZE, and the server's own floor comes from its config.
	if (quoted < ONE_TIER) return "MIN_ORDER_SIZE"

	// The nonce is covered by the userOpHash and the signature prefix is not, so
	// the binding that matters is the key, exactly as SolverAccount reads it.
	const commitment = op.signature.slice(0, 66) as HexString
	if (BigInt(op.nonce) >> 64n !== CryptoUtils.bidNonceKey(commitment, order.session as HexString)) {
		return "BAD_NONCE_BINDING"
	}

	const signer = await recoverSigner(op, entryPoint, chainId)
	if (signer.toLowerCase() !== op.sender.toLowerCase()) return "BAD_SIGNATURE"

	return null
}

/**
 * Who signed the op, over the bare userOpHash the EntryPoint derives. The
 * commitment prefix is not covered by that hash, so it is stripped first.
 */
async function recoverSigner(op: PackedUserOperation, entryPoint: HexString, chainId: number): Promise<string> {
	const typed = CryptoUtils.packedUserOpTypedData({ ...op, signature: "0x" }, entryPoint, BigInt(chainId))
	return recoverTypedDataAddress({
		...typed,
		signature: `0x${op.signature.slice(66)}` as Hex,
	})
}

/** The verdict each vector's description names. */
const EXPECTED: Record<string, string | null> = {
	v2_bid: null,
	v2_ask: null,
	v2_sponsored_declaration: null,
	v2_other_solver: null,
	v1_bid: "MISSING_VALID_UNTIL",
	v2_ttl_0: "TTL_TOO_SHORT",
	v2_ttl_899: "TTL_TOO_SHORT",
	v2_multi_leg: "UNSUPPORTED_SHAPE",
	v2_foreign_target: "NO_FILL_ORDER",
	v2_session_set: "NOT_PHANTOM",
	v2_requested_output: "NOT_PHANTOM",
	v2_zero_quote: "UNSUPPORTED_SHAPE",
	v2_below_one_tier: "MIN_ORDER_SIZE",
	v2_no_declaration: "MISSING_DECLARATION",
}

describe("the orderbook's golden vectors", () => {
	it("covers every vector in the file", () => {
		expect(vectors.cases.map((entry) => entry.name).sort()).toEqual(Object.keys(EXPECTED).sort())
	})

	it.each(vectors.cases.map((entry) => [entry.name, entry] as const))("%s", async (name, entry) => {
		expect(await refusalFor(entry.scale, vectors)).toBe(EXPECTED[name])
	})

	it("derives the commitment, nonce key and signer the orderbook records", async () => {
		const entry = vectors.cases.find((candidate) => candidate.name === "v2_bid")!
		const op = decodeUserOpScale(entry.scale)

		expect(op.signature.slice(0, 66)).toBe(entry.commitment)
		expect((BigInt(op.nonce) >> 64n).toString()).toBe(entry.nonceKey)
		expect(CryptoUtils.bidNonceKey(entry.commitment, ADDRESS_ZERO).toString()).toBe(entry.nonceKey)

		expect(await recoverSigner(op, vectors.entryPoint, vectors.chainId)).toBe(entry.recoveredSigner)
	})
})

/** Anvil's first account, which is the key the vectors were signed with. */
const SOLVER_KEY = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" as HexString
const USDC = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913" as HexString
const CNGN = "0x46C85152bFe9f96829aA94755D9f915F9B10EF5F" as HexString
const CHAIN = "EVM-8453"

/**
 * The real posting path over stubbed collaborators.
 *
 * Only two things are stood in for: the endpoints, which nothing on this path
 * reads, and the fee token the gateway warms its cache with, which costs a call
 * to a node. Everything the orderbook looks at is built by the code that builds
 * it in production.
 */
let cachedGateway: Promise<IntentGateway> | undefined

/** Built once: the solver account lookup inside `create` waits on a node that is not there. */
function intentGateway(): Promise<IntentGateway> {
	if (cachedGateway) return cachedGateway
	const chain = EvmChain.fromParams({
		chainId: vectors.chainId,
		host: "0x6FFe92e4d7a9D589549644544780e6725E84b248" as HexString,
		rpcUrl: "http://127.0.0.1:1",
	})
	// biome-ignore lint/suspicious/noExplicitAny: the fee token read is the one thing here that wants a node
	;(chain as any).getFeeTokenWithDecimals = async () => ({ address: USDC, decimals: 6 })
	cachedGateway = IntentGateway.create(chain, chain)
	return cachedGateway
}

async function postingService() {
	const signer = privateKeySigner(SOLVER_KEY)
	const gateway = await intentGateway()

	const configService = {
		loggers: undefined,
		getConfiguredChainIds: () => [],
		getIntentGatewayAddress: () => vectors.gateway,
		getRpcUrls: () => ["http://127.0.0.1:1"],
	}
	// biome-ignore lint/suspicious/noExplicitAny: narrow stubs for the collaborators this path touches
	const service = new ContractInteractionService({} as any, configService as any, signer)
	// biome-ignore lint/suspicious/noExplicitAny: the gateway is built above rather than from a node
	;(service as any).getIntentGateway = async () => gateway
	return { service, signer }
}

/** A limit order taking in 1,500,000 cNGN and paying out 1,000.5 USDC, as v2_bid does. */
async function postOne(overrides: { ttlSecs?: number; acceptedSourceChains?: string[] } = {}) {
	const { service, signer } = await postingService()
	const built = await service.prepareLimitOrderUserOp({
		fillChain: CHAIN,
		entryPointAddress: vectors.entryPoint,
		inputToken: CNGN,
		outputToken: USDC,
		inputAmount: 1_500_000_000_000n,
		outputAmount: 1_000_500_000n,
		orderNonce: 7n,
		ttlSecs: overrides.ttlSecs ?? 900,
		acceptedSourceChains: overrides.acceptedSourceChains ?? ["EVM-1", "EVM-8453"],
	})
	return { ...built, signer, op: decodeUserOpScale(built.userOp) }
}

describe("the op simplex posts", () => {
	it("is one the orderbook takes", async () => {
		const { userOp } = await postOne()
		expect(await refusalFor(userOp, vectors)).toBeNull()
	})

	it("carries the ttl where the orderbook reads it", async () => {
		// The service refuses a ttl this short before it ever builds an op. This is
		// about the field: a ttl written anywhere else would read as no bound at all.
		const { userOp } = await postOne({ ttlSecs: 899 })
		expect(await refusalFor(userOp, vectors)).toBe("TTL_TOO_SHORT")
	})

	it("quotes one leg, approved to the gateway first", async () => {
		const { op } = await postOne()
		const calls = decodeERC7821ExecuteBatch(op.callData)!

		expect(calls.map((call) => call.target.toLowerCase())).toEqual([
			USDC.toLowerCase(),
			vectors.gateway.toLowerCase(),
		])
		const { order, options } = decodeFillOrder(calls[1].data)!
		expect(options.outputs).toHaveLength(1)
		expect(BigInt(options.outputs[0].amount)).toBe(1_000_500_000n)
		expect(BigInt(options.validUntil)).toBe(900n)
		// Nothing is asked for and no session is set: a standing quote, not a fill.
		expect(order.session).toBe(ADDRESS_ZERO)
		expect(order.output.assets.every((asset) => BigInt(asset.amount) === 0n)).toBe(true)
	})

	it("declares the source chains the order accepts", async () => {
		const { op } = await postOne({ acceptedSourceChains: ["EVM-1", "EVM-42161"] })
		expect(decodePhantomBidDeclaration(op.paymasterAndData).acceptedSources).toEqual(["EVM-1", "EVM-42161"])
	})

	it("binds the nonce to the commitment and signs as the sender", async () => {
		const { commitment, op, signer } = await postOne()

		expect(op.sender.toLowerCase()).toBe(signer.address.toLowerCase())
		expect(op.signature.slice(0, 66)).toBe(commitment)
		expect(BigInt(op.nonce) >> 64n).toBe(CryptoUtils.bidNonceKey(commitment, ADDRESS_ZERO))
		// The low 64 bits are the sequence the EntryPoint counts within that key.
		expect(BigInt(op.nonce) & ((1n << 64n) - 1n)).toBe(0n)
	})
})
