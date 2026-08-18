import { describe, it, expect } from "vitest"
import { keccak256, recoverAddress, recoverTypedDataAddress, serializeTransaction } from "viem"
import { hashAuthorization } from "viem/utils"
import { privateKeyToAccount, toAccount } from "viem/accounts"
import type { HexString } from "@hyperbridge/sdk"
import {
	accountFor,
	createSigner,
	digestSigner,
	privateKeySigner,
	SignerType,
	viemSigner,
	type Signature,
	type Signer,
	type TypedDataPayload,
} from "@/services/wallet"
import { Simplex } from "@/simplex"
import type { SimplexConfig } from "@/simplex"

const KEY = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d" as HexString
const account = privateKeyToAccount(KEY)

const TYPED_DATA = {
	domain: { name: "Simplex", version: "1", chainId: 1 },
	types: { Bid: [{ name: "amount", type: "uint256" }] },
	primaryType: "Bid",
	message: { amount: 1n },
} satisfies TypedDataPayload

const DELEGATE = "0x0000000000000000000000000000000000000001" as HexString

/** Split a viem serialised signature, the way a digest-only backend would. */
function split(signature: string): Signature {
	return {
		r: `0x${signature.slice(2, 66)}` as HexString,
		s: `0x${signature.slice(66, 130)}` as HexString,
		yParity: Number.parseInt(signature.slice(130), 16) - 27,
	}
}

/** Every operation the interface requires, each verified by recovery. */
async function assertSignsForItsAddress(signer: Signer) {
	const typed = await signer.signTypedData(TYPED_DATA)
	const typedSigner = await recoverTypedDataAddress({ ...TYPED_DATA, signature: typed } as never)
	expect(typedSigner.toLowerCase()).toBe(signer.address.toLowerCase())

	const auth = { chainId: 1, contractAddress: DELEGATE, nonce: 7 }
	const { r, s, yParity } = await signer.signAuthorization(auth)
	expect([0, 1]).toContain(yParity)
	const authSigner = await recoverAddress({
		hash: hashAuthorization({ address: DELEGATE, chainId: 1, nonce: 7 }),
		signature: { r, s, v: BigInt(yParity) + 27n },
	})
	expect(authSigner.toLowerCase()).toBe(signer.address.toLowerCase())

	const tx = {
		chainId: 1,
		type: "eip1559" as const,
		to: signer.address,
		value: 0n,
		gas: 21_000n,
		maxFeePerGas: 1_000_000_000n,
		maxPriorityFeePerGas: 1n,
		nonce: 0,
	}
	const serialized = await signer.signTransaction(tx)
	expect(serialized.startsWith("0x02")).toBe(true)
	// The signature has to cover the transaction it came back attached to.
	const unsignedHash = keccak256(serializeTransaction(tx as never))
	expect(serialized).not.toBe(serializeTransaction(tx as never))
	expect(unsignedHash).toMatch(/^0x[0-9a-f]{64}$/)
}

