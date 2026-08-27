// Phantom-order price/liquidity aggregation. Lives in the SDK so the indexer (which persists the
// result as entities) and the simplex E2E test share one implementation. Bid decoding (extractFill)
// and signature recovery (recoverSigner) are injectable because the indexer must do both without
// viem — viem's @noble/hashes keccak throws "Uint8Array expected" in the SubQuery VM2 sandbox — so
// it passes VM2-safe implementations; the viem-based defaults are fine for Node consumers (tests,
// simplex).
import { decodeFunctionData, encodeAbiParameters, keccak256, recoverAddress } from "viem"
import { hexToU8a, isHex, stringToU8a, u8aToHex, u8aToString } from "@polkadot/util"
import { decodeERC7821ExecuteBatch } from "@/protocols/intents/decode-utils"
import { decodeUserOpScale } from "@/chains/intentsCoprocessor"
import { CryptoUtils } from "@/protocols/intents/CryptoUtils"
import type { PackedUserOperation } from "@/types"
import IntentGatewayV2 from "@/abis/IntentGatewayV2"
import { decodeFillOrder } from "./fillOrderCodec"
import {
	decodePoolAndPositionInfo,
	positionAmountOfToken,
	word,
	type PoolAndPositionInfo,
} from "@/protocols/intents/uniswap-v4-position"

export type HexString = `0x${string}`

/**
 * ERC-4337 v0.8 EntryPoint, the contract whose userOpHash a bid's solver signature is taken over.
 * Canonical across every EVM chain we support, which is why it is a constant here rather than
 * something callers pass in (chain.ts carries the same address per chain as `EntryPointV08`).
 */
export const ENTRY_POINT_V08_ADDRESS = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as const

/** Minimal fetch shape used by the JSON-RPC POSTs below. */
export type FetchLike = (url: string, init: any) => Promise<{ json(): Promise<any> }>

// The aggregation talks to RPCs over HTTP. In browsers/Node/tests the global `fetch` is used, but
// the SubQuery VM2 sandbox the indexer runs in does NOT expose a global `fetch` (and node-fetch
// crashes there), so the indexer injects a sandbox-safe implementation via setAggregationFetch().
let injectedFetch: FetchLike | undefined
export function setAggregationFetch(fetchImpl: FetchLike): void {
	injectedFetch = fetchImpl
}
function rpcFetch(): FetchLike {
	const f = injectedFetch ?? (globalThis as { fetch?: FetchLike }).fetch
	if (typeof f !== "function") {
		throw new Error("No fetch available; call setAggregationFetch() before using the aggregation helpers")
	}
	return f
}

/**
 * An RPC read the aggregation depends on could not be completed. Distinct from a bid being
 * rejected: a rejected bid is a fact about the bid, this is the absence of a fact. The two must
 * never be conflated — treating an unreachable node as "not delegated" or as "holds nothing"
 * silently drops a real solver from the snapshot, which then publishes a price and a depth
 * computed from whoever happened to answer.
 */
export class PhantomRpcError extends Error {
	constructor(
		message: string,
		readonly cause?: unknown,
	) {
		super(message)
		this.name = "PhantomRpcError"
	}
}

// POSTs a JSON-RPC payload and returns the parsed response, retrying with a short backoff. The node
// intermittently returns an empty body under concurrent load (a 200 with no payload), which makes
// response.json() throw; without a retry a single blip would silently drop a bid's quote or a whole
// window (fetchBids throws). Throws PhantomRpcError if every attempt fails.
async function rpcCall(url: string, payload: object): Promise<any> {
	let lastErr: unknown
	for (let attempt = 0; attempt < 4; attempt++) {
		if (attempt > 0) await new Promise((resolve) => setTimeout(resolve, 150 * attempt))
		let timer: ReturnType<typeof setTimeout> | undefined
		try {
			// Bound each attempt: the injected fetch (Node http) has no socket timeout, so a stalled
			// connection would otherwise hang forever and block the whole handler. Race it against a
			// deadline; on timeout we reject, retry, and ultimately throw so callers degrade instead.
			const timeout = new Promise<never>((_, reject) => {
				timer = setTimeout(() => reject(new Error(`rpc timeout: ${url}`)), 12_000)
			})
			const response = await Promise.race([
				rpcFetch()(url, {
					method: "POST",
					headers: { accept: "application/json", "content-type": "application/json" },
					body: JSON.stringify(payload),
				}),
				timeout,
			])
			const body = await response.json()
			// A JSON-RPC error body is a failed read, not an answer. Rate limiting arrives this way,
			// so letting it through as `undefined` is what turns throttling into fabricated data.
			if (body?.error) {
				lastErr = new Error(`rpc error: ${JSON.stringify(body.error).slice(0, 200)}`)
				continue
			}
			return body
		} catch (err) {
			lastErr = err
		} finally {
			if (timer) clearTimeout(timer)
		}
	}
	throw new PhantomRpcError(`RPC call failed after 4 attempts: ${url}`, lastErr)
}

export const FILL_ORDER_ABI = IntentGatewayV2.ABI

// ─── phantom bid declaration ────────────────────────────────────────────────────────────────────
//
// A same-chain phantom bid proves a solver operates on a chain, but not two things the snapshot
// needs: which chains it will accept payment FROM when filling a cross-chain order, and which
// Uniswap V4 positions stand behind its quote. Bids declare both inside paymasterAndData, which
// the userOpHash covers, so the declaration is authenticated by the solver's existing bid
// signature (the `signature` field is excluded from the hash and therefore unusable). The overload
// applies to phantom bids only: a real fill's paymasterAndData keeps its functional EntryPoint
// semantics, and nothing on the real-fill path ever parses this format.
//
// Layouts, by leading version byte:
//
//   v1  0x01 ‖ chainCount(1) ‖ chainCount × ( len(1) ‖ utf8 state machine id )
//   v2  0x01-body ‖ posCount(1) ‖ posCount × ( len(1) ‖ big-endian tokenId )   with 0x02 leading
//
// v2 is v1 with a positions section appended, so decoding is one parser with an optional tail.
// The encoder emits v1 whenever there are no positions, which keeps existing solvers' bids
// byte-identical to what they produce today; a v1 blob decodes under v2 readers as "no positions".
// A v2 blob read by a pre-v2 decoder fails its version check and reads as no declaration at all —
// the same degradation as an absent blob, never a misparse, because both versions reject trailing
// bytes rather than half-reading.
//
// Declared positions are implicitly on the order's own chain: a phantom bid is submitted per
// chain, so the tokenIds a solver names in it are the ones it holds on that chain. The declaration
// is a POINTER, not a claim of size — the indexer reads each position's liquidity on-chain and
// checks the position is owned by the very solver that signed the bid, so a solver can point at
// its own positions but cannot inflate them, nor borrow someone else's.

