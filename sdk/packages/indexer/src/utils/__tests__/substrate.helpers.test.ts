import { decodeRelayerAddress } from "../substrate.helpers"
import { encodeAddress } from "@polkadot/util-crypto"

describe("decodeRelayerAddress", () => {
	describe("raw 32-byte public keys", () => {
		it("should decode a raw 32-byte public key to SS58 address", () => {
			const rawPublicKey = "0x588177cf191e890d83b10e9653cfd663c8a382d5afce8518636dd9565c22a631"
			const result = decodeRelayerAddress(rawPublicKey)

			expect(result).toBe(encodeAddress(rawPublicKey))
		})
	})

	describe("SCALE-encoded Signature enum", () => {
		it("should decode EVM variant (index 0) with 20-byte address", () => {
			// Construct a SCALE-encoded EVM signature:
			// - Variant index: 0 (Evm)
			// - Compact length for address: 0x50 (20 in single-byte compact: 20 << 2 = 80 = 0x50)
			// - 20 bytes of address
			// - Compact length for signature + signature bytes (we'll add minimal data)

			const evmAddressHex = "d19651565559E361e984640ce5F69F1575149E63"
			const signatureHex = "00".repeat(65) // 65-byte signature placeholder

			// Build SCALE-encoded Signature::Evm { address, signature }
			// variant(1) + compact_len(1) + address(20) + compact_len(2) + signature(65)
			const variantByte = "00" // Evm variant
			const addressCompactLen = "50" // 20 << 2 = 80 = 0x50
			const sigCompactLen = "0501" // 65 in two-byte compact: ((65 << 2) | 1) = 261 = 0x0105, little endian = 0501

			const encoded = `0x${variantByte}${addressCompactLen}${evmAddressHex}${sigCompactLen}${signatureHex}`

			const result = decodeRelayerAddress(encoded)

			// Should extract the 20-byte EVM address and return as hex
			expect(result.toLowerCase()).toBe(`0x${evmAddressHex}`.toLowerCase())
		})

		it("should decode Sr25519 variant (index 1) with 32-byte public key", () => {
			const publicKeyHex = "588177cf191e890d83b10e9653cfd663c8a382d5afce8518636dd9565c22a631"
			const signatureHex = "00".repeat(64) // 64-byte signature placeholder

			// Build SCALE-encoded Signature::Sr25519 { public_key, signature }
			const variantByte = "01" // Sr25519 variant
			const pubKeyCompactLen = "80" // 32 << 2 = 128 = 0x80
			const sigCompactLen = "0101" // 64 in two-byte compact: ((64 << 2) | 1) = 257 = 0x0101, little endian = 0101

			const encoded = `0x${variantByte}${pubKeyCompactLen}${publicKeyHex}${sigCompactLen}${signatureHex}`

			const result = decodeRelayerAddress(encoded)

			// Should extract the 32-byte public key and return as SS58
			expect(result).toBe(encodeAddress(`0x${publicKeyHex}`))
		})

		it("should decode Ed25519 variant (index 2) with 32-byte public key", () => {
			const publicKeyHex = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
			const signatureHex = "00".repeat(64) // 64-byte signature placeholder

			// Build SCALE-encoded Signature::Ed25519 { public_key, signature }
			const variantByte = "02" // Ed25519 variant
			const pubKeyCompactLen = "80" // 32 << 2 = 128 = 0x80
			const sigCompactLen = "0101" // 64 in two-byte compact

			const encoded = `0x${variantByte}${pubKeyCompactLen}${publicKeyHex}${sigCompactLen}${signatureHex}`

			const result = decodeRelayerAddress(encoded)

			// Should extract the 32-byte public key and return as SS58
			expect(result).toBe(encodeAddress(`0x${publicKeyHex}`))
		})
	})

	describe("test signature decoding", () => {
		it("should handle signature decoding", () => {

			const signature =
				"0x01805254bab90041a14167817f565825aaefd6067695b343ec57a0263be9986b6b5c0101d8609bd3642cfa496af519019c3d5f5da538d016066e40ae9ffa0e04cb36e75d0b7e67a95f66a6dbd346b4c9e5fdcfb3b0bacc184c862b024e5e49fcc3a72d87"
			const publicKeyFromSignatureData = "5254bab90041a14167817f565825aaefd6067695b343ec57a0263be9986b6b5c"

			const result = decodeRelayerAddress(signature)

			// Should decode to SS58 address from the public key
			expect(result).toBe(encodeAddress(`0x${publicKeyFromSignatureData}`))
		})
	})
})