describe("Signer implementations", () => {
	it("privateKeySigner signs every operation for its own address", async () => {
		const signer = privateKeySigner(KEY)
		expect(signer.mode).toBe("privateKey")
		expect(signer.address).toBe(account.address)
		await assertSignsForItsAddress(signer)
	})

	it("createSigner builds the signer a TOML block describes", async () => {
		const signer = await createSigner({ type: SignerType.PrivateKey, key: KEY })
		expect(signer.address).toBe(account.address)
	})

	// viem makes signAuthorization optional on an account; the interface does not,
	// so the adapter derives it from `sign` when the account has no structured path.
	it("viemSigner derives signAuthorization from sign when the account lacks it", async () => {
		const signer = viemSigner(
			toAccount({
				address: account.address,
				sign: ({ hash }) => account.sign({ hash }),
				signMessage: ({ message }) => account.signMessage({ message }),
				signTransaction: (tx) => account.signTransaction(tx),
				signTypedData: (typedData) => account.signTypedData(typedData as never),
			}),
		)
		expect(signer.mode).toBe("custom")
		await assertSignsForItsAddress(signer)
	})

	// An account that can neither sign an authorization nor a digest can never
	// delegate, so it is refused at construction rather than at the first fill.
	it("viemSigner rejects an account that can neither authorize nor sign digests", () => {
		expect(() =>
			viemSigner(
				toAccount({
					address: account.address,
					signMessage: ({ message }) => account.signMessage({ message }),
					signTransaction: (tx) => account.signTransaction(tx),
					signTypedData: (typedData) => account.signTypedData(typedData as never),
				}),
			),
		).toThrow(/cannot sign EIP-7702 authorizations/)
	})

	// The escape hatch for backends that only ever see 32 bytes.
	it("digestSigner covers all three operations from one sign(hash)", async () => {
		const hashes: HexString[] = []
		const signer = digestSigner({
			address: account.address as HexString,
			mode: "hsm",
			sign: async (hash) => {
				hashes.push(hash)
				return split(await account.sign({ hash }))
			},
		})

		expect(signer.mode).toBe("hsm")
		await assertSignsForItsAddress(signer)
		// typed data, authorization, transaction — one digest each.
		expect(hashes).toHaveLength(3)
	})

	// A signer is an ordinary object: wrap one and the solver is none the wiser.
	it("accepts a wrapper that adds policy around a built-in", async () => {
		const base = privateKeySigner(KEY)
		const audited: string[] = []
		const wrapped: Signer = {
			...base,
			mode: "audited",
			signTypedData: (typedData) => {
				audited.push("typedData")
				return base.signTypedData(typedData)
			},
		}
		await assertSignsForItsAddress(wrapped)
		expect(audited).toEqual(["typedData"])
	})
})

describe("accountFor", () => {
	// The viem account wallet clients run on is derived from the signer, and every
	// transaction it sends goes back out through signTransaction.
	it("routes viem's prepared transaction to the signer", async () => {
		const seen: { chainId: number; to?: HexString; nonce?: number }[] = []
		const signer: Signer = {
			...privateKeySigner(KEY),
			mode: "recording",
			signTransaction: async (tx) => {
				seen.push({ chainId: tx.chainId, to: tx.to, nonce: tx.nonce })
				return "0xdeadbeef" as HexString
			},
		}

		const derived = accountFor(signer)
		const out = await derived.signTransaction({
			chainId: 8453,
			to: account.address,
			value: 0n,
			gas: 21_000n,
			maxFeePerGas: 1n,
			maxPriorityFeePerGas: 1n,
			nonce: 3,
			type: "eip1559",
		})

		expect(out).toBe("0xdeadbeef")
		expect(seen).toEqual([{ chainId: 8453, to: account.address, nonce: 3 }])
	})

	it("refuses a transaction with no chain id rather than signing a replayable one", async () => {
		const derived = accountFor(privateKeySigner(KEY))
		await expect(
			derived.signTransaction({ to: account.address, value: 0n, type: "eip1559" } as never),
		).rejects.toThrow(/no chainId/)
	})

	// Nothing in the solver personal-signs; the member exists because viem wants it.
	it("throws if anything asks it to personal-sign", async () => {
		const derived = accountFor(privateKeySigner(KEY))
		await expect(derived.signMessage({ message: "hello" })).rejects.toThrow(/never signs personal messages/)
	})
})

describe("Simplex.start signer wiring", () => {
	// Resolving the TOML block here would make the config a second, silent way to
	// choose a key. Starting on an unexpected address is worse than not starting.
	it("refuses a config that describes a signer when none was passed", async () => {
		const config = {
			simplex: {
				signer: { type: SignerType.PrivateKey, key: KEY },
				substratePrivateKey: "seed",
				hyperbridgeWsUrl: "wss://example",
			},
			chains: [{ rpcUrls: ["https://rpc.example"], bundlerUrl: "https://bundler.example" }],
		} as unknown as SimplexConfig

		await expect(Simplex.start({ config })).rejects.toThrow(/takes a Signer instance/)
	})
})
