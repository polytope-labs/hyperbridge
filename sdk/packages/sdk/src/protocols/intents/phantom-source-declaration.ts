// Accepted-source-chains declaration carried in a phantom bid's paymasterAndData.
//
// A same-chain phantom bid proves a solver operates on a chain, not which chains it will accept
// payment FROM when filling a cross-chain order. Bids declare that set here because
// paymasterAndData is covered by the userOpHash, so the declaration is authenticated by the
// solver's existing bid signature (the `signature` field is excluded from the hash and therefore
// unusable). The overload applies to phantom bids only: a real fill's paymasterAndData keeps its
// functional EntryPoint semantics, and nothing on the real-fill path ever parses this format.
//
// Layout: version(1) ‖ count(1) ‖ count × (length(1) ‖ utf8 state machine id). A declaration
// with no entries is a deliberate "accepts no source chains" and is distinct from an absent
// declaration ("0x"), which means the legacy default: all CCTP/USDT0-covered chains.

export type HexString = `0x${string}`

const DECLARATION_VERSION = 0x01

/** Upper bound on declared chains; one byte of count, and far beyond any real deployment. */
const MAX_DECLARED_CHAINS = 255

/**
 * Encodes the accepted source chains (state machine ids, e.g. "EVM-8453") into the
 * paymasterAndData declaration blob.
 */
export function encodeAcceptedSourceChains(chains: string[]): HexString {
	if (chains.length > MAX_DECLARED_CHAINS) {
		throw new Error(`Cannot declare more than ${MAX_DECLARED_CHAINS} source chains`)
	}

	const bytes: number[] = [DECLARATION_VERSION, chains.length]
	for (const chain of chains) {
		const encoded = utf8Encode(chain)
		if (encoded.length === 0 || encoded.length > 255) {
			throw new Error(`Invalid state machine id in source chain declaration: ${chain}`)
		}
		bytes.push(encoded.length, ...encoded)
	}
	return `0x${bytes.map((b) => b.toString(16).padStart(2, "0")).join("")}` as HexString
}

/**
 * Decodes a phantom bid's paymasterAndData into its declared source chains. Returns null for an
 * absent, unversioned or malformed blob — the legacy default — and an empty array only for an
 * explicit zero-entry declaration. Callers must preserve that distinction.
 */
export function decodeAcceptedSourceChains(paymasterAndData: string | undefined | null): string[] | null {
	if (!paymasterAndData) return null
	const hex = paymasterAndData.toLowerCase().replace(/^0x/, "")
	if (hex.length < 4 || hex.length % 2 !== 0 || !/^[0-9a-f]*$/.test(hex)) return null

	const bytes: number[] = []
	for (let i = 0; i < hex.length; i += 2) bytes.push(parseInt(hex.slice(i, i + 2), 16))
	if (bytes[0] !== DECLARATION_VERSION) return null

	const count = bytes[1]
	const chains: string[] = []
	let offset = 2
	for (let entry = 0; entry < count; entry++) {
		if (offset >= bytes.length) return null
		const length = bytes[offset]
		offset += 1
		if (length === 0 || offset + length > bytes.length) return null
		chains.push(utf8Decode(bytes.slice(offset, offset + length)))
		offset += length
	}
	// Trailing bytes mean this is not a declaration but something that happens to share the
	// version byte, so treat the whole blob as unparseable rather than half-reading it.
	if (offset !== bytes.length) return null

	return chains
}

// State machine ids are ASCII in practice, but encode/decode proper UTF-8 anyway so the codec
// never silently corrupts an exotic id. TextEncoder exists in Node and browsers but not in the
// SubQuery VM2 sandbox, hence the manual fallback.
function utf8Encode(value: string): number[] {
	if (typeof TextEncoder !== "undefined") return Array.from(new TextEncoder().encode(value))
	const bytes: number[] = []
	for (const char of value) {
		const code = char.codePointAt(0)!
		if (code <= 0x7f) bytes.push(code)
		else if (code <= 0x7ff) bytes.push(0xc0 | (code >> 6), 0x80 | (code & 0x3f))
		else if (code <= 0xffff) bytes.push(0xe0 | (code >> 12), 0x80 | ((code >> 6) & 0x3f), 0x80 | (code & 0x3f))
		else
			bytes.push(
				0xf0 | (code >> 18),
				0x80 | ((code >> 12) & 0x3f),
				0x80 | ((code >> 6) & 0x3f),
				0x80 | (code & 0x3f),
			)
	}
	return bytes
}

function utf8Decode(bytes: number[]): string {
	if (typeof TextDecoder !== "undefined") return new TextDecoder().decode(new Uint8Array(bytes))
	let out = ""
	for (let i = 0; i < bytes.length; ) {
		const byte = bytes[i]
		let code: number
		if (byte < 0x80) {
			code = byte
			i += 1
		} else if (byte < 0xe0) {
			code = ((byte & 0x1f) << 6) | (bytes[i + 1] & 0x3f)
			i += 2
		} else if (byte < 0xf0) {
			code = ((byte & 0x0f) << 12) | ((bytes[i + 1] & 0x3f) << 6) | (bytes[i + 2] & 0x3f)
			i += 3
		} else {
			code =
				((byte & 0x07) << 18) | ((bytes[i + 1] & 0x3f) << 12) | ((bytes[i + 2] & 0x3f) << 6) | (bytes[i + 3] & 0x3f)
			i += 4
		}
		out += String.fromCodePoint(code)
	}
	return out
}
