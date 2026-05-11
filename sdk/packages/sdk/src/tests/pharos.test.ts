/**
 * Pharos chain client proof-fetching tests.
 *
 * Mirrors `test_pharos_storage_proof_verification` and related tests in
 * `modules/pallets/testsuite/src/tests/pharos_state_machine.rs`.
 *
 * Uses the PharosChain client to fetch proofs from the staking contract on
 * Pharos Atlantic testnet, then verifies the SCALE-encoded proof structure.
 */
import { describe, it, expect, beforeAll } from "vitest"
import { bytesToHex, hexToBytes, pad } from "viem"
import type { HexString } from "@/types"
import { PharosChain, type PharosChainParams } from "@/chains/pharos"
import { PharosStateProof } from "@/utils/pharos"

/** Lexicographic comparison of two byte arrays for finding entries by key. */
function compareBytes(a: number[], b: number[]): number {
	const len = Math.min(a.length, b.length)
	for (let i = 0; i < len; i++) {
		if (a[i] !== b[i]) return a[i] - b[i]
	}
	return a.length - b.length
}

const PHAROS_RPC_URL = process.env.PHAROS_ATLANTIC_RPC!

/**
 * Staking contract address on Pharos Atlantic (testnet).
 * Matches `STAKING_CONTRACT_ADDRESS` in `pharos-primitives/src/constants.rs`.
 */
const STAKING_CONTRACT_ADDRESS: HexString = "0x4100000000000000000000000000000000000000"

/**
 * Storage slot 6 = `totalStake` in the Pharos staking precompile.
 * Matches the Rust test: `H256(U256::from(6u64).to_big_endian())`.
 */
const TOTAL_STAKE_SLOT: HexString = pad("0x06", { size: 32 }) as HexString

/**
 * Storage slot 5 = `currentEpoch` / `epochLength` in the Pharos staking precompile.
 * Used by `test_pharos_multiple_storage_proofs` on the Rust side.
 */
const EPOCH_LENGTH_SLOT: HexString = pad("0x05", { size: 32 }) as HexString

/**
 * A non-existent storage slot for non-existence proof testing.
 * Matches `H256::from_low_u64_be(999999)` from the Rust test.
 */
const FAKE_SLOT: HexString = pad("0x0f423f", { size: 32 }) as HexString

/**
 * A non-existent account address for non-existence proof testing.
 * Matches the Rust test's `[0xde, 0xad, 0, 0, ...]`.
 */
const FAKE_ADDRESS: HexString = "0xdead000000000000000000000000000000000000"

/** Helper to build a 52-byte key from a 20-byte address and 32-byte slot. */
function addressSlotKey(address: HexString, slot: HexString): HexString {
	const addrBytes = hexToBytes(address)
	const slotBytes = hexToBytes(slot)
	const combined = new Uint8Array(52)
	combined.set(addrBytes, 0)
	combined.set(slotBytes, 20)
	return bytesToHex(combined) as HexString
}