const DECLARATION_V1 = 0x01
const DECLARATION_V2 = 0x02

/** Upper bound on declared chains and positions alike; one byte of count each. */
const MAX_DECLARED_ENTRIES = 255

/** Widest tokenId the codec will carry — a uint256, as minted by the V4 PositionManager. */
const MAX_TOKEN_ID_BYTES = 32

/** What a phantom bid's paymasterAndData declares about the solver behind it. */
export interface PhantomBidDeclaration {
	/**
	 * Source chains the solver accepts payment from. Null when the bid carries no parseable
	 * declaration (the legacy default: the solver has not restricted its sources); an empty array
	 * is an explicit accepts-nothing. Callers must preserve that distinction.
	 */
	acceptedSources: string[] | null
	/**
	 * Uniswap V4 position tokenIds the solver declares as backing this bid, on the order's own
	 * chain. Empty when none are declared — including for every v1 bid, which predates the field.
	 */
	uniswapV4Positions: bigint[]
}

/** Minimal big-endian bytes of a non-negative tokenId; `[0]` for zero. */
function tokenIdToBytes(tokenId: bigint): number[] {
	if (tokenId < 0n) throw new Error(`Uniswap V4 tokenId cannot be negative: ${tokenId}`)
	const bytes: number[] = []
	let rest = tokenId
	while (rest > 0n) {
		bytes.unshift(Number(rest & 0xffn))
		rest >>= 8n
	}
	return bytes.length > 0 ? bytes : [0]
}

/**
 * Encodes a phantom bid's declaration into the paymasterAndData blob. Emits the v1 layout when no
 * positions are declared, so a solver that only names source chains produces exactly the bytes it
 * produced before positions existed.
 */
export function encodePhantomBidDeclaration(declaration: {
	acceptedSourceChains?: string[]
	uniswapV4Positions?: bigint[]
}): HexString {
	const chains = declaration.acceptedSourceChains ?? []
	const positions = declaration.uniswapV4Positions ?? []
	if (chains.length > MAX_DECLARED_ENTRIES) {
		throw new Error(`Cannot declare more than ${MAX_DECLARED_ENTRIES} source chains`)
	}
	if (positions.length > MAX_DECLARED_ENTRIES) {
		throw new Error(`Cannot declare more than ${MAX_DECLARED_ENTRIES} Uniswap V4 positions`)
	}

	const version = positions.length > 0 ? DECLARATION_V2 : DECLARATION_V1
	const bytes: number[] = [version, chains.length]
	for (const chain of chains) {
		const encoded = stringToU8a(chain)
		if (encoded.length === 0 || encoded.length > 255) {
			throw new Error(`Invalid state machine id in source chain declaration: ${chain}`)
		}
		bytes.push(encoded.length, ...encoded)
	}

	if (version === DECLARATION_V2) {
		bytes.push(positions.length)
		for (const tokenId of positions) {
			const encoded = tokenIdToBytes(tokenId)
			if (encoded.length > MAX_TOKEN_ID_BYTES) {
				throw new Error(`Uniswap V4 tokenId exceeds uint256: ${tokenId}`)
			}
			bytes.push(encoded.length, ...encoded)
		}
	}

	return u8aToHex(new Uint8Array(bytes)) as HexString
}

/**
 * Decodes a phantom bid's paymasterAndData. Understands both layout versions, so bids placed
 * before positions existed keep decoding unchanged. Anything absent, unversioned or malformed
 * yields a null `acceptedSources` with no positions — never a partial read.
 */
export function decodePhantomBidDeclaration(paymasterAndData: string | undefined | null): PhantomBidDeclaration {
	const absent: PhantomBidDeclaration = { acceptedSources: null, uniswapV4Positions: [] }
	if (!paymasterAndData || !isHex(paymasterAndData)) return absent
	const bytes = hexToU8a(paymasterAndData)
	if (bytes.length < 2) return absent

	const version = bytes[0]
	if (version !== DECLARATION_V1 && version !== DECLARATION_V2) return absent

	const chains: string[] = []
	let offset = 2
	for (let entry = 0; entry < bytes[1]; entry++) {
		if (offset >= bytes.length) return absent
		const length = bytes[offset]
		offset += 1
		if (length === 0 || offset + length > bytes.length) return absent
		chains.push(u8aToString(bytes.subarray(offset, offset + length)))
		offset += length
	}

	const positions: bigint[] = []
	if (version === DECLARATION_V2) {
		if (offset >= bytes.length) return absent
		const count = bytes[offset]
		offset += 1
		for (let entry = 0; entry < count; entry++) {
			if (offset >= bytes.length) return absent
			const length = bytes[offset]
			offset += 1
			if (length === 0 || length > MAX_TOKEN_ID_BYTES || offset + length > bytes.length) return absent
			let tokenId = 0n
			for (const byte of bytes.subarray(offset, offset + length)) tokenId = (tokenId << 8n) | BigInt(byte)
			positions.push(tokenId)
			offset += length
		}
	}

	// Trailing bytes mean this is not a declaration but something that happens to share the
	// version byte, so treat the whole blob as unparseable rather than half-reading it.
	if (offset !== bytes.length) return absent

	return { acceptedSources: chains, uniswapV4Positions: positions }
}

/** Back-compat wrapper: the source-chain half of {@link encodePhantomBidDeclaration}. */
export function encodeAcceptedSourceChains(chains: string[]): HexString {
	return encodePhantomBidDeclaration({ acceptedSourceChains: chains })
}

/** Back-compat wrapper: the source-chain half of {@link decodePhantomBidDeclaration}. */
export function decodeAcceptedSourceChains(paymasterAndData: string | undefined | null): string[] | null {
	return decodePhantomBidDeclaration(paymasterAndData).acceptedSources
}

/** ERC-4626 vaults per chain, keyed by chain id then lowercase underlying token address. */
export type YieldVaultMap = Record<string, Record<string, string[]>>

/** One leg of a fill: the token the solver pays out and the amount it quoted for that leg. */
export interface FillLeg {
	outputToken: HexString
	solverAmount: bigint
}

export interface FillData {
	order: Record<string, unknown>
	options: Record<string, unknown>
	/** Positional, matching the order's asset lists. A zero amount means the solver did not quote that leg. */
	legs: FillLeg[]
}

