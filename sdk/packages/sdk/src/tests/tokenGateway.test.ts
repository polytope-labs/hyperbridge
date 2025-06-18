import "log-timestamp"
import { ApiPromise, WsProvider } from "@polkadot/api"
import { Keyring } from "@polkadot/keyring"
import {
	createPublicClient,
	createWalletClient,
	getContract,
	hexToBytes,
	http,
	maxUint256,
	parseAbi,
	parseEventLogs,
	PublicClient,
	stringToHex,
	WalletClient,
} from "viem"
import { teleport } from "@/utils/tokenGateway"
import type { HexString } from "@/types"
import type { Signer, SignerResult } from "@polkadot/api/types"
import type { SignerPayloadRaw } from "@polkadot/types/types"
import { hexToU8a, u8aToHex } from "@polkadot/util"
import type { KeyringPair } from "@polkadot/keyring/types"
import { encodeISMPMessage } from "@/chain"
import { __test, ADDRESS_ZERO, bytes20ToBytes32 } from "@/utils"
import { createQueryClient } from "@/query-client"
import { IndexerClient } from "@/client"
import { privateKeyToAccount, privateKeyToAddress } from "viem/accounts"
import { bscTestnet, gnosisChiado } from "viem/chains"
import tokenGateway from "@/abis/tokenGateway"
import { keccakAsU8a } from "@polkadot/util-crypto"
import erc6160 from "@/abis/erc6160"

// private key for testnet transactions
const secret_key = process.env.SECRET_PHRASE || ""

/**
 * Jest test for the teleport function
 The goal of this test is to ensure the teleport extrinsic is correctly encoded
 The tx can be decoded by the rpc node
 */