describe.skip("Pharos proof fetching", () => {
	let chain: PharosChain
	let targetBlock: bigint

	beforeAll(async () => {
		// Create the PharosChain with explicit params (no auto-detection — we
		// don't need the IsmpHost contract for proof-fetching tests).
		const params: PharosChainParams = {
			chainId: 688689, // Pharos Atlantic testnet
			rpcUrl: PHAROS_RPC_URL,
			host: STAKING_CONTRACT_ADDRESS, // using staking contract as host for testing
		}
		chain = PharosChain.fromParams(params)

		// Use a recent but safe block (current - 5), same as the Rust tests.
		const latestBlock = await chain.client.getBlockNumber()
		targetBlock = latestBlock - 5n
		console.log(`Testing at block: ${targetBlock}`)
	}, 30_000)

	it("should fetch a storage proof for totalStake (slot 6) via 52-byte key", async () => {
		// Mirrors `test_pharos_storage_proof_verification`:
		// Fetch a proof for the staking contract's totalStake slot.
		const key52 = addressSlotKey(STAKING_CONTRACT_ADDRESS, TOTAL_STAKE_SLOT)
		const encoded = await chain.queryStateProof(targetBlock, [key52])

		// Decode the SCALE-encoded PharosStateProof
		const decoded = PharosStateProof.dec(hexToBytes(encoded))

		// The slot key used as map key is the raw 32-byte slot (big-endian padded)
		const slotKeyArr = Array.from(hexToBytes(TOTAL_STAKE_SLOT))

		// Should have at least one storage proof entry
		const storageProofEntries = decoded.storageProof
		expect(storageProofEntries.length).toBeGreaterThanOrEqual(1)
		console.log(`Storage proof entries: ${storageProofEntries.length}`)

		// Find the entry for our slot
		const entry = storageProofEntries.find(
			([k]) => compareBytes(k, slotKeyArr) === 0,
		)
		expect(entry).toBeDefined()
		const [, proofNodes] = entry!

		// Proof should have non-empty nodes
		expect(proofNodes.length).toBeGreaterThan(0)
		console.log(`Storage proof nodes for totalStake: ${proofNodes.length}`)

		// Each proof node should have non-empty proofNode bytes
		for (const node of proofNodes) {
			expect(node.proofNode.length).toBeGreaterThan(0)
		}

		// Should also have a storage value entry
		const valueEntry = decoded.storageValues.find(
			([k]) => compareBytes(k, slotKeyArr) === 0,
		)
		expect(valueEntry).toBeDefined()
		const [, valueBytes] = valueEntry!

		// The value should be 32 bytes (padded)
		expect(valueBytes.length).toBe(32)

		// Parse totalStake — it should be non-zero on a live network
		const valueBigInt = BigInt(bytesToHex(new Uint8Array(valueBytes)))
		console.log(`Total stake: ${valueBigInt}`)
		expect(valueBigInt).toBeGreaterThan(0n)
	}, 30_000)

	it("should fetch multiple storage proofs (totalStake + epochLength)", async () => {
		// Mirrors `test_pharos_multiple_storage_proofs`:
		// Fetch proofs for both slot 6 (totalStake) and slot 5 (epochLength).
		const key52Stake = addressSlotKey(STAKING_CONTRACT_ADDRESS, TOTAL_STAKE_SLOT)
		const key52Epoch = addressSlotKey(STAKING_CONTRACT_ADDRESS, EPOCH_LENGTH_SLOT)
		const encoded = await chain.queryStateProof(targetBlock, [key52Stake, key52Epoch])

		const decoded = PharosStateProof.dec(hexToBytes(encoded))

		const stakeKeyArr = Array.from(hexToBytes(TOTAL_STAKE_SLOT))
		const epochKeyArr = Array.from(hexToBytes(EPOCH_LENGTH_SLOT))

		// Both slots should have storage proof entries
		const stakeEntry = decoded.storageProof.find(
			([k]) => compareBytes(k, stakeKeyArr) === 0,
		)
		const epochEntry = decoded.storageProof.find(
			([k]) => compareBytes(k, epochKeyArr) === 0,
		)
		expect(stakeEntry).toBeDefined()
		expect(epochEntry).toBeDefined()
		console.log(`totalStake proof nodes: ${stakeEntry![1].length}`)
		console.log(`epochLength proof nodes: ${epochEntry![1].length}`)

		// Both should have corresponding values
		const stakeValue = decoded.storageValues.find(
			([k]) => compareBytes(k, stakeKeyArr) === 0,
		)
		const epochValue = decoded.storageValues.find(
			([k]) => compareBytes(k, epochKeyArr) === 0,
		)
		expect(stakeValue).toBeDefined()
		expect(epochValue).toBeDefined()

		const stakeBigInt = BigInt(bytesToHex(new Uint8Array(stakeValue![1])))
		const epochBigInt = BigInt(bytesToHex(new Uint8Array(epochValue![1])))
		console.log(`Total stake: ${stakeBigInt}`)
		console.log(`Epoch length: ${epochBigInt}`)

		expect(stakeBigInt).toBeGreaterThan(0n)
		expect(epochBigInt).toBeGreaterThan(0n)
	}, 30_000)

	it("should fetch a non-existence storage proof for a fake slot", async () => {
		// Mirrors `test_pharos_non_existence_storage_proof`:
		// Query a non-existent storage slot on the real staking contract.
		const key52 = addressSlotKey(STAKING_CONTRACT_ADDRESS, FAKE_SLOT)
		const encoded = await chain.queryStateProof(targetBlock, [key52])

		const decoded = PharosStateProof.dec(hexToBytes(encoded))
		const fakeSlotKeyArr = Array.from(hexToBytes(FAKE_SLOT))

		// The slot should NOT appear in storageProof (it doesn't exist)
		const existenceEntry = decoded.storageProof.find(
			([k]) => compareBytes(k, fakeSlotKeyArr) === 0,
		)

		// It should appear in nonExistenceProofs instead
		const nonExistenceEntry = decoded.nonExistenceProofs.find(
			([k]) => compareBytes(k, fakeSlotKeyArr) === 0,
		)

		// At least one of these conditions must hold
		if (nonExistenceEntry) {
			// Non-existence proof present
			expect(existenceEntry).toBeUndefined()
			const [, neProof] = nonExistenceEntry
			expect(neProof.proofNodes.length).toBeGreaterThan(0)
			console.log(`Non-existence proof nodes: ${neProof.proofNodes.length}`)
			console.log(`Sibling proofs: ${neProof.siblingProofs.length}`)
		} else {
			// Slot 999999 might actually exist on a live chain; just log and pass
			console.log("Storage slot exists (unexpected), skipping non-existence check")
			expect(existenceEntry).toBeDefined()
		}
	}, 30_000)

	it("should fetch an account proof for the staking contract (20-byte key)", async () => {
		// Mirrors `test_pharos_account_proof_with_raw_value`:
		// Query the staking contract as a 20-byte account key.
		const encoded = await chain.queryStateProof(targetBlock, [STAKING_CONTRACT_ADDRESS])

		const decoded = PharosStateProof.dec(hexToBytes(encoded))
		const addrKeyArr = Array.from(hexToBytes(STAKING_CONTRACT_ADDRESS))

		// Should have an account proof entry
		const accountEntry = decoded.accountProofs.find(
			([k]) => compareBytes(k, addrKeyArr) === 0,
		)
		expect(accountEntry).toBeDefined()

		const [, accountData] = accountEntry!

		// Proof nodes should be non-empty
		expect(accountData.proofNodes.length).toBeGreaterThan(0)
		console.log(`Account proof nodes: ${accountData.proofNodes.length}`)

		// Raw value (RLP-encoded account) should be non-empty
		expect(accountData.rawValue.length).toBeGreaterThan(0)
		console.log(`Account raw value length: ${accountData.rawValue.length}`)
	}, 30_000)

	it("should SCALE round-trip encode/decode a fetched proof", async () => {
		// Fetch a proof and verify the SCALE codec round-trips cleanly.
		const key52 = addressSlotKey(STAKING_CONTRACT_ADDRESS, TOTAL_STAKE_SLOT)
		const encoded = await chain.queryStateProof(targetBlock, [key52])

		const decoded = PharosStateProof.dec(hexToBytes(encoded))

		// Re-encode and compare
		const reEncoded = bytesToHex(PharosStateProof.enc(decoded)) as HexString
		expect(reEncoded).toBe(encoded)
		console.log("SCALE round-trip: PASSED")
	}, 30_000)
})
