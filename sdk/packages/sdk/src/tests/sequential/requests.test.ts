import "log-timestamp"

import {
	createPublicClient,
	createWalletClient,
	decodeFunctionData,
	getContract,
	hexToBytes,
	http,
	parseAbi,
	parseEventLogs,
	toHex,
} from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { bscTestnet, gnosisChiado } from "viem/chains"

import { IndexerClient } from "@/client"
import { type HexString, RequestStatus, TimeoutStatus } from "@/types"
import { getRequestCommitment, postRequestCommitment } from "@/utils"

import ERC6160 from "@/abis/erc6160"
import PING_MODULE from "@/abis/pingModule"
import EVM_HOST from "@/abis/evmHost"
import HANDLER from "@/abis/handler"
import { EvmChain, SubstrateChain } from "@/chain"
import { createQueryClient } from "@/query-client"
import { bigIntReplacer } from "@/helpers/data.helpers"

describe.sequential("Get and Post Requests", () => {
	let indexer: IndexerClient
	let hyperbridgeInstance: SubstrateChain

	beforeAll(async () => {
		const { gnosisChiadoHost, bscIsmpHost, hyperbridge } = await setUp()

		const query_client = createQueryClient({
			url: process.env.INDEXER_URL!,
		})

		indexer = new IndexerClient({
			source: {
				consensusStateId: "BSC0",
				rpcUrl: process.env.BSC_CHAPEL!,
				stateMachineId: "EVM-97",
				host: bscIsmpHost.address,
			},
			dest: {
				consensusStateId: "GNO0",
				rpcUrl: process.env.GNOSIS_CHIADO!,
				stateMachineId: "EVM-10200",
				host: gnosisChiadoHost.address,
			},
			hyperbridge: {
				consensusStateId: "PAS0",
				stateMachineId: "KUSAMA-4009",
				wsUrl: process.env.HYPERBRIDGE_GARGANTUA!,
			},
			queryClient: query_client,
			pollInterval: 1_000,
		})

		await hyperbridge.connect()
		hyperbridgeInstance = hyperbridge
	})

	afterAll(async () => {
		await hyperbridgeInstance.disconnect()
	})

	describe("Post Request", () => {
		it.skip("should stream and query the timeout status", async () => {
			const { bscTestnetClient, bscHandler, bscPing, gnosisChiadoHost } = await setUp()
			console.log("\n\nSending Post Request\n\n")

			const hash = await bscPing.write.ping([
				{
					dest: await gnosisChiadoHost.read.host(),
					count: BigInt(1),
					fee: BigInt(0),
					module: process.env.PING_MODULE_ADDRESS! as HexString,
					timeout: BigInt(150), // so it can timeout
				},
			])

			// wait for tx receipt to become available
			await new Promise((resolve) => setTimeout(resolve, 5000))

			const receipt = await bscTestnetClient.waitForTransactionReceipt({
				hash,
				confirmations: 1,
			})

			console.log(`Transaction reciept: ${bscTestnet.blockExplorers.default.url}/tx/${hash}`)
			console.log("Block: ", receipt.blockNumber)

			// parse EvmHost PostRequestEvent emitted in the transcation logs
			const event = parseEventLogs({ abi: EVM_HOST.ABI, logs: receipt.logs })[0]

			if (event.eventName !== "PostRequestEvent") {
				throw new Error("Unexpected Event type")
			}

			const request = event.args

			console.log("PostRequestEvent", { request })

			const commitment = postRequestCommitment(request).commitment

			console.log("Post Request Commitment:", commitment)
			const statusStream = indexer.postRequestStatusStream(commitment)

			for await (const status of statusStream) {
				console.log(JSON.stringify(status, null, 4))

				if (status.status === TimeoutStatus.PENDING_TIMEOUT) {
					console.log("Request is now timed out", request.timeoutTimestamp)
				}
			}

			console.log("Starting timeout stream")

			for await (const timeout of indexer.postRequestTimeoutStream(commitment)) {
				console.log(JSON.stringify(timeout, null, 4))
				switch (timeout.status) {
					case TimeoutStatus.DESTINATION_FINALIZED_TIMEOUT:
						console.log(
							`Status ${timeout.status}, Transaction: https://gargantua.statescan.io/#/extrinsics/${timeout.metadata?.transactionHash}`,
						)
						break
					case TimeoutStatus.HYPERBRIDGE_TIMED_OUT:
						console.log(
							`Status ${timeout.status}, Transaction: https://gargantua.statescan.io/#/extrinsics/${timeout.metadata?.transactionHash}`,
						)
						break
					case TimeoutStatus.HYPERBRIDGE_FINALIZED_TIMEOUT: {
						console.log(
							`Status ${timeout.status}, Transaction: https://testnet.bscscan.com/tx/${timeout.metadata?.transactionHash}`,
						)
						const { args, functionName } = decodeFunctionData({
							abi: HANDLER.ABI,
							data: timeout.metadata?.calldata! as any,
						})

						expect(functionName).toBe("handlePostRequestTimeouts")

						try {
							const hash = await bscHandler.write.handlePostRequestTimeouts(args as any)
							await bscTestnetClient.waitForTransactionReceipt({
								hash,
								confirmations: 1,
							})

							console.log(`Transaction timeout submitted: https://testnet.bscscan.com/tx/${hash}`)
						} catch (e) {
							console.error("Error self-relaying: ", e)
						}

						break
					}
					default:
						console.log("Unknown timeout status")
						break
				}
			}

			const req = await indexer.queryRequestWithStatus(commitment)
			console.log("Full status", JSON.stringify(req, null, 4))

			const hyperbridgeFinalizedStatus = req?.statuses.find(
				(status) => status.status === TimeoutStatus.HYPERBRIDGE_FINALIZED_TIMEOUT,
			)
			expect(hyperbridgeFinalizedStatus).toBeDefined()
			expect(hyperbridgeFinalizedStatus?.metadata.calldata).toBeDefined()
		}, 1200_000)

		it("Should scale encode the ISMP message correctly", async () => {
			const tx = hyperbridgeInstance.encode({
				kind: "TimeoutPostRequest",
				proof: {
					consensusStateId: "GNO0",
					height: 14521611n,
					proof: "0x1c5108f90211a0948c6c60ecc8a4c5b52abc1dd10f443ebb8260026b0d612bca6b9a109212b3b9a0dca88b1cb7af156a5ab839733fba6f5e453f79ae6e9b562d842e79b306f8ecb3a04ad21a053a57b8e4809e4784a1ff1a8c88c8df8aa390a6e564d95c5453b7c77ea05a35d3990b8c0aba736489c26a60d92e05af997ba7650e1634c5c2be8072154aa08341930c3f806f141a6f466a943081d21075cc11bb8d8581ff713fd6a83e0295a00f4c53e27af241636913b27be4e1ab580e581da0ea9d7d0a11c3ba8b73a4df81a08305b1e68ad37a0a002c581a22226a750ea111309034eecf3be7877256cbe10aa098e0f88044865234a7b764acbe97dfdcc346c64f2d63fbea3af8b896d37b865fa083050f2c1090ac0019a31dd6e84003021efc47f5a3c4dedbd169d91be7919a83a0c1faf279cc5357f52d599c445ba348878526d800608cedc27dd895c089c6a057a026c8da47d503c7fec2d87a3f85566fadf6c63df1dd6ec1dda2afa26ea577decda086815dd8cf3128abc904157125d8267f8a4e9ee1e861c56948fa6b6168f3ae91a0a81e2e30cd6cde88f964bcc7c899fd32d407af2a8468edcae84a54bb8fc9d0a3a007bf1d0f0353768af0e089b573ec639ef4249501f7a1039eb450db10cf28f028a0f7531550fcec0ad4b7a0d5f96fa6365d168de66a728c184eeb5ebbe8d191bcf7a00dd5b164fa5d099edcbe7e0162d12641be90c72bb7a8c37e1febc8d1bd5c7b3b805108f90211a09a1d017a8974b524b67f0a921e89e325a8ed0ffe7b8892af8fe4bd2b99c66558a0ddf5c3ffb5c578b4ca041827aef19b4c9ead253d78c67de74c0b253f2f56a77ea0c6b54e42752c28482f9e58ebd22d2d7426f6d5cc338f095ea7671779a9c7f8dca076b618c9095f44fbba89dc5fe73aebf5c45dac20c03c8daccd8f73ceb24cba2aa0f1017b5734304a4ced1661857526efb7855d377242108d31c11643284126ef9ba0f4d111be728321b23cea690a7b9686ea51cbfc823c46f9eb5b6f0b96468feb28a0ee238f35db6c736a95d09fa9008ba8e25db3a43fbb30e7c51d7411ee624e415ba0e0b1ab410b78755484780705e3055846eba49200b103c8a4644cc37550c1d021a03c8cdc01c96ef1e540a6926a3eec37f6b2613238c94cbd42c13cd216c08d9b12a025a093975e7ed5c51bec4532d0c28edf786e1371c461f1481b6bb062ab288608a00ed06e2a0a12022da0413b0df5041454e13f4dfdfa6b83e319bd384c0dc512b3a028e008fbcdba5786df9b8b99f62e9e73371b3495bba3b59a5006c606d5ab0ea7a0692b1bae78364df1315367a88c0c47109517baae362693dd79e5a85fed857d74a0f263961c7a17e6472641655af05daed3b3b70658180b4d0de3d95803c913c407a0f870b1a0ae3b7e6d8f607a338411d65b4b7767eb7fd7ddcf6b9b8d1410277e68a04be83aa290d05e9af78d11ac3143f0b08d7b87521688c753d728ca6b803f6244805108f90211a095fb2f625de7c891c05162826929b968b4e59e64b826afdc9a59fd2541d0e44da0dfc72e5ac52c0ec372d7c3de2a77ea59763c125e47d5205c853b0a9f35f64eafa0df761e6267819e5532ff85de0ef2560aef8f36795ed98229522472c65280cea7a09f7bc872fceb7837b7a0f474e0a6deafbe45874bbab03afbf501e4ac480a9d47a06fb5586dbe8dfc142f0be2a879cc28a864d47166d11c9aa57acf1ba14ef7eef0a00a8b655c514f7b93307f022108ec5a183be1e9096ac9612cd8a17f114bfc3c75a0655d4d9e180ce561d946ce4f1315afda7c1eccf740a2fe95e86091a008deb21aa04c4d1e5d4002204d98071a2665fb129998711eaf18b707d527a9bda172c461c1a0a682c3d910f43d54b4a6ce04cf0a920bf547033a0bdee23c7302c10fdf41c63ca09487089830300a19bd4d687476aa9adb28b9cf5f65cdf61a3518395dd0eff401a03d13224efbb3ff1b18a7c40aaebd674f4a7c691811eb669e6cadf1e043aa0750a0251e21bfe77234fa3bfb9928805af1277f1ee6a926f41093002f9acf1b6d9768a0fe0c5a99d645bd10b0e16cc5be8119c093f0db286e2ae80aeace6228ec23b6eea06ab1924c038d4cacee65538304695f8de7ff1c031376696e0be49b527ea84b3ca0a9c56a940a38b3a66ce5a011305960728cf38199286044e279ff698e9ce0ede6a00c06521e6742b3b7a8aa281a104698f27b0cd8d5390407daa3dea17f090fe569805108f90211a0cf37e8b4ed3bb3639c29e8fc5aeeb5b1206ce04901b9d51da34630ab40f34315a06be004fb4675f716a473c56177b79495b82c591813f8e7117f6b7a48202fdf5ba0d10ed0457ed1f3412db8d7537965ceb6fd564f07665d92cd69f57c60bc397025a058ec4a802528243b2a2119f123c19f035bfafc12ec96a2c0f6b4560ad44ae8d3a013b26d7707e7d0325317ace8de38989c690d04f2108c53be0942fae5cb0138e3a0d560187232aa66cc14705e9a1516c0d0184321df87ef5d47a63793f6b474ba6da089f077ee6eef8bc74ef6abe1684e47a6aada71a642ef9fb196d1b324a1de3c43a084a2bd45a56c85f7ba779b29c6380528d3d41333f153a24f53ff74d5af545235a016fe9254442db909b6a26b20a7c53901258f951c45044e6f9c938c8f589c0a9ba09e60c9e117435ece82c77963133c45ccf747c62496a46096e97d8628b8b13f26a04e3cffb04935d9915ffe51c323617de30dfca9403107ec194e1b85d2b458d7d0a0424ab8ab54a83774bd1eebcedc673cc01003717d17d8020fe85ae0393cc0e192a096da2524c757030a1079aa5d72a7de7be2537bdb41724c3999ef974e601f28cea041dbfd72ec2b4483056c260163bd7992f0cad4aa9ea1431472489c8e72bbe0aaa0e1944e4f8d1e8dffd9c118e209f0da886472ed8f82c4528aa8326ebab2098580a0d30eb7982d94297340a0aa5d612d16faae89df31ab20c2ca9da0378b1c54681a804d03f8d180a008e0eefa0183e091c2d605fce96e72446df6727e3a9573a12d51ca1463e66646a0c97bb301153ef741b60ceade236f492ac9dfd8c2a258caa165d21f987bbddc83a0ccfc8d240c5fb876969cd673617a860c35cffd969e0f6f8bdbc723307a0f841880808080a05daf87c236d3cec54be9abd4152e41f24a3099fec3c963c403469b77399c6d638080a0747e7e7315c251e1e44bcc6b8175e998320352061d7e6221746c3643b40c50508080a063c7bb9c5c53f351713f35177ae7bd54b57d0c62381c7484c1283a8dec6625e980804d01f851808080808080808080a06ec39144a27d9b64efe6db9b9283ceaa0381fd97d4c72f988e08357ef8ed8d9180808080a0bac8d471d92f1032eebdee3ac2a3bddd5804a5d324f1e4b8089540a9705d59ed8080a501f8679e2058ffd1710ecb31f89ba278f724badf9ad59584e4f90244ec528a3308feb846f8440180a0e0a4a5d6327d44a504037776854a3c423f51f21d6261387c4f835eb54299ee04a05f6dcb4dbb25da4ccadbb6f4660038481bdee8d3d6ac92910c7a6afe4b128cd0045058a41b89f4871725e5d898d98ef4bf917601c5eb145108f90211a0bd912e9bb0d615c781c25848951288013fc57ea9f1f0c9c7c26c463c3486a041a0de4f88c2a54f752a9a0753b92d0688f628f3d9bbefc846775787194f69d611e3a08872d061c1cc82e833cdec0343d7459d3d0e783465eb26092984f927fa294a95a06322f53441a8fa9f8de9a7d0f947a78e976060bec2982e37dc83b3b472360d7aa04427c604d9bb04df996a76be768e92e473876790d44ace9276b9889f4f44e411a087fbb0ce680b9454d94e10d38a1ad888de629b837afca4f3606e559dac8f4196a0c1cc52248ff71edccc25edd78e6d45c5d456858812090419d310c9f5da1d1b75a0013eefc9b3abd91a7108312a4139ef7dd76800884a3cbd1ccddf5c480e056934a034a167eb036651a9e0b3ff8478fbf284f6d03487aee0dd92b1ef25926a144779a0c78c14222e216fe49e633194808b14adb90d1495b7f9a85ee9fd9f5244426270a05c7ce9359fb7008546d2198f35599d99c9a8f15856a829310d00a252378bb8fca080c34b805b7f080edfd0f4626dfe375177a2dee0d877c9289dee77c27926515ea061f9a47e3ee5e4267d3d06059b1ead3e0cb6b09648dac712bf1081b2ba96e4efa0ab9264ffab8ad5a39bd49d0145741faee9c48456d886456a3dee823832dbd8b5a07306361d1ec2f53320108262300ad840247c50856ffe4f15288ad938ae5d7311a0e3a754b96c767dbabf0bab5fcdfa89bcd5090069453e9c93cdaeca8f0364175b805108f90211a07709767459e851bfb2afee62a891596801a19a5e312f38804769a6ec1b08503ea016ee2a870298b3820f09eabba17d398bdcb2854dab24913bfe3c42db8aa3076ea01d6e60144cb2af7d5b736d01a67092d000292ea1b579477ecdd7e3e7cea4c8f9a0998e0bb03ddec391b980fd44f4c1d53edce04beb416028c5bebe182740084e88a07c932c5fef311a348194d5c92626476d618abda6cbb85c8ebccd7379d28715d7a00675f4d77b0dbbd293ea6ebe30ac4ecd25783c0fe0b2b5b6ca104a45f7dafc4da041facc5bcb26f8c9782abbc4a9d758807877d2bcb334490f9b30328f6cd6f5cca0e49fdfc75c8c17899310f1dcd5fd39006a1e4b5c7b7348e113498d0c5f8d33cba0d94a1af5fd0fc0612af04d1b34e65cffc55c26b48ccf8304201b36aed8610c13a0e99921dcdad0ee6a59ac3c18e3f8f230d94960e0d9f1f8ee42e66a2800d8ea85a096d74d756c0614605400f2cfed4288b5c749aa83fc8c621df84620ea0805f58ba01a7a062edd3555372c176c3ee44ece0cff099f49c3d31ba623dda195c3ab9a3da07c7363c3181257dda6726da1df43e60fd3466f90780b2788513a4a259a01b69aa0cbd2fd49e10525d9f705ee670e5e4a03a9cd83806ed3673a1ff026cba942b15ca0f18f0a5587d18fc1bccb67f7574f8246b0d49eaaabd6d2d9f108d4802d13452da0b48138276dbe009c9221084821521f01986c2bbb7da9862349ba672c77d9f2d5805107f901d1a0c70c0047a62253b52fc39226a7eaa55356f1f0b75b1e2a0138b6bc61b84355c9a0a9652fd2e605f2f6705de369f9c6abf1b5b0891fb1be73f8bca492003005cc9ca0198b8955576bdbfcb957f6721302a5f4c3748fe8eb9910620a53bba25c70056aa03870f2c723b2c099169030a6edafd911b48ae9ec76e37a6c836b67f891e805faa05b7c3a7edac8be95cd8bf5092c620b9672ccd4e0642c148ece9dd62a7d4d45b7a095f5c32e09269c1476f0098120eeafb998c280092583f915a7338006713a8136a08bd6298bfc98f6f25c8adc4a28005af529478ddd70a48111e9c807032c002393a04704ab8c161e123e6b8c2a2fdc0aba207f37f8269154587ee5c1902e95a8ddd3a0eb96c063c0048ab4ffa32bd3fd0cfd70ebc96222b3ff35c345ce30a0666ea8c280a014e30d258afcc23ea4e16493fd04aa95e87325480e583cda6713dbc3b42766baa0631597a49f516827dd2abda01b46dd1da17ee14ffb5c0e148eb36ba66e915f42a0323a1937eb7d238e2a7f5bd2537d250622e68344d5cc60987c147773dddeb6d680a0956b6cdffa6b1496ae1dfb2284161c98d57503cb91cfc83b6e5aaa79790cec68a0b23d3c71647522b78f5bf35ea01e3fbe2809d6a86305140d970e9734720aa13d804d01f85180a000b4bcbd5d7ff682d53a0e6b503eb602caa980433067877fac4ab5146b88abad80808080a0222ba5547832038028bad582b04df45de3149a42c99514f67cfd89ea9f97a9e78080808080808080808000",
					stateMachine: "EVM-10200",
				},
				requests: [
					{
						body: "0x68656c6c6f2066726f6d2045564d2d3937",
						dest: "EVM-10200",
						from: "0xFE9f23F0F2fE83b8B9576d3FC94e9a7458DdDD35",
						nonce: 2247n,
						source: "EVM-97",
						timeoutTimestamp: 1740610545n,
						to: "0xfe9f23f0f2fe83b8b9576d3fc94e9a7458dddd35",
					},
				],
			})

			const evmChain = new EvmChain({
				url: process.env.GNOSIS_CHIADO!,
				chainId: 10200,
				host: "0x58A41B89F4871725E5D898d98eF4BF917601c5eB",
			})

			const receipt = await evmChain.queryRequestReceipt(
				"0x659f5bc8bb0073d4f1a25c1d420770d6aa08177bdf8d6bd6bd8476184d77de7d",
			)

			expect(receipt).toBeUndefined()

			// should not throw
			hyperbridgeInstance.api?.tx.ismp.handleUnsigned(hexToBytes(tx).slice(2))
		})

		it("should successfully stream and query the post request status", async () => {
			const { bscTestnetClient, gnosisChiadoHandler, bscPing, gnosisChiadoClient, gnosisChiadoHost } =
				await setUp()
			console.log("\n\nSending Post Request\n\n")

			const hash = await bscPing.write.ping([
				{
					dest: await gnosisChiadoHost.read.host(),
					count: BigInt(1),
					fee: BigInt(0),
					module: process.env.PING_MODULE_ADDRESS! as HexString,
					timeout: BigInt(60 * 60),
				},
			])

			const receipt = await bscTestnetClient.waitForTransactionReceipt({
				hash,
				confirmations: 1,
			})

			console.log(`Transaction reciept: ${bscTestnet.blockExplorers.default.url}/tx/${hash}`)
			console.log("Block: ", receipt.blockNumber)

			// parse EvmHost PostRequestEvent emitted in the transcation logs
			const event = parseEventLogs({ abi: EVM_HOST.ABI, logs: receipt.logs })[0]

			if (event.eventName !== "PostRequestEvent") {
				throw new Error("Unexpected Event type")
			}

			const request = event.args
			console.log("PostRequestEvent", { request })
			const commitment = postRequestCommitment(request).commitment

			for await (const status of indexer.postRequestStatusStream(commitment)) {
				console.log(JSON.stringify(status, null, 4))
				switch (status.status) {
					case RequestStatus.SOURCE_FINALIZED: {
						console.log(
							`Status ${status.status}, Transaction: https://gargantua.statescan.io/#/extrinsics/${status.metadata.transactionHash}`,
						)
						break
					}
					case RequestStatus.HYPERBRIDGE_DELIVERED: {
						console.log(
							`Status ${status.status}, Transaction: https://gargantua.statescan.io/#/extrinsics/${status.metadata.transactionHash}`,
						)
						break
					}
					case RequestStatus.HYPERBRIDGE_FINALIZED: {
						console.log(
							`Status ${status.status}, Transaction: https://gnosis-chiado.blockscout.com/tx/${status.metadata.transactionHash}`,
						)
						const { args, functionName } = decodeFunctionData({
							abi: HANDLER.ABI,
							data: status.metadata.calldata,
						})

						expect(functionName).toBe("handlePostRequests")

						try {
							const hash = await gnosisChiadoHandler.write.handlePostRequests(args as any)
							await gnosisChiadoClient.waitForTransactionReceipt({
								hash,
								confirmations: 1,
							})

							console.log(`Transaction submitted: https://gnosis-chiado.blockscout.com/tx/${hash}`)
						} catch (e) {
							console.error("Error self-relaying: ", e)
						}

						break
					}
					case RequestStatus.DESTINATION: {
						console.log(
							`Status ${status.status}, Transaction: https://gnosis-chiado.blockscout.com/tx/${status.metadata.transactionHash}`,
						)
						break
					}
				}
			}

			const req = await indexer.queryRequestWithStatus(commitment)
			console.log(JSON.stringify(req, bigIntReplacer, 4))
			expect(req?.statuses.length).toBe(5)
		}, 1_000_000)
	})

	describe("Get Request", () => {
		it("should successfully stream and query the get request status", async () => {
			const { bscTestnetClient, bscPing, gnosisChiadoHost, bscIsmpHost, bscHandler } = await setUp()
			console.log("\n\nSending Get Request\n\n")

			const latestHeight = await hyperbridgeInstance.latestStateMachineHeight({
				stateId: { Evm: 10200 },
				consensusStateId: toHex("GNO0"),
			})

			const hash = await bscPing.write.dispatch([
				{
					source: await bscIsmpHost.read.host(),
					dest: await gnosisChiadoHost.read.host(),
					nonce: await bscIsmpHost.read.nonce(),
					from: process.env.PING_MODULE_ADDRESS! as `0x${string}`,
					timeoutTimestamp: BigInt(Math.floor(Date.now() / 1000) + 60 * 60),
					keys: ["0xFE9f23F0F2fE83b8B9576d3FC94e9a7458DdDD35"],
					height: latestHeight,
					context: "0x",
				},
			])

			const receipt = await bscTestnetClient.waitForTransactionReceipt({
				hash,
				confirmations: 1,
			})

			console.log(`Transaction reciept: ${bscTestnet.blockExplorers.default.url}/tx/${hash}`)
			console.log("Block: ", receipt.blockNumber)

			// parse EvmHost GetRequestEvent emitted in the transcation logs
			const event = parseEventLogs({ abi: EVM_HOST.ABI, logs: receipt.logs })[0]

			if (event.eventName !== "GetRequestEvent") {
				throw new Error("Unexpected Event type")
			}

			const request = event.args
			console.log("GetRequestEvent", { request })
			const commitment = getRequestCommitment({ ...request, keys: [...request.keys] })
			console.log("Get Request Commitment: ", commitment)

			for await (const status of indexer.getRequestStatusStream(commitment)) {
				switch (status.status) {
					case RequestStatus.SOURCE_FINALIZED: {
						console.log(
							`Status ${status.status}, Transaction: https://gargantua.statescan.io/#/extrinsics/${status.metadata.transactionHash}`,
						)
						break
					}
					case RequestStatus.HYPERBRIDGE_DELIVERED: {
						console.log(
							`Status ${status.status}, Transaction: https://gargantua.statescan.io/#/extrinsics/${status.metadata.transactionHash}`,
						)
						break
					}
					case RequestStatus.HYPERBRIDGE_FINALIZED: {
						console.log(
							`Status ${status.status}, Transaction: https://testnet.bscscan.com/tx/${status.metadata.transactionHash}`,
						)
						const { args, functionName } = decodeFunctionData({
							abi: HANDLER.ABI,
							data: status.metadata.calldata,
						})

						expect(functionName).toBe("handleGetResponses")

						try {
							const hash = await bscHandler.write.handleGetResponses(args as any)
							await bscTestnetClient.waitForTransactionReceipt({
								hash,
								confirmations: 1,
							})

							console.log(`Transaction submitted: https://testnet.bscscan.com/tx/${hash}`)
						} catch (e) {
							console.error("Error self-relaying: ", e)
						}

						break
					}
					case RequestStatus.DESTINATION: {
						console.log(
							`Status ${status.status}, Transaction: https://testnet.bscscan.com/tx/${status.metadata.transactionHash}`,
						)
						break
					}
				}
			}

			const req = await indexer.queryGetRequest(commitment)
			expect(req?.statuses.length).toBe(3)
		}, 1_000_000)
	})
})