describe("teleport function", () => {
	it.skip("should teleport assets correctly", async () => {
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
			const stream = await teleport({ apiPromise: api, who: bob.address, params, options: { signer } })

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

	it("should query the order status", async () => {
		const { bscTokenGateway, bscPublicClient, bscWalletClient } = await setUp()
		const bscIsmpHostAddress = "0x8Aa0Dea6D675d785A882967Bf38183f6117C09b7" as HexString
		const gnosisChiadoIsmpHostAddress = "0x58a41b89f4871725e5d898d98ef4bf917601c5eb" as HexString
		const query_client = createQueryClient({
			url: process.env.INDEXER_URL!,
		})

		const indexer = new IndexerClient({
			source: {
				consensusStateId: "BSC0",
				rpcUrl: process.env.BSC_CHAPEL!,
				stateMachineId: "EVM-97",
				host: bscIsmpHostAddress,
			},
			dest: {
				consensusStateId: "GNO0",
				rpcUrl: process.env.GNOSIS_CHIADO!,
				stateMachineId: "EVM-10200",
				host: gnosisChiadoIsmpHostAddress,
			},
			hyperbridge: {
				consensusStateId: "PAS0",
				stateMachineId: "KUSAMA-4009",
				wsUrl: process.env.HYPERBRIDGE_GARGANTUA!,
			},
			queryClient: query_client,
			pollInterval: 1_000,
		})

		const amount = BigInt(200000000000)

		const params = {
			amount: amount,
			relayerFee: BigInt(0),
			assetId: u8aToHex(keccakAsU8a("USD.h")),
			redeem: false,
			to: bytes20ToBytes32(privateKeyToAddress(process.env.PRIVATE_KEY as any) as HexString),
			dest: stringToHex("EVM-10200"),
			timeout: 65337297n,
			nativeCost: BigInt(0),
			data: "0x" as HexString,
		}

		const { erc20, erc6160 } = await getAssetDetails(params.assetId, bscPublicClient)

		const tokenToApprove = erc20 != ADDRESS_ZERO ? erc20 : erc6160

		// Reject test if assetId is unknown

		if (tokenToApprove === ADDRESS_ZERO) {
			throw new Error("Unknown asset Id")
		}

		// Apporve tokens if needed
		await approveTokens(bscWalletClient, bscPublicClient, erc6160, bscTokenGateway.address)
		await dripTokensIfNeeded(bscWalletClient, bscPublicClient, amount)

		const tx = await bscTokenGateway.write.teleport([params], {
			account: bscWalletClient.account!,
			chain: bscTestnet,
		})

		const receipt = await bscPublicClient.waitForTransactionReceipt({
			hash: tx,
			confirmations: 1,
		})

		console.log("Teleported to Gnosis Chiado:", receipt.transactionHash)

		// Get the commitment from the AssetTeleported event
		const teleportEvent = parseEventLogs({
			abi: tokenGateway.ABI,
			logs: receipt.logs,
			strict: false,
		})[0] as { eventName: "AssetTeleported"; args: any }

		if (teleportEvent.eventName !== "AssetTeleported") {
			throw new Error("Unexpected Event type")
		}

		const commitment = teleportEvent.args.commitment
		console.log("Teleport Commitment:", commitment)

		// Stream the teleport status
		for await (const status of indexer.tokenGatewayAssetTeleportedStatusStream(commitment)) {
			console.log(JSON.stringify(status, (_, value) => (typeof value === "bigint" ? value.toString() : value), 4))
			switch (status.status) {
				case "TELEPORTED": {
					console.log(
						`Status ${status.status}, Transaction: https://testnet.bscscan.com/tx/${status.metadata.transactionHash}`,
					)
					break
				}
				case "RECEIVED": {
					console.log(
						`Status ${status.status}, Transaction: https://gnosis-chiado.blockscout.com/tx/${status.metadata.transactionHash}`,
					)
					break
				}
				case "REFUNDED": {
					console.log(
						`Status ${status.status}, Transaction: https://testnet.bscscan.com/tx/${status.metadata.transactionHash}`,
					)
					break
				}
			}
		}
	}, 300_000_000)
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

describe("encode ISMP Message", () => {
	const JsonBigInt = {
		stringify: (value: unknown) => {
			function bigIntReplacer(key: string, value: unknown) {
				if (typeof value === "bigint") {
					return `${value.toString()}n`
				}

				return value
			}

			return JSON.stringify(value, bigIntReplacer)
		},

		parse: (text: string) => {
			function bigIntReviver(key: string, value: unknown) {
				if (typeof value === "string" && /^\d+n$/.test(value)) {
					return BigInt(value.slice(0, -1))
				}
				return value
			}

			return JSON.parse(text, bigIntReviver)
		},
	}

	it("should encode `PostRequest`", () => {
		const payload = `{"kind":"PostRequest","proof":{"stateMachine":"POLKADOT-3367","consensusStateId":"DOT0","proof":"0x000020150180a00080ca3a85525576886bec6c8d14af36a431f65cd69b186534ac4663fa0e27814cc3807c23c7a89f2bbd4f645294d8b38cd95e4a0d3f3d0d3b1fea5201f4af039a24be31018d02657175657374300080a920a50b2e9ba78e1dd0672a66d2f570ab33e118d0bd1f96d2147896292a5569807dc17b63e746171eff175360d83f867cf3a0b970e1f2c1ea44458204386168fd790895036f6d6d69746d656e7473ffff809218209ff94f779d571f37ac604f169a6882596e117c82e1ddde30a9468d838980f5e98bf19d3da8debf66441bb7765fb7ecb30acfa407afd713066d810ba54b2a80d955284bd0aa6fd3a5fb345362d970c6aefa4059ae58e943ba2d0592beaaf2a680306a9f48c656fadc0f5b1d7389d363f437c877bbb5f378ea1c78971950540bc580112aa1a00afd2d8fb0fbbafa51eb2b224e7e0c5c5883f6282a017afffee5075f804d1a02f10c23773279ed2f8e8343113c2070f7ae543c44a793fd585c040cb6e48051509abed107806b1442ee57a46b88775574d3e618a0a88e60489cfa731a4185808cd548c4fdd2a56c64ede561ff904286d2c55fbd00a3282d0b1e9272ad32b0dc805a475ea80a255761e0bc86a5f2dcdbcc489bdab805f028f6c6737094cec2a8a28052d89e68c5f281764966bf0e5c7933254f8447878cb9d9a783065f1000b5b98880469b1151ce050506cde87707e3ea0201bf646f3db6b587df3ab613f663be9a3080f54f9705d886edea6980898d31b9793ad0526baaf0187bcaa9bf9bf7015d4bf38086c4d2041c66d77db5ef174567343ad51e1458ccaaf7bf49d5d1a33522d7b86e80e0d7418f7da18a52401ce28e3d6955eb0da2a257fe2efdbc7c5d7fcddcb1d4f080e606d29855e081f7a32280db940a61f738025889da4a89040dfc270e30bbf26280217ca4f37e1b38b6e0d40f661f8d3754060a4580605e9027221415051762101e4d0880ffff80f784b6cf74b54ffc87cf3435415bb5368ac1c2c7cad21aa416d9ed9c8801ef94805e1501d7a070fb808add0bb93bc3e7df658631164e64cea044572bb39ce0568580f2d9de6fd21fcaf07fc676fb6cb05964fabb221b1365038ca34fc4cbaaf47051807ba5e861d3acae1495773cb408588cea830341d3db9f4719cfe175bbc904bfa880f571713eedbb22d8c61b06f974fea2fe03b587d346cebb249181d57e637409e180db5cf05a2bbd19eec2b26e515f04de442f97aa9a59c4792feb15d84c311e8ebb80d9adb2288642139f815640885701207eb9f91a77580f793b1e385f9814d8d630804edf752f1ff409cbafbbe40b46c3a21a3758e304439eff3c5f55cf6a0e912db080ec03d93b653c1479708b08eabe329c35986f5f4c2a78b45542f50be3d246afaf80c40e9c9da3c628cf4d57a7c89cad3126ce90d3e7c2759b047e0d5832eac46c7080dacedd80db8b7949b1cc8d58d3a62db2b69f2cafe92e72d8732647537f33688b8093d5b873c6d79abaeeb8267f057f0e93a8bbbda00ea0bdc24a1f9fa3e4d403838028509038542f7deb35cc961f071875280af9a0e485cbce77b8c763074aa6b4e58038ed71251cc4e26631ad2f6e504380cdca45be1db98e1c7eb43123e1eed615e980e96c04521701fbf5aec0069c482e705fb64d305889c4bcdd392b72e84ff3b4fe8024dc2f6f85ecff62cc0a40b8689a899d1f00f1e449e9171d89adef8b7cbb5d40b90580737e801e17cce5a9dd5178697c49d63b0e92578929dd5f99c296f4297901af00009e10809cda22e50076c30855370e72246c6137e761b4ddcd4f35b45ae8c4aea5e99f68809a1245a8f9e826ff521e7936c926445ce1b8dae51d45db5932ddaa19db17551e802ddf046292ae5ced08e848aa06204ae9a59a4b1ccf6f64810b74a169b8a095c18029e516dd9c41341073ae89072f0ec7d605a3c542983532c823a52e29a580167f808e47453bf2762aa02cf8837ab6580818408bc9447136d13cd91b37d29ec795cf807d59458a0199a22a2644a2b6f1e4d9c39654b32441db4a686e170b9e8b4618d0802ea1e2eccab065cf03b83de59a71a1f1647a38f4e72cb0ae9f58a46b5886a5f88064c5264fa7561678d05e3420bd50e928d4d5afaa5756fbecd96986bd2ce16e4f80c802bad572870bee7acb5fc358c58667a37cfe686a7cc3a750e8e317735f54a980f35ead630446103d8b800d49c625d3a0767c6671b9d0dcfb2191a22287d0b20d9901808c0080e5f168fcc2393a88f033fb5d856367912d336085ebd3ee8746041099586cbb6c801967e4d33076a7a26f4dd046fcd9aa24876269adcb4979a1420988aac3d8f3ef802ff41d763a2d5667944151c6312b3c1a332efcb70066df372e24b240e4df7aa301013f1dfcb5d3bc4413d7e22c617ab322ba07ecd0c1ee81332f769319759e19fede364499657d121012cca16379e58e1693794d3d26f04e9c3f4e737c6bc79b67cc05015c12000000000000b22400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","height":"5065606n"},"requests":[{"commitment":"0xcc93fcb5d3bc4413d7e22c617ab322ba07ecd0c1ee81332f769319759e19fede","timeoutTimestamp":"1746109367","source":"EVM-1","dest":"SUBSTRATE-cere","to":"0xa09b1c60e8650245f92518c8a17314878c4043ed","from":"0xFd413e3AFe560182C4471F4d143A96d3e259B6dE","nonce":"162","body":"0x000000000000000000000000000000000000000000000000000000000b2d05e000ac05b69379f7ac8d594d29d1cc11e6ed5bec3b481c0882bbb1c4fdaa08ba77c60000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c4ce6549c5f26de05898ab6e99005f0dbcd0d83a7aec163072f2e144b16f8a27a7ef164080577f3d4f1c8914de44768dd227ff34","statuses":[{"status":"SOURCE","metadata":{"blockHash":"0xde6cfc83c0745bfa30aa4e6cb0d798e8aea8eaf40d77fba8a1cc909875cc2cf6","blockNumber":22389271,"transactionHash":"0x691d41aa65f692285e4caab8ad79afa9299b17b44aea58d0442ceb8357c48b77","timestamp":"1746105767"}},{"status":"HYPERBRIDGE_DELIVERED","metadata":{"blockHash":"0xf353c01572d36f56b0c6a9b1e4b1f83c73bc482e553e1cf547c7c463ebd0390c","blockNumber":5065599,"transactionHash":"0xa74595b6c684437b0af33eaf4e4fcd6e60c3950af9b762ba86cdbce471645d81","timestamp":"1746106788000"}},{"status":"DESTINATION","metadata":{"blockHash":"0x1f37d55ecedf679ad221fb1d80db7c7119cc63c8d4f4617516476601988680e7","blockNumber":20453321,"transactionHash":"0xfa32d1d156443f2b016c62ce830fb9275ac8d111d23c3698183e639553f7be79","timestamp":"1746106908000"}}]}],"signer":"0x0000000000000000000000000000000000000000000000000000000000000000"}`

		const message = JsonBigInt.parse(payload)
		const output = encodeISMPMessage(message)

		expect(u8aToHex(output)).toMatchInlineSnapshot(
			`"0x04020400010000000363657265a20000000000000050fd413e3afe560182c4471f4d143a96d3e259b6de50a09b1c60e8650245f92518c8a17314878c4043edb7831368000000008502000000000000000000000000000000000000000000000000000000000b2d05e000ac05b69379f7ac8d594d29d1cc11e6ed5bec3b481c0882bbb1c4fdaa08ba77c60000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c4ce6549c5f26de05898ab6e99005f0dbcd0d83a7aec163072f2e144b16f8a27a7ef164080577f3d4f1c8914de44768dd227ff3401270d0000444f5430864b4d0000000000a91c000020150180a00080ca3a85525576886bec6c8d14af36a431f65cd69b186534ac4663fa0e27814cc3807c23c7a89f2bbd4f645294d8b38cd95e4a0d3f3d0d3b1fea5201f4af039a24be31018d02657175657374300080a920a50b2e9ba78e1dd0672a66d2f570ab33e118d0bd1f96d2147896292a5569807dc17b63e746171eff175360d83f867cf3a0b970e1f2c1ea44458204386168fd790895036f6d6d69746d656e7473ffff809218209ff94f779d571f37ac604f169a6882596e117c82e1ddde30a9468d838980f5e98bf19d3da8debf66441bb7765fb7ecb30acfa407afd713066d810ba54b2a80d955284bd0aa6fd3a5fb345362d970c6aefa4059ae58e943ba2d0592beaaf2a680306a9f48c656fadc0f5b1d7389d363f437c877bbb5f378ea1c78971950540bc580112aa1a00afd2d8fb0fbbafa51eb2b224e7e0c5c5883f6282a017afffee5075f804d1a02f10c23773279ed2f8e8343113c2070f7ae543c44a793fd585c040cb6e48051509abed107806b1442ee57a46b88775574d3e618a0a88e60489cfa731a4185808cd548c4fdd2a56c64ede561ff904286d2c55fbd00a3282d0b1e9272ad32b0dc805a475ea80a255761e0bc86a5f2dcdbcc489bdab805f028f6c6737094cec2a8a28052d89e68c5f281764966bf0e5c7933254f8447878cb9d9a783065f1000b5b98880469b1151ce050506cde87707e3ea0201bf646f3db6b587df3ab613f663be9a3080f54f9705d886edea6980898d31b9793ad0526baaf0187bcaa9bf9bf7015d4bf38086c4d2041c66d77db5ef174567343ad51e1458ccaaf7bf49d5d1a33522d7b86e80e0d7418f7da18a52401ce28e3d6955eb0da2a257fe2efdbc7c5d7fcddcb1d4f080e606d29855e081f7a32280db940a61f738025889da4a89040dfc270e30bbf26280217ca4f37e1b38b6e0d40f661f8d3754060a4580605e9027221415051762101e4d0880ffff80f784b6cf74b54ffc87cf3435415bb5368ac1c2c7cad21aa416d9ed9c8801ef94805e1501d7a070fb808add0bb93bc3e7df658631164e64cea044572bb39ce0568580f2d9de6fd21fcaf07fc676fb6cb05964fabb221b1365038ca34fc4cbaaf47051807ba5e861d3acae1495773cb408588cea830341d3db9f4719cfe175bbc904bfa880f571713eedbb22d8c61b06f974fea2fe03b587d346cebb249181d57e637409e180db5cf05a2bbd19eec2b26e515f04de442f97aa9a59c4792feb15d84c311e8ebb80d9adb2288642139f815640885701207eb9f91a77580f793b1e385f9814d8d630804edf752f1ff409cbafbbe40b46c3a21a3758e304439eff3c5f55cf6a0e912db080ec03d93b653c1479708b08eabe329c35986f5f4c2a78b45542f50be3d246afaf80c40e9c9da3c628cf4d57a7c89cad3126ce90d3e7c2759b047e0d5832eac46c7080dacedd80db8b7949b1cc8d58d3a62db2b69f2cafe92e72d8732647537f33688b8093d5b873c6d79abaeeb8267f057f0e93a8bbbda00ea0bdc24a1f9fa3e4d403838028509038542f7deb35cc961f071875280af9a0e485cbce77b8c763074aa6b4e58038ed71251cc4e26631ad2f6e504380cdca45be1db98e1c7eb43123e1eed615e980e96c04521701fbf5aec0069c482e705fb64d305889c4bcdd392b72e84ff3b4fe8024dc2f6f85ecff62cc0a40b8689a899d1f00f1e449e9171d89adef8b7cbb5d40b90580737e801e17cce5a9dd5178697c49d63b0e92578929dd5f99c296f4297901af00009e10809cda22e50076c30855370e72246c6137e761b4ddcd4f35b45ae8c4aea5e99f68809a1245a8f9e826ff521e7936c926445ce1b8dae51d45db5932ddaa19db17551e802ddf046292ae5ced08e848aa06204ae9a59a4b1ccf6f64810b74a169b8a095c18029e516dd9c41341073ae89072f0ec7d605a3c542983532c823a52e29a580167f808e47453bf2762aa02cf8837ab6580818408bc9447136d13cd91b37d29ec795cf807d59458a0199a22a2644a2b6f1e4d9c39654b32441db4a686e170b9e8b4618d0802ea1e2eccab065cf03b83de59a71a1f1647a38f4e72cb0ae9f58a46b5886a5f88064c5264fa7561678d05e3420bd50e928d4d5afaa5756fbecd96986bd2ce16e4f80c802bad572870bee7acb5fc358c58667a37cfe686a7cc3a750e8e317735f54a980f35ead630446103d8b800d49c625d3a0767c6671b9d0dcfb2191a22287d0b20d9901808c0080e5f168fcc2393a88f033fb5d856367912d336085ebd3ee8746041099586cbb6c801967e4d33076a7a26f4dd046fcd9aa24876269adcb4979a1420988aac3d8f3ef802ff41d763a2d5667944151c6312b3c1a332efcb70066df372e24b240e4df7aa301013f1dfcb5d3bc4413d7e22c617ab322ba07ecd0c1ee81332f769319759e19fede364499657d121012cca16379e58e1693794d3d26f04e9c3f4e737c6bc79b67cc05015c12000000000000b22400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000000"`,
		)
	})
})

it("should ensure wasm function loads", async () => {
	await expect(__test()).resolves.toMatchInlineSnapshot(
		`"{"root":"0x85835c4d5287fe023073eb733f4e4103935d61b4397f0f9d0fe627d434757fa5","proof":["0x894376e04f932deadc9ab212ac514f37b41e670be2f8002babde1faf20935461","0xf3ace1896f86f91627cc1c09eeaba2cd76d82a75be6f09b94c861524fa5e5289","0x0ffa0900c838d17341df2d00fa4832755de619e646137844700668ad544c8aae","0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470","0x9c6b2c1b0d0b25a008e6c882cc7b415f309965c72ad2b944ac0931048ca31cd5","0xfadbd3c7f79fa2bdc4f24857709cd4a4e870623dc9e9abcdfd6e448033e35212"],"mmr_size":236,"leaf_positions":[232],"keccak_hash_calldata":"0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"}"`,
	)
})

async function setUp() {
	const account = privateKeyToAccount(process.env.PRIVATE_KEY as any)

	const bscWalletClient = createWalletClient({
		chain: bscTestnet,
		account,
		transport: http(process.env.BSC_CHAPEL),
	})

	const gnosisChiadoWalletClient = createWalletClient({
		chain: gnosisChiado,
		account,
		transport: http(process.env.GNOSIS_CHIADO),
	})

	const bscPublicClient = createPublicClient({
		chain: bscTestnet,
		transport: http(process.env.BSC_CHAPEL),
	})

	const gnosisChiadoPublicClient = createPublicClient({
		chain: gnosisChiado,
		transport: http(process.env.GNOSIS_CHIADO),
	})

	const bscTokenGateway = getContract({
		address: process.env.TOKEN_GATEWAY_ADDRESS! as HexString,
		abi: tokenGateway.ABI,
		client: { public: bscPublicClient, wallet: bscWalletClient },
	})

	const gnosisChiadoTokenGateway = getContract({
		address: process.env.TOKEN_GATEWAY_ADDRESS! as HexString,
		abi: tokenGateway.ABI,
		client: { public: gnosisChiadoPublicClient, wallet: gnosisChiadoWalletClient },
	})

	return {
		bscTokenGateway,
		gnosisChiadoTokenGateway,
		bscPublicClient,
		gnosisChiadoPublicClient,
		bscWalletClient,
		gnosisChiadoWalletClient,
	}
}

async function getAssetDetails(assetId: string, publicClient: PublicClient) {
	const erc20 = await publicClient.readContract({
		abi: tokenGateway.ABI,
		address: process.env.TOKEN_GATEWAY_ADDRESS! as HexString,
		functionName: "erc20",
		args: [assetId as HexString],
	})

	const erc6160 = await publicClient.readContract({
		abi: tokenGateway.ABI,
		address: process.env.TOKEN_GATEWAY_ADDRESS! as HexString,
		functionName: "erc6160",
		args: [assetId as HexString],
	})

	return {
		erc20,
		erc6160,
	}
}

async function approveTokens(
	walletClient: WalletClient,
	publicClient: PublicClient,
	tokenAddress: HexString,
	spender: HexString,
) {
	const approval = await publicClient.readContract({
		abi: erc6160.ABI,
		address: tokenAddress,
		functionName: "allowance",
		args: [walletClient.account?.address as HexString, spender],
		account: walletClient.account,
	})

	if (approval == 0n) {
		console.log("Approving tokens for test")
		const tx = await walletClient.writeContract({
			abi: erc6160.ABI,
			address: tokenAddress,
			functionName: "approve",
			args: [spender, maxUint256],
			chain: walletClient.chain,
			account: walletClient.account!,
		})

		const receipt = await publicClient.waitForTransactionReceipt({ hash: tx })
		console.log("Approved tokens for test:", receipt)
	}
}

async function dripTokensIfNeeded(walletClient: WalletClient, publicClient: PublicClient, amountCheck: bigint) {
	const USDH = "0xA801da100bF16D07F668F4A49E1f71fc54D05177" as HexString
	const balance = await publicClient.readContract({
		abi: erc6160.ABI,
		address: USDH,
		functionName: "balanceOf",
		args: [walletClient.account?.address as HexString],
		account: walletClient.account,
	})

	if (balance < amountCheck) {
		console.log("Dripping tokens for test")
		const tx = await walletClient.writeContract({
			abi: parseAbi(["function drip(address token) public"]),
			address: "0x1794aB22388303ce9Cb798bE966eeEBeFe59C3a3",
			functionName: "drip",
			args: [USDH],
			chain: walletClient.chain,
			account: walletClient.account!,
		})

		const receipt = await publicClient.waitForTransactionReceipt({ hash: tx })
		console.log("Dripped tokens for test:", receipt)
	}
}
