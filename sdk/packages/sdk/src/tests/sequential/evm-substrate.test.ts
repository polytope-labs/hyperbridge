import { RequestStatus, TimeoutStatus, type HexString } from "@/types"
import EVM_HOST from "@/abis/evmHost"
import PING_MODULE from "@/abis/pingModule"
import ERC6160 from "@/abis/erc6160"
import HANDLER from "@/abis/handler"
import TOKEN_GATEWAY from "@/abis/tokenGateway"
import Keyring, { decodeAddress } from "@polkadot/keyring"
import { u8aToHex } from "@polkadot/util"
import {
	type Chain,
	createPublicClient,
	createWalletClient,
	getContract,
	http,
	keccak256,
	parseEventLogs,
	parseUnits,
	type PrivateKeyAccount,
	toHex,
} from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { bscTestnet } from "viem/chains"
import { DEFAULT_LOGGER, normalizeTimestamp, postRequestCommitment } from "@/utils"
import { IndexerClient } from "@/client"
import { createQueryClient } from "@/query-client"
import { bigIntReplacer } from "@/helpers/data.helpers"

const logger = DEFAULT_LOGGER.withTag("evm-substrate")

const Source = {
	name: "BSC Testnet",
	chainId: 97,
	stateMachineId: "EVM-97",
	networkType: "testnet",
	rpcUrls: [process.env.BSC_CHAPEL as string],
	consensus: { layer: "BNB Testnet", stateId: "BSC0" },
	ismpHost: "0x8Aa0Dea6D675d785A882967Bf38183f6117C09b7",
} as const

const Destination = {
	group: "substrate",
	name: "Cere Testnet",
	networkType: "testnet",
	chainId: "SUBSTRATE-cere",
	consensus: {
		layer: "Cere",
		stateId: "CERE",
	},
	rpcUrls: [process.env.CERE_LOCAL as string],
	estimatedTransferTime: "10.4 minute",
}

const Token = {
	name: "Cere",
	symbol: "CERE",
	address: "0xf310641B4B6c032D0c88d72d712C020fCa9805A3",
	decimals: 18,
}

function assertIsToday(timestamp: bigint) {
	const dateToCheck = new Date(Number(normalizeTimestamp(timestamp)))
	const today = new Date()

	const isToday =
		dateToCheck.getDate() === today.getDate() &&
		dateToCheck.getMonth() === today.getMonth() &&
		dateToCheck.getFullYear() === today.getFullYear()

	// Difference between timestamps and current timestamps must be less than an hour
	let diff = (today.getTime() - dateToCheck.getTime()) / 1000
	expect(isToday).toBeTruthy()
	expect(diff < 3600).toBeTruthy()
}

test("EVM -> Substrate token transfer", { timeout: 5_400_000 }, async () => {
	// get token data
	const token = Token
	const indexer = getIndexer()

	// setup account
	// setup EVM account
	const sender_account = privateKeyToAccount(process.env.PRIVATE_KEY as HexString)
	const recipient_account = getSubstrateAccount()

	const helper = await createHelpers({
		account: sender_account,
		chain: bscTestnet,
		rpc_url: process.env.BSC_CHAPEL as string,
	})

	// make transfer
	const tx_hash = await initiateEvmTx({
		source: Source,
		destination: Destination,
		from: sender_account.address,
		recipient: encodePolkaAddress(recipient_account.address) as HexString,
		token: token,
		amount: 0.02,
		timeout: 3600,
		relayerFee: 0,
		account: sender_account,
		helper: helper,
	})

	const postRequest = await getCommitment(helper, tx_hash)
	const commitment = postRequest.commitment

	console.log("Post Request Commitment:", commitment)
	const statusStream = indexer.postRequestStatusStream(commitment)

	for await (const status of statusStream) {
		switch (status.status) {
			case RequestStatus.SOURCE: {
				assertIsToday(BigInt(status.metadata.timestamp!))
				break
			}
			case RequestStatus.SOURCE_FINALIZED: {
				assertIsToday(BigInt(status.metadata.timestamp!))
				break
			}

			case RequestStatus.HYPERBRIDGE_DELIVERED: {
				assertIsToday(BigInt(status.metadata.timestamp!))
				break
			}

			case RequestStatus.HYPERBRIDGE_FINALIZED: {
				assertIsToday(BigInt(status.metadata.timestamp!))
				break
			}
		}

		console.log(JSON.stringify(status, null, 4))
		if (status.status === RequestStatus.HYPERBRIDGE_FINALIZED) {
			break
		}
	}

	const req = await indexer.queryRequestWithStatus(commitment)
	console.log("Full status", JSON.stringify(req, bigIntReplacer, 4))

	if (!req) {
		throw new Error("No RequestWithStatues")
	}

	const statuses = new Set(req.statuses.map((status) => status.status))

	expect(statuses).toContain(RequestStatus.HYPERBRIDGE_FINALIZED)
})

const singleton = <T>(fn: () => T) => {
	const EMPTY = "$EMPTY$"

	let output: T | typeof EMPTY = EMPTY

	return (): T => {
		if (output !== EMPTY) return output
		output = fn()
		return output
	}
}

