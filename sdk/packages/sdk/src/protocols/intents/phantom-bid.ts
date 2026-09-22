// Phantom bids: the signed `fillOrder` UserOps solvers post to the HyperFX orderbook, and the
// declaration of accepted source chains they carry in `paymasterAndData`. The indexer imports
// these through the `intents-helpers` entry, which must load without viem in the SubQuery VM2
// sandbox, so nothing here reaches for viem.
import { hexToU8a, isHex, u8aToHex } from "@polkadot/util"
import type { RpcBidInfo } from "@/types"

export type HexString = `0x${string}`

/** Minimal fetch shape used by the JSON-RPC POSTs below. */
export type FetchLike = (url: string, init: any) => Promise<{ json(): Promise<any> }>

// Bid reads go to the node over HTTP. In browsers/Node/tests the global `fetch` is used, but the
// SubQuery VM2 sandbox the indexer runs in does NOT expose a global `fetch` (and node-fetch crashes
// there), so the indexer injects a sandbox-safe implementation via setAggregationFetch().
let injectedFetch: FetchLike | undefined
export function setAggregationFetch(fetchImpl: FetchLike): void {
	injectedFetch = fetchImpl
}
function rpcFetch(): FetchLike {
	const f = injectedFetch ?? (globalThis as { fetch?: FetchLike }).fetch
	if (typeof f !== "function") {
		throw new Error("No fetch available; call setAggregationFetch() before reading bids")
	}
	return f
}

// POSTs a JSON-RPC payload and returns the parsed response, retrying with a short backoff. The node
// intermittently returns an empty body under concurrent load (a 200 with no payload), which makes
// response.json() throw; without a retry a single blip would silently drop a bid's quote or a whole
// window (fetchBids throws). Throws if every attempt fails.
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
	throw new Error(`RPC call failed after 4 attempts: ${url}`, { cause: lastErr })
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

// ─── phantom bid declaration ────────────────────────────────────────────────────────────────────
//
// A same-chain phantom bid proves a solver operates on a chain, but not which chains it will accept
// payment FROM when filling a cross-chain order. Bids declare that inside paymasterAndData, which
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
// The v2 positions tail named Uniswap V4 positions backing a bid. Nothing prices them any more —
// the HyperFX orderbook parses a v2 declaration and discards its tokenIds — but the tail stays in
// the codec so bids solvers already sign keep decoding.
//
// Two shapes of paymasterAndData are on the wire, and the decoder reads both:
//
//   bare         declaration                                     (the original phantom-bid shape)
//   sponsored    paymaster(20) ‖ verificationGasLimit(16) ‖ postOpGasLimit(16)
//                ‖ 0x02 ‖ token(20) ‖ permitAmount(32) ‖ nonce(32) ‖ deadline(32) ‖ v(1) ‖ r(32) ‖ s(32)
//                ‖ [declaration]
//
// The sponsored shape is the EntryPoint v0.8 paymasterAndData a real bid carries, with the Simplex
// paymaster's PERMIT2 mode section (a per-op Permit2 SignatureTransfer permit, see
// SimplexPaymaster._parsePermit2Data). A solver that builds its phantom bid on the same path as a
// real bid produces exactly this, with the declaration appended after the permit; the paymaster
// never parses a phantom bid, so the trailing bytes cost nothing. The declaration is optional there
// — a sponsored bid with no tail is a solver that declared nothing, the same as an empty field.

const DECLARATION_V1 = 0x01
const DECLARATION_V2 = 0x02

/** EntryPoint v0.8: paymaster address, then two uint128 gas limits, before the paymaster's own data. */
const PAYMASTER_ADDRESS_BYTES = 20
const PAYMASTER_GAS_LIMIT_BYTES = 16
const PAYMASTER_DATA_OFFSET = PAYMASTER_ADDRESS_BYTES + 2 * PAYMASTER_GAS_LIMIT_BYTES

/** SimplexPaymaster's mode byte for a Permit2 SignatureTransfer permit — the mode every simplex bid authorizes with. */
const SIMPLEX_MODE_PERMIT2 = 0x02
/** mode(1) + token(20) + permitAmount(32) + nonce(32) + deadline(32) + v(1) + r(32) + s(32). */
const PERMIT2_DATA_BYTES = 1 + 20 + 32 + 32 + 32 + 1 + 32 + 32
/** A complete Permit2-sponsored paymasterAndData, before any declaration is appended. */
export const PERMIT2_SPONSORSHIP_BYTES = PAYMASTER_DATA_OFFSET + PERMIT2_DATA_BYTES