async function setUp() {
	const account = privateKeyToAccount(process.env.PRIVATE_KEY as any)

	const bscWalletClient = createWalletClient({
		chain: bscTestnet,
		account,
		transport: http(process.env.BSC_CHAPEL),
	})

	const gnosisChiadoWallet = createWalletClient({
		chain: gnosisChiado,
		account,
		transport: http(process.env.GNOSIS_CHIADO),
	})

	const bscTestnetClient = createPublicClient({
		chain: bscTestnet,
		transport: http(process.env.BSC_CHAPEL),
	})

	const gnosisChiadoClient = createPublicClient({
		chain: gnosisChiado,
		transport: http(process.env.GNOSIS_CHIADO),
	})

	const bscPing = getContract({
		address: process.env.PING_MODULE_ADDRESS! as HexString,
		abi: PING_MODULE.ABI,
		client: { public: bscTestnetClient, wallet: bscWalletClient },
	})

	const bscIsmpHostAddress = await bscPing.read.host()

	const bscIsmpHost = getContract({
		address: bscIsmpHostAddress,
		abi: EVM_HOST.ABI,
		client: bscTestnetClient,
	})

	const bscHostParams = await bscIsmpHost.read.hostParams()

	const bscHandler = getContract({
		address: bscHostParams.handler,
		abi: HANDLER.ABI,
		client: { public: bscTestnetClient, wallet: bscWalletClient },
	})

	const bscFeeToken = getContract({
		address: bscHostParams.feeToken,
		abi: ERC6160.ABI,
		client: { public: bscTestnetClient, wallet: bscWalletClient },
	})

	const gnosisChiadoPing = getContract({
		address: process.env.PING_MODULE_ADDRESS! as HexString,
		abi: PING_MODULE.ABI,
		client: gnosisChiadoClient,
	})

	const gnosisChiadoHostAddress = await gnosisChiadoPing.read.host()

	const gnosisChiadoHost = getContract({
		address: gnosisChiadoHostAddress,
		abi: EVM_HOST.ABI,
		client: gnosisChiadoClient,
	})

	const gnosisChiadoHostParams = await gnosisChiadoHost.read.hostParams()

	const gnosisChiadoHandler = getContract({
		address: gnosisChiadoHostParams.handler,
		abi: HANDLER.ABI,
		client: { public: gnosisChiadoClient, wallet: gnosisChiadoWallet },
	})

	const tokenFaucet = getContract({
		address: "0x17d8cc0859fbA942A7af243c3EBB69AbBfe0a320",
		abi: parseAbi(["function drip(address token) public"]),
		client: { public: bscTestnetClient, wallet: bscWalletClient },
	})

	const hyperbridge = new SubstrateChain({
		ws: process.env.HYPERBRIDGE_GARGANTUA!,
		hasher: "Keccak",
	})

	return {
		bscTestnetClient,
		bscFeeToken,
		account,
		tokenFaucet,
		gnosisChiadoHandler,
		bscHandler,
		bscPing,
		gnosisChiadoClient,
		gnosisChiadoHost,
		bscIsmpHost,
		hyperbridge,
	}
}