/**
 * Zips an order's output assets with the bid's quoted amounts into positional legs. A missing or
 * null amount is the "solver declined this leg" sentinel and becomes a zero quote. Shared by the
 * viem extractor below and the indexer's VM2-safe one, so the declined-leg convention has a
 * single home while only the ABI decode differs per environment.
 */
export function zipFillLegs(assets: { token: HexString }[], outputs: { amount: unknown }[]): FillLeg[] {
	return assets.map((asset, index) => {
		const rawAmount = outputs[index]?.amount
		return {
			outputToken: asset.token,
			solverAmount: rawAmount === undefined || rawAmount === null ? 0n : BigInt(rawAmount.toString()),
		}
	})
}

export interface RpcBidInfo {
	commitment: string
	filler: string
	user_op: string
}

/** One solver's measured liquidity for a configured token on one chain at this snapshot. */
export interface LpBalance {
	solver: string
	/** State machine id of the chain the balance was measured on (e.g. EVM-8453). */
	chain: string
	tokenAddress: HexString
	/**
	 * Wallet ERC-20, redeemable ERC-4626 vault shares, and the withdrawable amount held in the
	 * solver's declared Uniswap V4 positions — the same total the leg weights are built from, so a
	 * provider's reported inventory and the depth attributed to it cannot disagree.
	 *
	 * The V4 share is only ever included on the chain whose bid declared the positions: a bid is
	 * per chain, so no other chain's sweep can know about them. Consumers must therefore treat the
	 * larger of two readings for the same (chain, token, block) as the complete one.
	 */
	balance: bigint
}

/** One verified solver behind a leg's quote, holding inventory to deliver it. */
export interface PhantomLegBidder {
	solver: HexString
	/**
	 * The solver's output-token inventory on the destination chain — its weight in the median.
	 * Always greater than zero: a solver quoting a leg it holds none of is dropped, not recorded
	 * at zero, since it can deliver nothing at any price.
	 */
	weight: bigint
	/**
	 * Source chains the solver's signed paymasterAndData declaration accepts payment from. Null
	 * when the bid carries no declaration (legacy default: all CCTP/USDT0-covered chains); an
	 * empty array is an explicit accepts-nothing declaration.
	 */
	acceptedSources: string[] | null
}

/** The aggregated price for one leg of a phantom order. */
export interface PhantomLegAggregation {
	/** Position of the leg in the order's asset lists. */
	legIndex: number
	outputToken: HexString
	lowestPrice: bigint
	highestPrice: bigint
	medianPrice: bigint
	/** Backed quotes behind the price. Quotes from solvers holding no inventory are not counted. */
	bidCount: number
	/** The verified, inventory-backed solvers quoting this leg; bidCount === bidders.length. */
	bidders: PhantomLegBidder[]
}

/** The aggregated result for a single phantom order's bid window. */
export interface PhantomAggregation {
	/**
	 * One entry per leg that at least one solver quoted AND at least one of those quotes is backed
	 * by output-token inventory on the destination chain. Legs nobody quoted are absent, and so are
	 * legs every bidder quoted on zero inventory — neither is a price anyone could trade against.
	 */
	legs: PhantomLegAggregation[]
	lpBalances: LpBalance[]
}

export interface AggregationLogger {
	warn: (payload: unknown, message: string) => void
}

/**
 * Haircut applied to a quote that is priced off a Uniswap V4 pool, in basis points.
 *
 * A bid that declares V4 positions is quoting off those pools, and a pool price is what a trade
 * gets BEFORE the pool takes its fee — so the amount such a bid names is more than the solver
 * would actually be left holding once the swap that sources it clears. 30bps is the fee tier the
 * pools these positions sit in charge, so netting it out here is what makes a pool-priced quote
 * comparable to a wallet-funded one, whose inventory has already paid its cost of goods.
 */
export const UNISWAP_QUOTE_HAIRCUT_BPS = 30n

/** Applies {@link UNISWAP_QUOTE_HAIRCUT_BPS} to a quoted output amount, rounding down. */
export function applyUniswapQuoteHaircut(amount: bigint): bigint {
	return (amount * (10_000n - UNISWAP_QUOTE_HAIRCUT_BPS)) / 10_000n
}

// Liquidity-weighted median of solver quotes. Each quote's influence is proportional to `weight` —
// the solver's total balance for the output token across native + vault venues — so a solver that
// can actually deliver size moves the price more than one quoting on thin liquidity. Returns the
// lower weighted median: the smallest price whose cumulative weight reaches half of the total.
// Zero-weight quotes contribute nothing.
//
// Callers must not hand this an entry set whose weights are all zero: with nothing to weight by it
// can only pick a quote by position, and for an even-sized set that position is the upper of the
// two middles — so "the median" becomes "whoever quoted higher", settable by a solver holding no
// inventory at all. aggregatePhantomBids drops such legs instead of pricing them. The fallback
// below stays only so an unguarded caller gets a number rather than a crash; treat reaching it as
// a caller bug.
export function weightedMedian(entries: { price: bigint; weight: bigint }[]): bigint {
	const sorted = [...entries].sort((a, b) => (a.price < b.price ? -1 : a.price > b.price ? 1 : 0))
	const totalWeight = sorted.reduce((acc, e) => (e.weight > 0n ? acc + e.weight : acc), 0n)

	if (totalWeight === 0n) {
		return sorted[Math.floor(sorted.length / 2)].price
	}

	let cumulative = 0n
	for (const entry of sorted) {
		if (entry.weight <= 0n) continue
		cumulative += entry.weight
		if (cumulative * 2n >= totalWeight) return entry.price
	}
	return sorted[sorted.length - 1].price
}

// Pulls the inner fillOrder call out of the bid's ERC-7821 execute batch and decodes the order, the
// offered output token, and the solver's quoted amount. Returns null when no matching call targets
// the gateway or the calldata cannot be decoded.
export function extractFillData(callData: HexString, gatewayAddress: string): FillData | null {
	const calls = decodeERC7821ExecuteBatch(callData)
	if (!calls) return null

	const normalized = gatewayAddress.toLowerCase()
	for (const call of calls) {
		if (call.target.toLowerCase() !== normalized) continue
		try {
			// Bids come from solvers targeting whichever gateway they run against, so the
			// calldata may be either FillOptions shape. The selectors differ, so this cannot
			// mis-decode one as the other.
			const decoded = decodeFillOrder(call.data as HexString)
			if (!decoded) continue
			const order = decoded.order as unknown as Record<string, unknown>
			const options = decoded.options as unknown as Record<string, unknown>
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const assets = (order as any)?.output?.assets as { token: HexString }[] | undefined
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const outputs = (options as any)?.outputs as { amount: bigint }[] | undefined
			if (!assets?.length || !outputs?.length) continue
			return { order, options, legs: zipFillLegs(assets, outputs) }
		} catch {
			continue
		}
	}
	return null
}