/** Upper bound on declared chains and positions alike; one byte of count each. */
export const MAX_DECLARED_ENTRIES = 255

/** Widest tokenId the codec will carry — a uint256, as minted by the V4 PositionManager. */
const MAX_TOKEN_ID_BYTES = 32

/**
 * The Permit2 sponsorship a bid's paymasterAndData carries when it was built on the real-bid path:
 * which paymaster it names and the fee-token permit it signed for that paymaster. Read for the
 * record only — the bid's authenticity comes from the solver signature over the userOpHash, which
 * covers these bytes, so nothing here is verified further and the permit signature is not kept.
 */
export interface PhantomBidSponsorship {
	paymaster: HexString
	/** The fee token the permit draws on. */
	token: HexString
	permitAmount: bigint
	/** Permit2 unordered nonce — random per op, which is what lets a solver's bids run in parallel. */
	nonce: bigint
	/** Unix seconds after which the permit is unusable. */
	deadline: bigint
}

/** What a phantom bid's paymasterAndData declares about the solver behind it. */
export interface PhantomBidDeclaration {
	/**
	 * Source chains the solver accepts payment from. Null when the bid carries no parseable
	 * declaration (the legacy default: the solver has not restricted its sources); an empty array
	 * is an explicit accepts-nothing. Callers must preserve that distinction.
	 */
	acceptedSources: string[] | null
	/**
	 * Uniswap V4 position tokenIds from a v2 declaration. Decoded for compatibility only; nothing
	 * prices them. Empty when none are declared — including for every v1 bid.
	 */
	uniswapV4Positions: bigint[]
}

// The chain ids in a declaration are UTF-8, encoded and decoded here by hand rather than through
// TextEncoder/TextDecoder (which is what @polkadot/util's stringToU8a/u8aToString wrap). The
// indexer runs this decoder inside SubQuery's vm2 sandbox, which exposes neither as a global and
// whose `util` fallback rejects a sandbox-created Uint8Array as "not an ArrayBufferView" — so every
// bid carrying a source-chain declaration threw on the first chain name and was dropped as
// "Failed to process bid", while bids declaring nothing, or only positions, sailed through. Plain
// arithmetic over the bytes has no realm to be on the wrong side of.

/** UTF-8 bytes of `text`, with no TextEncoder. */
function utf8Encode(text: string): number[] {
	const bytes: number[] = []
	for (const char of text) {
		const codePoint = char.codePointAt(0)!
		if (codePoint < 0x80) bytes.push(codePoint)
		else if (codePoint < 0x800) bytes.push(0xc0 | (codePoint >> 6), 0x80 | (codePoint & 0x3f))
		else if (codePoint < 0x10000) {
			bytes.push(0xe0 | (codePoint >> 12), 0x80 | ((codePoint >> 6) & 0x3f), 0x80 | (codePoint & 0x3f))
		} else {
			bytes.push(
				0xf0 | (codePoint >> 18),
				0x80 | ((codePoint >> 12) & 0x3f),
				0x80 | ((codePoint >> 6) & 0x3f),
				0x80 | (codePoint & 0x3f),
			)
		}
	}
	return bytes
}

/**
 * The string `bytes` encode in UTF-8, with no TextDecoder. Null for anything that is not
 * well-formed UTF-8 — a truncated sequence, a stray continuation byte, an overlong form, a
 * surrogate, or a code point past U+10FFFF — since a declaration naming an unreadable chain is
 * malformed as a whole.
 */
