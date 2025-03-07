import { ApiPromise, WsProvider } from "@polkadot/api"
import { Keyring } from "@polkadot/keyring"
import type { HexString } from "@/types"
import { teleportDot } from "@/utils/xcmGateway"
import type { Signer, SignerResult } from "@polkadot/api/types"
import type { SignerPayloadRaw } from "@polkadot/types/types"
import { u8aToHex, hexToU8a } from "@polkadot/util"
import type { KeyringPair } from "@polkadot/keyring/types"

// private key for testnet transactions
const secret_key = process.env.SECRET_PHRASE || ""

/**
 * Jest test for the teleport function
 The goal of this test is to ensure the teleport extrinsic is correctly encoded
 The tx can be decoded by the rpc node
 */
describe("teleport DOT", () => {
	it("should teleport DOT correctly", async () => {
		// Set up the connection to a local node
		const relayProvider = new WsProvider(process.env.PASEO_RPC_URL)
		const relayApi = await ApiPromise.create({ provider: relayProvider })

		const wsProvider = new WsProvider(process.env.HYPERBRIDGE_GARGANTUA)
		const hyperbridge = await ApiPromise.create({ provider: wsProvider })

		console.log("Api connected")
		// Set up BOB account from keyring
		const keyring = new Keyring({ type: "sr25519" })
		const bob = keyring.addFromUri(secret_key)
		// Implement the Signer interface
		const signer: Signer = createKeyringPairSigner(bob)

		const params = {
			destination: 97,
			recipient: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e" as HexString,
			amount: BigInt(1),
			timeout: BigInt(3600),
			paraId: 4009,
		}

		try {
			// Call the teleport function
			//
			console.log("Teleport Dot started")
			const result = await teleportDot(relayApi, hyperbridge, bob.address, { signer }, params)

			for await (const event of result) {
				console.log(event.kind)
				if (event.kind === "Error") {
					throw new Error(event.error)
				}

				if (event.kind === "Ready") {
					console.log(event)
				}

				if (event.kind === "Dispatched") {
					console.log(event)
					return
				}
			}
		} catch (error) {
			expect(error).toBeUndefined()
		}
	}, 300_000)
})

function createKeyringPairSigner(pair: KeyringPair): Signer {
	return {
		/**
		 * Signs a raw payload
		 */
		async signRaw({ data }: SignerPayloadRaw): Promise<SignerResult> {
			// Sign the data
			const signature = u8aToHex(pair.sign(hexToU8a(data), { withType: true }))

			return {
				id: 1,
				signature,
			}
		},
	}
}