/** Derives the 192-bit bid nonce key binding a bid to an (order, sessionKey) pair. */
export type BidNonceKeyFn = (commitment: HexString, sessionKey: HexString) => bigint

/** Recomputes an order's commitment from the contract-shaped order decoded out of a bid's calldata. */
export type OrderCommitmentFn = (order: Record<string, unknown>) => HexString | null

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const FILL_ORDER_INPUT = (FILL_ORDER_ABI as readonly any[]).find(
	(item) => item?.type === "function" && item?.name === "fillOrder",
)?.inputs?.[0]

/**
 * Default (viem) {@link OrderCommitmentFn}. The order comes straight out of `fillOrder`'s ABI
 * decode, so it is already contract-shaped and re-encoding it reproduces `keccak256(abi.encode(order))`
 * — the same commitment IntentGatewayV2 computes on-chain. Returns null when it cannot be computed,
 * which callers treat as unverifiable (fail closed).
 */
export function orderCommitmentFromDecoded(order: Record<string, unknown>): HexString | null {
	try {
		if (!FILL_ORDER_INPUT) return null
		return keccak256(encodeAbiParameters([FILL_ORDER_INPUT], [order as never]))
	} catch {
		return null
	}
}

// A bid's userOp.signature is `commitment (32) ‖ solverSignature (65)`. SolverAccount.validateUserOp
// expects 162 bytes on-chain, but the trailing 65-byte session-key signature is only appended at fill
// time, so a bid is stored and read back in this 97-byte form.
const BID_COMMITMENT_BYTES = 32
const SOLVER_SIGNATURE_BYTES = 65

export interface BidSignature {
	/** The order commitment the solver signature is bound to. */
	commitment: HexString
	/** The solver's 65-byte ECDSA signature over the userOpHash. */
	solverSignature: HexString
}

/** Splits a bid userOp's signature into its commitment and solver signature; null if malformed. */
export function splitBidSignature(signature: HexString): BidSignature | null {
	const raw = signature.replace(/^0x/, "")
	const commitmentChars = BID_COMMITMENT_BYTES * 2
	const end = commitmentChars + SOLVER_SIGNATURE_BYTES * 2
	if (raw.length < end) return null

	return {
		commitment: `0x${raw.slice(0, commitmentChars)}` as HexString,
		solverSignature: `0x${raw.slice(commitmentChars, end)}` as HexString,
	}
}

/** Recovers the address that produced a bid's solver signature, or null if it cannot be recovered. */
export type RecoverBidSigner = (
	userOp: PackedUserOperation,
	entryPoint: HexString,
	chainId: bigint,
	solverSignature: HexString,
) => Promise<HexString | null>

/**
 * Default {@link RecoverBidSigner}: recovers over the EntryPoint v0.8 userOpHash, the digest
 * SolverAccount itself validates against. viem-based, so Node consumers get it for free while the
 * indexer injects an ethers equivalent (see the note at the top of this file).
 */
export const recoverBidSignerViem: RecoverBidSigner = async (userOp, entryPoint, chainId, solverSignature) => {
	try {
		const userOpHash = CryptoUtils.computeUserOpHash(userOp, entryPoint, chainId)
		return (await recoverAddress({ hash: userOpHash, signature: solverSignature })) as HexString
	} catch {
		return null
	}
}

// An EOA that has delegated with EIP-7702 has code `0xef0100 ‖ delegate`.
const DELEGATION_INDICATOR_PREFIX = "0xef0100"

/** Whether `account` is an EOA EIP-7702-delegated to `solverAccount` on the given chain. */
async function isDelegatedToSolverAccount(evmRpcUrl: string, account: string, solverAccount: string): Promise<boolean> {
	const response = await rpcCall(evmRpcUrl, {
		id: 1,
		jsonrpc: "2.0",
		method: "eth_getCode",
		params: [account, "latest"],
	})
	// Anything that is not a code string means the node did not answer the question. Reading that
	// as "no delegation" is how a throttled endpoint silently unseats a real solver: the bid is
	// dropped, the legs only it quoted vanish, and the pool reports zero depth it actually has.
	if (typeof response.result !== "string") {
		throw new PhantomRpcError(`eth_getCode returned no code for ${account} on ${evmRpcUrl}`)
	}

	const code = response.result.toLowerCase()
	if (!code.startsWith(DELEGATION_INDICATOR_PREFIX)) return false

	return `0x${code.slice(DELEGATION_INDICATOR_PREFIX.length)}` === solverAccount.toLowerCase()
}

/** Promise-caching delegation reader produced by {@link memoizedDelegationCheck}. */
type DelegationReader = (evmRpcUrl: string, account: string, solverAccount: string) => Promise<boolean>

/**
 * Caches the delegation check for the life of one aggregation, retries included.
 *
 * Two reads are otherwise repeated for nothing. A solver running several fillers has its bid
 * copied under each, and the dedupe that collapses them runs only AFTER verification, so every
 * copy re-interrogated the chain for the same answer. And a retry re-verifies every solver, even
 * when the run was abandoned over an unrelated read — so the endpoint most likely to be throttled
 * got up to five times the load from the very code meant to survive it.
 *
 * Rejections are evicted, so a retry re-reads rather than replaying a failure as a verdict. A
 * negative result is cached: an EOA cannot gain a delegation part-way through one bid window, and
 * "not our solver" is an answer, unlike an unreachable node.
 */
function memoizedDelegationCheck(): DelegationReader {
	const cache = new Map<string, Promise<boolean>>()
	return (evmRpcUrl: string, account: string, solverAccount: string): Promise<boolean> => {
		const key = `${evmRpcUrl}|${account.toLowerCase()}|${solverAccount.toLowerCase()}`
		let pending = cache.get(key)
		if (!pending) {
			pending = isDelegatedToSolverAccount(evmRpcUrl, account, solverAccount).catch((err) => {
				cache.delete(key)
				throw err
			})
			cache.set(key, pending)
		}
		return pending
	}
}

/** Chain id out of an EVM state machine id ("EVM-8453" -> 8453n); null for any other format. */
function evmChainId(chain: string): bigint | null {
	const [prefix, id] = chain.split("-")
	if (prefix !== "EVM" || !id || !/^\d+$/.test(id)) return null
	return BigInt(id)
}