function utf8Decode(bytes: Uint8Array): string | null {
	let text = ""
	for (let offset = 0; offset < bytes.length; ) {
		const lead = bytes[offset]
		let codePoint: number
		let continuations: number
		if (lead < 0x80) {
			codePoint = lead
			continuations = 0
		} else if ((lead & 0xe0) === 0xc0) {
			codePoint = lead & 0x1f
			continuations = 1
		} else if ((lead & 0xf0) === 0xe0) {
			codePoint = lead & 0x0f
			continuations = 2
		} else if ((lead & 0xf8) === 0xf0) {
			codePoint = lead & 0x07
			continuations = 3
		} else {
			return null
		}
		if (offset + continuations >= bytes.length) return null
		for (let index = 1; index <= continuations; index++) {
			const byte = bytes[offset + index]
			if ((byte & 0xc0) !== 0x80) return null
			codePoint = (codePoint << 6) | (byte & 0x3f)
		}
		const overlong =
			(continuations === 1 && codePoint < 0x80) ||
			(continuations === 2 && codePoint < 0x800) ||
			(continuations === 3 && codePoint < 0x10000)
		if (overlong || codePoint > 0x10ffff || (codePoint >= 0xd800 && codePoint <= 0xdfff)) return null
		text += String.fromCodePoint(codePoint)
		offset += continuations + 1
	}
	return text
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
		const encoded = utf8Encode(chain)
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
 * Parses a declaration that starts at `start` and must run exactly to the end of `bytes`. Null for
 * anything unversioned, truncated or followed by trailing bytes — never a partial read.
 */
function parseDeclaration(bytes: Uint8Array, start: number): PhantomBidDeclaration | null {
	if (bytes.length - start < 2) return null

	const version = bytes[start]
	if (version !== DECLARATION_V1 && version !== DECLARATION_V2) return null

	const chains: string[] = []
	let offset = start + 2
	for (let entry = 0; entry < bytes[start + 1]; entry++) {
		if (offset >= bytes.length) return null
		const length = bytes[offset]
		offset += 1
		if (length === 0 || offset + length > bytes.length) return null
		const chain = utf8Decode(bytes.subarray(offset, offset + length))
		if (chain === null) return null
		chains.push(chain)
		offset += length
	}

	const positions: bigint[] = []
	if (version === DECLARATION_V2) {
		if (offset >= bytes.length) return null
		const count = bytes[offset]
		offset += 1
		for (let entry = 0; entry < count; entry++) {
			if (offset >= bytes.length) return null
			const length = bytes[offset]
			offset += 1
			if (length === 0 || length > MAX_TOKEN_ID_BYTES || offset + length > bytes.length) return null
			positions.push(bytesToBigInt(bytes.subarray(offset, offset + length)))
			offset += length
		}
	}

	// Trailing bytes mean this is not a declaration but something that happens to share the
	// version byte, so treat the whole blob as unparseable rather than half-reading it.
	if (offset !== bytes.length) return null

	return { acceptedSources: chains, uniswapV4Positions: positions }
}

function bytesToBigInt(bytes: Uint8Array): bigint {
	let value = 0n
	for (const byte of bytes) value = (value << 8n) | BigInt(byte)
	return value
}

function bytesToAddress(bytes: Uint8Array): HexString {
	return u8aToHex(bytes) as HexString
}

/** Whether `bytes` opens with a complete Permit2-sponsored paymasterAndData (see the layout note above). */
function hasPermit2Sponsorship(bytes: Uint8Array): boolean {
	return bytes.length >= PERMIT2_SPONSORSHIP_BYTES && bytes[PAYMASTER_DATA_OFFSET] === SIMPLEX_MODE_PERMIT2
}

/** Reads the sponsorship fields off a blob {@link hasPermit2Sponsorship} accepted. */
function parseSponsorship(bytes: Uint8Array): PhantomBidSponsorship {
	let offset = 0
	const take = (length: number) => {
		const slice = bytes.subarray(offset, offset + length)
		offset += length
		return slice
	}
	const paymaster = bytesToAddress(take(PAYMASTER_ADDRESS_BYTES))
	take(2 * PAYMASTER_GAS_LIMIT_BYTES)
	take(1) // mode, already checked
	const token = bytesToAddress(take(20))
	const permitAmount = bytesToBigInt(take(32))
	const nonce = bytesToBigInt(take(32))
	const deadline = bytesToBigInt(take(32))
	return { paymaster, token, permitAmount, nonce, deadline }
}

/** Everything a phantom bid's paymasterAndData was found to carry. */
export interface PhantomBidPaymasterAndData {
	/**
	 * Which shape the field was read as: a bare declaration, a Permit2-sponsored bid (with or
	 * without a declaration appended), or neither — empty, or bytes matching no known layout.
	 */
	mode: "declaration" | "permit2" | "none"
	/** Never null: an unreadable or missing declaration is the absent one (null sources, no positions). */
	declaration: PhantomBidDeclaration
	/** The paymaster fields of a sponsored bid; null for the other two modes. */
	sponsorship: PhantomBidSponsorship | null
}

/** A fresh absent declaration per call: consumers may extend the positions list, and must not share one. */
const absentDeclaration = (): PhantomBidDeclaration => ({ acceptedSources: null, uniswapV4Positions: [] })

/**
 * Decodes a phantom bid's paymasterAndData in whichever shape it takes.
 *
 * A bare declaration is tried first, so every bid placed before sponsored phantom bids existed
 * decodes to the byte exactly as it did. Failing that, a blob opening with a complete
 * Permit2-sponsored payload is read as a sponsored bid, and whatever follows the permit is parsed
 * as its declaration. Both parsers demand exact consumption — a bare declaration must end where
 * the bytes end, and a sponsored bid's tail must be a whole declaration or nothing — so neither
 * shape can be half-read as the other.
 *
 * A malformed tail on a sponsored bid yields the absent declaration but still reports the
 * sponsorship: the bid is still a bid, it just declared nothing readable.
 */
export function decodePhantomBidPaymasterAndData(
	paymasterAndData: string | undefined | null,
): PhantomBidPaymasterAndData {
	const none: PhantomBidPaymasterAndData = { mode: "none", declaration: absentDeclaration(), sponsorship: null }
	if (!paymasterAndData || !isHex(paymasterAndData)) return none
	const bytes = hexToU8a(paymasterAndData)

	const bare = parseDeclaration(bytes, 0)
	if (bare) return { mode: "declaration", declaration: bare, sponsorship: null }

	if (!hasPermit2Sponsorship(bytes)) return none
	const sponsorship = parseSponsorship(bytes)
	const tail = bytes.length > PERMIT2_SPONSORSHIP_BYTES ? parseDeclaration(bytes, PERMIT2_SPONSORSHIP_BYTES) : null
	return { mode: "permit2", declaration: tail ?? absentDeclaration(), sponsorship }
}

/**
 * Decodes the declaration out of a phantom bid's paymasterAndData, whichever shape it takes (see
 * {@link decodePhantomBidPaymasterAndData}). Understands both declaration versions, so bids placed
 * before positions existed keep decoding unchanged. Anything absent, unversioned or malformed
 * yields a null `acceptedSources` with no positions — never a partial read.
 */
export function decodePhantomBidDeclaration(paymasterAndData: string | undefined | null): PhantomBidDeclaration {
	return decodePhantomBidPaymasterAndData(paymasterAndData).declaration
}

/**
 * Builds a phantom bid's paymasterAndData: the declaration alone, or appended to a Permit2
 * sponsorship when the bid was built on the real-bid path and carries one.
 *
 * `sponsorship` is the packed EntryPoint v0.8 paymasterAndData the paymaster builder produced. It
 * must be exactly the Permit2-mode layout — the only one the decoder recognises — so a caller
 * cannot sign bytes a decoder would read as "no declaration"; anything else throws. Empty
 * or omitted, the result is the bare declaration, byte-identical to what unsponsored bids carry.
 */
export function encodePhantomBidPaymasterAndData(bid: {
	sponsorship?: HexString | null
	acceptedSourceChains?: string[]
	uniswapV4Positions?: bigint[]
}): HexString {
	const declaration = encodePhantomBidDeclaration(bid)
	const sponsorship = bid.sponsorship
	if (!sponsorship || sponsorship === "0x") return declaration

	if (!isHex(sponsorship)) throw new Error("Phantom bid sponsorship is not hex")
	const bytes = hexToU8a(sponsorship)
	if (bytes.length !== PERMIT2_SPONSORSHIP_BYTES || !hasPermit2Sponsorship(bytes)) {
		throw new Error(
			`Phantom bid sponsorship must be a ${PERMIT2_SPONSORSHIP_BYTES}-byte Permit2-mode paymasterAndData, got ${bytes.length} bytes`,
		)
	}
	return `${sponsorship}${declaration.slice(2)}` as HexString
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
