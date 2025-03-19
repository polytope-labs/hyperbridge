import "log-timestamp"
import { ApiPromise, WsProvider } from "@polkadot/api"
import { Keyring } from "@polkadot/keyring"
import { hexToBytes, toHex } from "viem"
import { teleport } from "@/utils/tokenGateway"
import type { HexString } from "@/types"
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
describe("teleport function", () => {
	it("should teleport assets correctly", async () => {
		// Set up the connection to a local node
		const wsProvider = new WsProvider(process.env.BIFROST_PASEO)
		const api = await ApiPromise.create({ provider: wsProvider })

		console.log("Api connected")
		// Set up BOB account from keyring
		const keyring = new Keyring({ type: "sr25519" })
		const bob = keyring.addFromUri(secret_key)
		// Implement the Signer interface
		const signer: Signer = createKeyringPairSigner(bob)

		// Prepare test parameters
		const params = {
			symbol: "BNC",
			destination: "EVM-84532",
			recipient: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e" as HexString,
			amount: BigInt(200000000000),
			timeout: BigInt(3600),
			tokenGatewayAddress: hexToBytes("0xFcDa26cA021d5535C3059547390E6cCd8De7acA6"),
			relayerFee: BigInt(0),
			redeem: false,
		}

		try {
			// Call the teleport function
			console.log("Teleport started")
			let dispatched = null
			let finalized = null
			const stream = await teleport(api, bob.address, params, { signer })
			for await (const event of stream) {
				console.log(event.kind)
				if (event.kind === "Dispatched") {
					dispatched = event
				}
				if (event.kind === "Finalized") {
					finalized = event
				}
			}

			expect(dispatched).toMatchObject(
				expect.objectContaining({
					kind: "Dispatched",
					transaction_hash: expect.stringContaining("0x"),
					block_number: expect.any(BigInt),
					commitment: expect.stringContaining("0x"),
				}),
			)

			expect(finalized).toMatchObject(
				expect.objectContaining({
					kind: "Finalized",
					transaction_hash: expect.stringContaining("0x"),
					block_number: expect.any(BigInt),
					commitment: expect.stringContaining("0x"),
				}),
			)
		} catch (error) {
			console.log(error)
			// The extrinsic should be decoded correctly but should fail at transaction fee payment since we are using the BOB account
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