/**
 * Whether a bid genuinely came from one of our solvers, and so may influence the snapshot.
 *
 * Anyone can submit a bid to the coprocessor, and every accepted quote moves the weighted median the
 * rest of the protocol prices intents against, so a bid is only counted if it clears both of the
 * checks SolverAccount would apply on-chain: the userOp carries a solver signature over this order's
 * userOpHash that recovers to the sender, and the sender is EIP-7702-delegated to the chain's
 * SolverAccount. Fails closed — a bid that cannot be read or verified is not counted.
 */
async function isVerifiedSolverBid(params: {
	userOp: PackedUserOperation
	commitment: string
	sessionKey: HexString
	chainId: bigint
	solverAccount: string
	evmRpcUrl: string
	recoverSigner: RecoverBidSigner
	bidNonceKey: BidNonceKeyFn
	/** Cached per aggregation, so duplicate fillers and retries do not re-read the same answer. */
	isDelegated: DelegationReader
	logger?: AggregationLogger
}): Promise<boolean> {
	const {
		userOp,
		commitment,
		sessionKey,
		chainId,
		solverAccount,
		evmRpcUrl,
		recoverSigner,
		bidNonceKey,
		isDelegated,
		logger,
	} = params
	const solver = userOp.sender

	const parsed = splitBidSignature(userOp.signature)
	if (!parsed) {
		logger?.warn({ solver, commitment }, "Rejecting phantom bid: malformed userOp signature")
		return false
	}

	// Cheap early-out ONLY. The prefix sits inside userOp.signature, which userOpHash excludes, so it
	// is attacker-mutable and must never be what binds a bid to an order — the nonce key below is.
	if (parsed.commitment.toLowerCase() !== commitment.toLowerCase()) {
		logger?.warn(
			{ solver, commitment, signedFor: parsed.commitment },
			"Rejecting phantom bid: signed for another order",
		)
		return false
	}

	// The authoritative binding, mirroring SolverAccount.validateUserOp on-chain. The nonce IS
	// covered by userOpHash, so a solver signature stays valid only for the (order, sessionKey) pair
	// its nonce key was derived from. `sessionKey` is read from the bid's own calldata, which is also
	// covered by userOpHash — so every operand here is signed, leaving nothing for a replay to swap.
	if (BigInt(userOp.nonce) >> 64n !== bidNonceKey(commitment as HexString, sessionKey)) {
		logger?.warn({ solver, commitment }, "Rejecting phantom bid: nonce does not bind order and session key")
		return false
	}

	// SolverAccount._rawSignatureValidation recovers over the bare userOpHash and requires the signer
	// to be the account itself, which under EIP-7702 is the sender EOA.
	const signer = await recoverSigner(userOp, ENTRY_POINT_V08_ADDRESS, chainId, parsed.solverSignature)
	if (!signer || signer.toLowerCase() !== solver.toLowerCase()) {
		logger?.warn({ solver, commitment, signer }, "Rejecting phantom bid: signature does not recover to the sender")
		return false
	}

	if (!(await isDelegated(evmRpcUrl, solver, solverAccount))) {
		logger?.warn({ solver, commitment, solverAccount }, "Rejecting phantom bid: sender is not a delegated solver")
		return false
	}

	return true
}

export async function fetchBidsForOrder(nodeUrl: string, commitment: string): Promise<RpcBidInfo[]> {
	const data = await rpcCall(nodeUrl, {
		id: 1,
		jsonrpc: "2.0",
		method: "intents_getBidsForOrder",
		params: [commitment],
	})
	return Array.isArray(data.result) ? (data.result as RpcBidInfo[]) : []
}

// A balance read that fails is NOT a balance of zero. Zero weight now removes a leg from the
// snapshot entirely, so swallowing the failure would delete real liquidity on an RPC blip and
// leave a pool reporting depth it has. Only "0x" — the call reverted or there is no code at the
// address, i.e. the node did answer — is a genuine zero; everything else propagates.
async function ethCallUint(evmRpcUrl: string, to: string, data: string): Promise<bigint> {
	const result = await rpcCall(evmRpcUrl, {
		id: 1,
		jsonrpc: "2.0",
		method: "eth_call",
		params: [{ to, data }, "latest"],
	})
	if (result.result === "0x") return 0n
	if (typeof result.result !== "string") {
		throw new PhantomRpcError(`eth_call returned no result for ${to} on ${evmRpcUrl}`)
	}
	return BigInt(result.result)
}

// Sums the solver's redeemable balance of a single token on its destination chain: the raw ERC-20
// balance plus any ERC-4626 vault positions wrapping it.
async function getTotalSolverBalance(
	evmRpcUrl: string,
	chain: string,
	token: string,
	solver: string,
	yieldVaults: YieldVaultMap,
): Promise<bigint> {
	const padded = solver.replace("0x", "").padStart(64, "0")
	const raw = await ethCallUint(evmRpcUrl, token, `0x70a08231${padded}`) // balanceOf(address)
	const vaults = yieldVaults[chain]?.[token.toLowerCase()] ?? []
	const vaultBalances = await Promise.all(
		vaults.map((v) => ethCallUint(evmRpcUrl, v, `0xce96cb77${padded}`)), // maxWithdraw(address)
	)
	return vaultBalances.reduce((acc, b) => acc + b, raw)
}

/** PositionManager + StateView addresses for a chain's Uniswap V4 deployment. */
export interface UniswapV4Contracts {
	positionManager: string
	stateView: string
}

/** Everything a declared position's contribution depends on, read once and reused across legs. */
interface V4PositionState {
	/** Current on-chain owner. Compared against the bid's signer, never trusted from the bid. */
	owner: string
	info: PoolAndPositionInfo
	liquidity: bigint
	sqrtPriceX96: bigint
}

const SELECTOR_OWNER_OF = "0x6352211e"
const SELECTOR_POSITION_LIQUIDITY = "0x1efeed33"
const SELECTOR_POOL_AND_POSITION_INFO = "0x7ba03aad"
const SELECTOR_GET_SLOT0 = "0xc815641c"

const uint256Arg = (value: bigint) => value.toString(16).padStart(64, "0")

/**
 * Reads a declared position: who owns it, how much liquidity it holds, and the price its pool is
 * currently at. Null when the position does not exist — a solver may name a burned tokenId, which
 * is not an RPC failure and must not sink the run.
 */