const getIndexer = singleton(() => {
	const query_client = createQueryClient({
		url: process.env.INDEXER_URL as string,
	})

	return new IndexerClient({
		source: {
			consensusStateId: Source.consensus.stateId,
			rpcUrl: Source.rpcUrls[0],
			stateMachineId: Source.stateMachineId,
			host: Source.ismpHost,
		},
		dest: {
			hasher: "Blake2",
			wsUrl: Destination.rpcUrls[0],
			consensusStateId: Destination.consensus.stateId,
			stateMachineId: Destination.chainId,
		},
		hyperbridge: {
			consensusStateId: "PAS0",
			stateMachineId: "KUSAMA-4009",
			wsUrl: process.env.HYPERBRIDGE_GARGANTUA as string,
		},
		queryClient: query_client,
		pollInterval: 1000,
	})
})

function getSubstrateAccount() {
	const keyring = new Keyring({ type: "sr25519" })
	const bob = keyring.addFromUri(process.env.SECRET_PHRASE as string)

	return bob
}

function encodePolkaAddress(polkaAddress?: string): string {
	const keyring = new Keyring()

	return polkaAddress ? keyring.encodeAddress(polkaAddress, 0) : ""
}

const readAssetId = (token_symbol: string) => {
	const encoder = new TextEncoder()
	return keccak256(encoder.encode(token_symbol))
}

async function initiateEvmTx(params: BridgeParams) {
	const { account, helper } = params
	const to: HexString = u8aToHex(decodeAddress(params.recipient, false))

	const evm_token = params.token
	const assetId = readAssetId(evm_token.symbol)

	if (!assetId) {
		throw new Error(`Invalid assetId for token ${params.token.name}`)
	}

	const nativeCost = 0n

	const transfer_params = {
		amount: parseUnits(String(params.amount), params.token.decimals),
		assetId: assetId,
		data: "0x",
		dest: toHex(params.destination.chainId),
		nativeCost,
		redeem: false,
		relayerFee: parseUnits(
			String(params.relayerFee),
			18, // todo: Fetch FeeToken decimal
		),
		timeout: BigInt(params.timeout),
		to,
	} as const

	DEFAULT_LOGGER.debug("Initializing EVM transaction with params", {
		token: params.token,
		params: transfer_params,
	})

	const hash = await helper.tokenGateway.write.teleport([transfer_params], {
		// chain: bscTestnet,
		value: nativeCost,
		account,
	})

	// add to store
	return hash
}

async function createHelpers(params: { chain: Chain; account: PrivateKeyAccount; rpc_url: string }) {
	const { chain, account, rpc_url: rpc } = params

	const walletClient = createWalletClient({
		chain: chain,
		account,
		transport: http(rpc),
	})

	const publicClient = createPublicClient({
		chain: chain,
		transport: http(process.env.BSC_CHAPEL),
	})

	const sharedClient = { public: publicClient, wallet: walletClient }

	const ping = getContract({
		address: process.env.PING_MODULE_ADDRESS as HexString,
		abi: PING_MODULE.ABI,
		client: sharedClient,
	})

	const hostAddress = await ping.read.host()

	const host = getContract({
		address: hostAddress,
		abi: EVM_HOST.ABI,
		client: publicClient,
	})

	const hostParams = await host.read.hostParams()

	const handler = getContract({
		address: hostParams.handler,
		abi: HANDLER.ABI,
		client: sharedClient,
	})

	const feeToken = getContract({
		address: hostParams.feeToken,
		abi: ERC6160.ABI,
		client: sharedClient,
	})

	const tokenGateway = getContract({
		abi: TOKEN_GATEWAY.ABI,
		address: tokenGatewayAddress,
		client: sharedClient,
	})

	return {
		chain,
		publicClient,
		tokenGateway,
		walletClient: walletClient,
		host,
		ping,
		handler,
		feeToken: feeToken,
	}
}

const tokenGatewayAddress = process.env.TOKEN_GATEWAY_ADDRESS as HexString

async function getCommitment(helper: THelper, tx_hash: HexString) {
	// wait for tx receipt to become available
	await new Promise((resolve) => setTimeout(resolve, 5000))

	const receipt = await helper.publicClient.waitForTransactionReceipt({
		hash: tx_hash,
		confirmations: 1,
	})

	logger.log(`Transaction reciept: ${helper.chain.blockExplorers?.default?.url}/tx/${tx_hash}`)
	logger.log("Block: ", receipt.blockNumber)

	// parse EvmHost PostRequestEvent emitted in the transcation logs
	const event = parseEventLogs({ abi: EVM_HOST.ABI, logs: receipt.logs })[0]

	if (event.eventName !== "PostRequestEvent") {
		throw new Error("Unexpected Event type")
	}

	const request = event.args

	console.log("PostRequestEvent", { request })

	const commitment = postRequestCommitment(request).commitment

	return { ...request, commitment }
}

type THelper = Awaited<ReturnType<typeof createHelpers>>

type BridgeParams = {
	readonly source: typeof Source
	readonly from: HexString
	readonly destination: typeof Destination
	readonly token: typeof Token
	readonly amount: number
	readonly timeout: number
	readonly relayerFee: number
	readonly recipient: HexString
	readonly account: PrivateKeyAccount
	readonly helper: Awaited<ReturnType<typeof createHelpers>>
}
