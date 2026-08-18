import { describe, it, expect } from "vitest"
import { keccak256, recoverAddress, recoverTypedDataAddress, toHex } from "viem"
import { privateKeyToAccount, toAccount } from "viem/accounts"
import type { HexString } from "@hyperbridge/sdk"
import {
	accountFor,
	createSigner,
	privateKeySigner,
	SignerType,
	viemSigner,
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

/** Both primitives every backend has to provide, checked by recovery. */
async function assertSignsForItsAddress(signer: Signer) {
	const hash = keccak256(toHex("simplex")) as HexString

	const { r, s, yParity } = await signer.signRawHash(hash)
	expect([0, 1]).toContain(yParity)
	const recovered = await recoverAddress({ hash, signature: { r, s, v: BigInt(yParity) + 27n } })
	expect(recovered.toLowerCase()).toBe(signer.address.toLowerCase())

	const typed = await signer.signTypedData(TYPED_DATA)
	const typedSigner = await recoverTypedDataAddress({ ...TYPED_DATA, signature: typed } as never)
	expect(typedSigner.toLowerCase()).toBe(signer.address.toLowerCase())
}

describe("Signer implementations", () => {
	it("privateKeySigner signs for its own address", async () => {
		const signer = privateKeySigner(KEY)
		expect(signer.mode).toBe("privateKey")
		expect(signer.address).toBe(account.address)
		await assertSignsForItsAddress(signer)
	})

	// The adapter for custody that already speaks viem: account in, Signer out.
	it("viemSigner derives the interface from any local account", async () => {
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

	// EIP-7702 delegation is signed with `account.sign`, and a solver that cannot
	// delegate cannot bid — so the gap is refused at construction, not at the fill.
	it("viemSigner rejects an account that cannot sign raw hashes", () => {
		expect(() =>
			viemSigner(
				toAccount({
					address: account.address,
					signMessage: ({ message }) => account.signMessage({ message }),
					signTransaction: (tx) => account.signTransaction(tx),
					signTypedData: (typedData) => account.signTypedData(typedData as never),
				}),
			),
		).toThrow(/cannot sign raw hashes/)
	})

	it("createSigner builds the signer a TOML block describes", async () => {
		const signer = await createSigner({ type: SignerType.PrivateKey, key: KEY })
		expect(signer.address).toBe(account.address)
	})

	// A custom signer is an ordinary object: no viem types, two methods.
	it("accepts a hand-written signer with no viem account behind it", async () => {
		const calls: string[] = []
		const custom: Signer = {
			address: account.address as HexString,
			mode: "audited",
			signTypedData: async (typedData) => {
				calls.push("typedData")
				return account.signTypedData(typedData as never) as Promise<HexString>
			},
			signRawHash: async (hash) => {
				calls.push("rawHash")
				const sig = await account.sign({ hash })
				return {
					r: `0x${sig.slice(2, 66)}` as HexString,
					s: `0x${sig.slice(66, 130)}` as HexString,
					yParity: Number.parseInt(sig.slice(130), 16) - 27,
				}
			},
		}
		await assertSignsForItsAddress(custom)
		expect(calls).toEqual(["rawHash", "typedData"])
	})
})

describe("accountFor", () => {
	// The viem account the wallet clients run on is derived, so a signer that
	// implements nothing but the two required methods can still send transactions.
	it("serialises and digest-signs transactions for a signer without signTransaction", async () => {
		const digests: HexString[] = []
		const signer: Signer = {
			address: account.address as HexString,
			signTypedData: (td) => account.signTypedData(td as never) as Promise<HexString>,
			signRawHash: async (hash) => {
				digests.push(hash)
				const sig = await account.sign({ hash })
				return {
					r: `0x${sig.slice(2, 66)}` as HexString,
					s: `0x${sig.slice(66, 130)}` as HexString,
					yParity: Number.parseInt(sig.slice(130), 16) - 27,
				}
			},
		}

		const derived = accountFor(signer)
		const serialized = await derived.signTransaction({
			chainId: 1,
			to: account.address,
			value: 0n,
			gas: 21_000n,
			maxFeePerGas: 1_000_000_000n,
			maxPriorityFeePerGas: 1n,
			nonce: 0,
			type: "eip1559",
		})

		expect(serialized.startsWith("0x02")).toBe(true)
		expect(digests).toHaveLength(1)
	})

	// A backend that takes transactions gets the transaction, not a hash.
	it("hands the transaction to a signer that implements signTransaction", async () => {
		const seen: { chainId: number; to?: HexString }[] = []
		const signer: Signer = {
			address: account.address as HexString,
			signTypedData: (td) => account.signTypedData(td as never) as Promise<HexString>,
			signRawHash: async () => ({ r: "0x00" as HexString, s: "0x00" as HexString, yParity: 0 }),
			signTransaction: async (tx) => {
				seen.push({ chainId: tx.chainId, to: tx.to })
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
		expect(seen).toEqual([{ chainId: 8453, to: account.address }])
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