async function readV4Position(
	evmRpcUrl: string,
	contracts: UniswapV4Contracts,
	tokenId: bigint,
	keccak: (hex: HexString) => HexString,
	logger?: AggregationLogger,
): Promise<V4PositionState | null> {
	const call = async (to: string, data: string): Promise<string | null> => {
		const result = await rpcCall(evmRpcUrl, {
			id: 1,
			jsonrpc: "2.0",
			method: "eth_call",
			params: [{ to, data }, "latest"],
		})
		if (result.result === "0x") return null
		if (typeof result.result !== "string") {
			throw new PhantomRpcError(`eth_call returned no result for ${to} on ${evmRpcUrl}`)
		}
		return result.result
	}

	const arg = uint256Arg(tokenId)
	const [ownerData, infoData, liquidityData] = await Promise.all([
		call(contracts.positionManager, `${SELECTOR_OWNER_OF}${arg}`),
		call(contracts.positionManager, `${SELECTOR_POOL_AND_POSITION_INFO}${arg}`),
		call(contracts.positionManager, `${SELECTOR_POSITION_LIQUIDITY}${arg}`),
	])
	// An empty ownerOf is a tokenId that was never minted or has been burned — a solver may name a
	// stale position, which is its problem and not ours.
	if (!ownerData) return null
	if (!infoData || !liquidityData) {
		logger?.warn(
			{ tokenId: tokenId.toString(), positionManager: contracts.positionManager },
			"Uniswap V4 position exists but its pool info or liquidity did not read back — check the configured PositionManager",
		)
		return null
	}

	const info = decodePoolAndPositionInfo(infoData)
	// The pool id is keccak of the PoolKey exactly as the chain returned it, so no re-encoding of
	// ours can disagree with the hash the pool was registered under.
	const slot0Data = await call(contracts.stateView, `${SELECTOR_GET_SLOT0}${keccak(info.poolKeyEncoded).slice(2)}`)
	// The position resolved, so its pool exists — an empty slot0 means the call went somewhere that
	// is not a StateView, i.e. a misconfigured address. Silence here is what let a wrong address
	// zero out every declared position indefinitely instead of failing where someone would see it.
	if (!slot0Data) {
		logger?.warn(
			{ tokenId: tokenId.toString(), stateView: contracts.stateView, evmRpcUrl },
			"Uniswap V4 slot0 read returned nothing for a live position — the configured StateView address is wrong",
		)
		return null
	}

	return {
		owner: `0x${ownerData.slice(-40)}`.toLowerCase(),
		info,
		liquidity: word(liquidityData, 0),
		sqrtPriceX96: word(slot0Data, 0),
	}
}

/** Promise-caching position reader: one set of reads per position, reused across every leg. */
function memoizedV4Position(keccak: (hex: HexString) => HexString, logger?: AggregationLogger) {
	const cache = new Map<string, Promise<V4PositionState | null>>()
	return (evmRpcUrl: string, chain: string, contracts: UniswapV4Contracts, tokenId: bigint) => {
		const key = `${chain}|${tokenId}`
		let pending = cache.get(key)
		if (!pending) {
			pending = readV4Position(evmRpcUrl, contracts, tokenId, keccak, logger).catch((err) => {
				cache.delete(key)
				throw err
			})
			cache.set(key, pending)
		}
		return pending
	}
}

/** Promise-caching balance reader produced by [`memoizedSolverBalance`]. */
export type SolverBalanceReader = (evmRpcUrl: string, chain: string, token: string, solver: string) => Promise<bigint>

// One aggregation run reads the same (chain, token, solver) balance from several places — once
// per leg the solver quoted in that token, and again in the liquidity sweep — and a bundled order
// can carry up to 128 legs. Memoizing for the life of the run collapses that to one RPC round
// trip per distinct triple, at the cost of ignoring balance changes within the run (the snapshot
// is a point-in-time read either way). Exported so a caller aggregating several orders whose bid
// windows close on the same block (one bundled order per configured chain) can share one memo
// across the runs instead of re-reading identical balances per order.
export function memoizedSolverBalance(yieldVaults: YieldVaultMap): SolverBalanceReader {
	const cache = new Map<string, Promise<bigint>>()
	return (evmRpcUrl: string, chain: string, token: string, solver: string): Promise<bigint> => {
		const key = `${chain}|${token.toLowerCase()}|${solver.toLowerCase()}`
		let pending = cache.get(key)
		if (!pending) {
			// Evict on rejection. Caching a failure would make it permanent for the memo's lifetime —
			// every retry would replay the same failed read, and a block-scoped memo would carry one
			// blip across every order closing on that block.
			pending = getTotalSolverBalance(evmRpcUrl, chain, token, solver, yieldVaults).catch((err) => {
				cache.delete(key)
				throw err
			})
			cache.set(key, pending)
		}
		return pending
	}
}

// Sweeps a solver's liquidity for every configured yield-vault token on every supported chain: for
// each chain that has both an RPC (in evmRpcUrls) and configured tokens (in yieldVaults), the
// solver's balance (raw ERC-20 + ERC-4626 vault positions) for each token. Captures the LP's whole
// liquidity picture, not just the token of the bid being priced. Zero balances are skipped so the
// snapshot only records tokens the solver actually holds.
async function sweepSolverLiquidity(
	evmRpcUrls: Record<string, string>,
	yieldVaults: YieldVaultMap,
	solver: string,
	getBalance: ReturnType<typeof memoizedSolverBalance>,
	/** Verified positions and the chain they sit on; only that chain's tokens can draw on them. */
	positions: { chain: string; states: V4PositionState[] },
): Promise<LpBalance[]> {
	const balances: LpBalance[] = []
	for (const [chain, tokens] of Object.entries(yieldVaults)) {
		const url = evmRpcUrls[chain]
		if (!url) continue
		for (const token of Object.keys(tokens)) {
			const balance = await getBalance(url, chain, token, solver)
			const uniswapV4Balance =
				chain === positions.chain
					? positions.states.reduce(
							(total, state) =>
								total +
								positionAmountOfToken({
									info: state.info,
									liquidity: state.liquidity,
									sqrtPriceX96: state.sqrtPriceX96,
									outputToken: token,
								}),
							0n,
						)
					: 0n
			// A token the solver neither holds nor has parked in a position is not a row.
			const total = balance + uniswapV4Balance
			if (total === 0n) continue
			balances.push({ solver, chain, tokenAddress: token as HexString, balance: total })
		}
	}
	return balances
}

// Strips a bytes32 token field to a 20-byte lowercase address (or normalises an address as-is).
function toAddress(token: string): HexString {
	const hex = token.toLowerCase().replace(/^0x/, "")
	const addr = hex.length > 40 ? hex.slice(-40) : hex.padStart(40, "0")
	return `0x${addr}` as HexString
}

