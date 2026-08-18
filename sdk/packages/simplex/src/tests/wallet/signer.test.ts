import { describe, it, expect } from "vitest"
import { keccak256, recoverAddress, toHex } from "viem"
import { privateKeyToAccount, toAccount } from "viem/accounts"
import type { HexString } from "@hyperbridge/sdk"
import { createSigner, privateKeySigner, SignerType, viemSigner, type Signer } from "@/services/wallet"
import { Simplex } from "@/simplex"
import type { SimplexConfig } from "@/simplex"

const KEY = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d" as HexString
const account = privateKeyToAccount(KEY)

/** Both primitives every backend has to provide, checked by recovery. */
async function assertSignsForItsAddress(signer: Signer) {
	const hash = keccak256(toHex("simplex")) as HexString

	const { r, s, yParity } = await signer.signRawHash(hash)
	expect([0, 1]).toContain(yParity)
	const recovered = await recoverAddress({ hash, signature: { r, s, v: BigInt(yParity) + 27n } })
	expect(recovered.toLowerCase()).toBe(signer.account.address.toLowerCase())

	const typed = await signer.signTypedData({
		domain: { name: "Simplex", version: "1", chainId: 1 },
		types: { Bid: [{ name: "amount", type: "uint256" }] },
		primaryType: "Bid",
		message: { amount: 1n },
	})
	expect(typed).toMatch(/^0x[0-9a-f]{130}$/)
}

describe("Signer implementations", () => {
	it("privateKeySigner signs for its own address", async () => {
		const signer = privateKeySigner(KEY)
		expect(signer.mode).toBe("privateKey")
		expect(signer.account.address).toBe(account.address)
		await assertSignsForItsAddress(signer)
	})

	// The escape hatch every custom backend goes through: viem account in, the
	// whole Signer interface out.
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
		expect(signer.account.address).toBe(account.address)
	})

	// A custom signer is an ordinary object: wrap the built-in and the solver is
	// none the wiser.
	it("accepts a wrapper that adds policy around a built-in", async () => {
		const base = privateKeySigner(KEY)
		const seen: number[] = []
		const audited: Signer = {
			...base,
			mode: "audited",
			signTypedData: (typedData, chainId) => {
				seen.push(chainId ?? 0)
				return base.signTypedData(typedData, chainId)
			},
		}
		await assertSignsForItsAddress(audited)
		expect(seen).toEqual([0])
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