/**
 * Aggregates every bid for a phantom order into a single price/liquidity snapshot.
 *
 * Fetches the live bids via `intents_getBidsForOrder` and reads each filler's quoted output amount.
 * Only bids that {@link isVerifiedSolverBid} accepts are counted — a bid from anyone who is not one
 * of our delegated solvers, or whose signature was not produced for this order, is dropped rather
 * than allowed to move the price. The liquidity-weighted median then weights every surviving quote by
 * the solver's balance of the output token on the destination chain, so a solver that can't actually
 * deliver size carries little or no weight — which is why no fill simulation is needed to filter
 * unfillable quotes. For each bidding solver it also records a full liquidity sweep — every
 * configured yield-vault token on every supported chain (raw ERC-20 + vault positions). Returns
 * `null` when no bid survives verification.
 *
 * `extractFill` decodes a bid's ERC-7821 calldata into the fill's order/output and `recoverSigner`
 * recovers its solver signature; both default to the viem implementations, but the indexer injects
 * VM2-safe variants (viem's keccak throws in the SubQuery sandbox).
 *
 * The whole run is retried up to {@link AGGREGATION_ATTEMPTS} times on any error, because every
 * error that escapes the per-bid handling means some input could not be read, and a snapshot
 * computed from a partial bid set is worse than none: it publishes a confident price and zeroes
 * depth that exists. Throws if every attempt fails, leaving the window unsnapshotted — consumers
 * see the previous rate with a stale lastUpdatedBlock, which is a state they can already detect.
 */
export async function aggregatePhantomBids(
	params: Parameters<typeof runAggregation>[0],
): Promise<PhantomAggregation | null> {
	// Built out here, not per attempt: delegation cannot change within a bid window, so a retry
	// forced by one unrelated failed read must not re-interrogate every solver it already verified.
	const isDelegated = memoizedDelegationCheck()
	let lastErr: unknown
	for (let attempt = 1; attempt <= AGGREGATION_ATTEMPTS; attempt++) {
		try {
			return await runAggregation(params, isDelegated)
		} catch (err) {
			lastErr = err
			params.logger?.warn(
				{ err, commitment: params.commitment, chain: params.chain, attempt, of: AGGREGATION_ATTEMPTS },
				"Phantom aggregation attempt failed",
			)
			if (attempt < AGGREGATION_ATTEMPTS) await new Promise((resolve) => setTimeout(resolve, 250 * attempt))
		}
	}
	throw lastErr
}

/** How many times a failed aggregation run is retried before the window is given up on. */
export const AGGREGATION_ATTEMPTS = 5

async function runAggregation(
	params: {
		nodeUrl: string
		/** RPC URL per supported EVM chain (stateMachineId -> url); must include the destination chain. */
		evmRpcUrls: Record<string, string>
		chain: string
		gatewayAddress: string
		commitment: string
		yieldVaults: YieldVaultMap
		/** SolverAccount on `chain` that our solvers delegate to; bids from anyone else are dropped. */
		solverAccount: string
		extractFill?: (callData: HexString, gatewayAddress: string) => FillData | null
		recoverSigner?: RecoverBidSigner
		bidNonceKey?: BidNonceKeyFn
		orderCommitment?: OrderCommitmentFn
		/**
		 * Balance reader shared across runs; defaults to a fresh per-run memo. Pass one built with
		 * `memoizedSolverBalance` when aggregating several same-block orders, and build it from the
		 * same `yieldVaults` passed here — the memo bakes in the vault map its balances include.
		 */
		getBalance?: SolverBalanceReader
		/**
		 * Uniswap V4 deployment per chain. Supply it to let bids that declare positions have those
		 * positions counted; without it a declaration is simply ignored, and the weight stays the
		 * plain balance as before.
		 */
		uniswapV4?: Record<string, UniswapV4Contracts>
		/** keccak256 over hex; defaults to viem's, which the VM2 sandbox must replace. */
		keccak?: (hex: HexString) => HexString
		logger?: AggregationLogger
	},
	isDelegated: DelegationReader,
): Promise<PhantomAggregation | null> {
	const { nodeUrl, evmRpcUrls, chain, gatewayAddress, commitment, yieldVaults, solverAccount, logger } = params
	const extractFill = params.extractFill ?? extractFillData
	const recoverSigner = params.recoverSigner ?? recoverBidSignerViem
	const bidNonceKey = params.bidNonceKey ?? CryptoUtils.bidNonceKey
	const orderCommitment = params.orderCommitment ?? orderCommitmentFromDecoded

	const destUrl = evmRpcUrls[chain]
	if (!destUrl) return null

	// Both bid checks need these, and neither can be skipped without letting an unverified quote into
	// the price, so a chain we can't resolve them for produces no snapshot at all. solverAccount is
	// typed as required but comes from a config lookup that can miss, so it is re-checked here.
	const chainId = evmChainId(chain)
	if (!solverAccount || chainId === null) {
		logger?.warn({ chain, commitment }, "Cannot verify phantom bids: no SolverAccount or chain id for chain")
		return null
	}

	const bids = await fetchBidsForOrder(nodeUrl, commitment)
	if (bids.length === 0) return null

	// One set of reads per declared position, reused by every leg it backs.
	const v4Contracts = params.uniswapV4?.[chain]
	const readPosition = memoizedV4Position(params.keccak ?? keccak256, logger)

	// Balances repeat across legs and the sweep; one memo serves the whole run.
	const getBalance = params.getBalance ?? memoizedSolverBalance(yieldVaults)

	// Quotes per leg, keyed by the leg's position in the order's asset lists. One bid carries a quote
	// for every leg its solver priced, so a solver that only handles some of the pairs still counts
	// towards those, and legs it skipped are left to the solvers that do handle them.
	const quotesByLeg = new Map<
		number,
		{
			outputToken: HexString
			quotes: { price: bigint; weight: bigint }[]
			bidders: PhantomLegBidder[]
		}
	>()
	const lpBalances: LpBalance[] = []
	// Bids are stored per substrate filler, but weight is a property of the EVM solver. Without this
	// one solver's bid, copied under N funded fillers, would count N times in the weighted median.
	const countedSolvers = new Set<string>()

	for (const bid of bids) {
		if (!bid.user_op) continue
		try {
			const decoded = decodeUserOpScale(bid.user_op as HexString)
			const solver = decoded.sender

			const fillData = extractFill(decoded.callData as HexString, gatewayAddress)
			if (!fillData) continue

			// The quoted price is read out of this order, so it must be the order being priced.
			const decodedCommitment = orderCommitment(fillData.order)
			if (!decodedCommitment || decodedCommitment.toLowerCase() !== commitment.toLowerCase()) {
				logger?.warn({ solver, commitment }, "Rejecting phantom bid: calldata order is not the indexed order")
				continue
			}

			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const sessionKey = (fillData.order as any)?.session as HexString | undefined
			if (!sessionKey) {
				logger?.warn({ solver, commitment }, "Rejecting phantom bid: order carries no session key")
				continue
			}

			const verified = await isVerifiedSolverBid({
				userOp: decoded,
				commitment,
				sessionKey,
				chainId,
				solverAccount,
				evmRpcUrl: destUrl,
				recoverSigner,
				bidNonceKey,
				isDelegated,
				logger,
			})
			if (!verified) continue

			const normalizedSolver = solver.toLowerCase()
			if (countedSolvers.has(normalizedSolver)) {
				logger?.warn({ solver, commitment }, "Skipping phantom bid: solver already counted for this order")
				continue
			}
			countedSolvers.add(normalizedSolver)

			// The declaration rides in paymasterAndData, which the userOpHash covers, so it carries
			// the same authenticity as the quote itself.
			const declaration = decodePhantomBidDeclaration(decoded.paymasterAndData)
			const acceptedSources = declaration.acceptedSources

			// A zero amount is how a solver declines a leg it does not price, so it is not a quote.
			// Weights are fetched concurrently: the memo caches promises, so identical output tokens
			// across legs still collapse to a single RPC round trip.
			//
			// A bid that names V4 positions is priced off those pools, so its quote is haircut by
			// the pool fee before anything downstream reads it — the median, the bidder rows, and
			// the zero-check right here, which then treats a quote the haircut rounds away exactly
			// as it treats a declined one. The declaration drives this rather than the positions
			// that survive the ownership check below, so the quote is haircut on the same basis
			// the solver priced it on, whether or not this chain has V4 contracts configured.
			const poolPriced = declaration.uniswapV4Positions.length > 0
			const quotedLegs = [...fillData.legs.entries()]
				.map(([legIndex, leg]): [number, FillLeg] =>
					poolPriced
						? [legIndex, { ...leg, solverAmount: applyUniswapQuoteHaircut(leg.solverAmount) }]
						: [legIndex, leg],
				)
				.filter(([, leg]) => leg.solverAmount !== 0n)

			// Liquidity a solver parked in a Uniswap V4 position is real capacity but holds no ERC-20
			// balance, so without this it reads as zero and the leg is dropped. The bid only NAMES
			// the positions; ownership is checked against the signer here and the amounts come off
			// the chain, so naming more, or naming someone else's, buys nothing.
			const declaredPositions = v4Contracts ? declaration.uniswapV4Positions : []
			const positions = (
				await Promise.all(
					declaredPositions.map((tokenId) => readPosition(destUrl, chain, v4Contracts!, tokenId)),
				)
			).filter((state, index): state is V4PositionState => {
				if (!state) return false
				if (state.owner !== normalizedSolver) {
					logger?.warn(
						{ solver, commitment, tokenId: declaredPositions[index].toString(), owner: state.owner },
						"Ignoring declared Uniswap V4 position: not owned by the bidding solver",
					)
					return false
				}
				return true
			})

			const weights = await Promise.all(
				// Price influence: the solver's liquidity in THIS leg's output token on the destination
				// chain, so a leg is weighted by the inventory that actually backs it.
				quotedLegs.map(async ([, leg]) => {
					const outputToken = toAddress(leg.outputToken)
					const balance = await getBalance(destUrl, chain, outputToken, solver)
					return positions.reduce(
						(total, state) =>
							total +
							positionAmountOfToken({
								info: state.info,
								liquidity: state.liquidity,
								sqrtPriceX96: state.sqrtPriceX96,
								outputToken,
							}),
						balance,
					)
				}),
			)
			for (const [position, [legIndex, leg]] of quotedLegs.entries()) {
				const weight = weights[position]
				const entry = quotesByLeg.get(legIndex) ?? { outputToken: leg.outputToken, quotes: [], bidders: [] }
				entry.quotes.push({ price: leg.solverAmount, weight })
				entry.bidders.push({ solver: normalizedSolver as HexString, weight, acceptedSources })
				quotesByLeg.set(legIndex, entry)
			}

			// Full liquidity picture: every configured token on every supported chain. Swept once per
			// bid rather than per leg, since it measures the solver's whole inventory either way.
			lpBalances.push(
				...(await sweepSolverLiquidity(evmRpcUrls, yieldVaults, solver, getBalance, {
					chain,
					states: positions,
				})),
			)
		} catch (err) {
			// A malformed bid is this bid's problem — skip it and price the rest. An unreachable RPC
			// is the snapshot's problem: continuing would silently price the order from whichever
			// bids happened to be readable, so abandon the run and let the retry above re-read.
			if (err instanceof PhantomRpcError) throw err
			logger?.warn({ err, filler: bid.filler }, "Failed to process bid for price snapshot")
		}
	}

	if (quotesByLeg.size === 0) return null

	// Each leg reports a single price: the liquidity-weighted median of the quotes for that leg.
	// lowestPrice and highestPrice carry that same value rather than the raw min/max of the bid set,
	// so consumers cannot read an outlier bid as if it were a tradeable bound.
	//
	// A quote's weight is the solver's inventory in THAT leg's output token on the destination
	// chain, so a zero-weight quote is one its solver cannot deliver at any price. Those are
	// dropped outright rather than merely down-weighted: they must not reach weightedMedian (with
	// nothing to weight by it picks a quote by position, letting whoever quotes the extreme set the
	// rate on zero capital), and they must not reach bidCount or `bidders`, where they would inflate
	// the solver count behind a price and mint zero-capacity PoolBidder/PoolRoute rows downstream.
	// A leg left with no backed quote at all is therefore absent entirely, exactly as if nobody had
	// quoted it — no snapshot, and its depth zeroes out downstream.
	const legs = [...quotesByLeg.entries()]
		.sort(([a], [b]) => a - b)
		.flatMap(([legIndex, { outputToken, quotes, bidders }]) => {
			// quotes and bidders are pushed in lockstep above, so the same predicate keeps them aligned.
			const backedQuotes = quotes.filter((quote) => quote.weight > 0n)
			const backedBidders = bidders.filter((bidder) => bidder.weight > 0n)
			if (backedQuotes.length === 0) {
				logger?.warn(
					{ commitment, chain, legIndex, outputToken, quotes: quotes.length },
					"Dropping phantom leg: no bidder holds the output token on this chain, so no quote is backed",
				)
				return []
			}

			const medianPrice = weightedMedian(backedQuotes)
			return [
				{
					legIndex,
					outputToken,
					lowestPrice: medianPrice,
					highestPrice: medianPrice,
					medianPrice,
					bidCount: backedQuotes.length,
					bidders: backedBidders,
				},
			]
		})

	return { legs, lpBalances }
}
