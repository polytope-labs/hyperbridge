import { parseUnits, padHex, maxUint256, type Hex } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { bytes20ToBytes32, type HexString, parseStateMachineId } from "@hyperbridge/sdk"
import { ChainClientManager } from "../ChainClientManager"
import { FillerConfigService } from "../FillerConfigService"
import { getLogger, type Logger } from "../Logger"
import { OFT_ABI } from "@/config/abis/Oft"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { RebalanceOptions } from "."

const LZ_SCAN_API = "https://scan.layerzero-api.com/v1/messages/tx"
const DEST_DELIVERED_TIMEOUT_MS = 15 * 60_000 // 15 minutes
const DEST_DELIVERED_POLL_MS = 10_000 // 10 seconds

export interface Usdt0TransferResult {
	success: boolean
	txHash: HexString
	amountSent: bigint
	amountReceived: bigint
	nativeFee: bigint
}

export interface Usdt0EstimateResult {
	amountSent: bigint
	amountReceived: bigint
	nativeFee: bigint
	minAmount: bigint
	maxAmount: bigint
}

export class Usdt0Rebalancer {
	private readonly chainClientManager: ChainClientManager
	private readonly configService: FillerConfigService
	private readonly privateKey: HexString
	private readonly logger: Logger

	constructor(chainClientManager: ChainClientManager, configService: FillerConfigService, privateKey: HexString) {
		this.chainClientManager = chainClientManager
		this.configService = configService
		this.privateKey = privateKey
		this.logger = getLogger("Usdt0Rebalancer")
	}

	async sendUsdt0(options: RebalanceOptions): Promise<Usdt0TransferResult> {
		const { amount, source, destination, recipientAddress } = options
		const sourceChainId = parseStateMachineId(source).stateId.Evm
		const destEid = this.configService.getLayerZeroEid(destination)
		if (!destEid) throw new Error(`Chain ${destination} not supported by USDT0 (no LayerZero EID)`)
		const oftAddress = this.configService.getUsdt0OftAddress(source)
		const tokenAddress = this.configService.getUsdtAsset(source)
		if (!oftAddress) throw new Error(`Chain ${source} not supported by USDT0 (no OFT address)`)
		if (!tokenAddress || tokenAddress === "0x") throw new Error(`Chain ${source} has no USDT configured`)

		this.logger.info({ amount, source, destination, destEid }, "Initiating USDT0 transfer")

		const publicClient = this.chainClientManager.getPublicClient(source)
		const walletClient = this.chainClientManager.getWalletClient(source)
		const account = privateKeyToAccount(this.privateKey as `0x${string}`)
		const recipient = recipientAddress || account.address
		const amountWei = parseUnits(amount, 6)

		if (sourceChainId === 1) {
			const allowance = await publicClient.readContract({
				address: tokenAddress as `0x${string}`,
				abi: ERC20_ABI,
				functionName: "allowance",
				args: [account.address, oftAddress],
			})
			if (allowance < amountWei) {
				const approveTx = await walletClient.writeContract({
					address: tokenAddress as `0x${string}`,
					abi: ERC20_ABI,
					functionName: "approve",
					args: [oftAddress, maxUint256],
					account,
					chain: walletClient.chain,
				})
				await publicClient.waitForTransactionReceipt({ hash: approveTx, confirmations: 1 })
			}
		}

		const recipientBytes32 = bytes20ToBytes32(recipient)
		const sendParam = {
			dstEid: destEid,
			to: recipientBytes32,
			amountLD: amountWei,
			minAmountLD: 0n,
			extraOptions: "0x" as HexString,
			composeMsg: "0x" as HexString,
			oftCmd: "0x" as HexString,
		}

		const [, , oftReceipt] = await publicClient.readContract({
			address: oftAddress,
			abi: OFT_ABI,
			functionName: "quoteOFT",
			args: [sendParam],
		})
		sendParam.minAmountLD = oftReceipt.amountReceivedLD

		const msgFee = await publicClient.readContract({
			address: oftAddress,
			abi: OFT_ABI,
			functionName: "quoteSend",
			args: [sendParam, false],
		})

		const txHash = await walletClient.writeContract({
			address: oftAddress,
			abi: OFT_ABI,
			functionName: "send",
			args: [sendParam, msgFee, recipient],
			value: msgFee.nativeFee,
			account,
			chain: walletClient.chain,
		})

		const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash, confirmations: 1 })
		if (receipt.status !== "success") {
			return {
				success: false,
				txHash,
				amountSent: oftReceipt.amountSentLD,
				amountReceived: oftReceipt.amountReceivedLD,
				nativeFee: msgFee.nativeFee,
			}
		}

		await this.waitForDelivered(txHash)
		this.logger.info({ txHash }, "LayerZero message delivered on destination")

		return {
			success: true,
			txHash,
			amountSent: oftReceipt.amountSentLD,
			amountReceived: oftReceipt.amountReceivedLD,
			nativeFee: msgFee.nativeFee,
		}
	}

	private async waitForDelivered(srcTxHash: string): Promise<void> {
		const deadline = Date.now() + DEST_DELIVERED_TIMEOUT_MS
		const url = `${LZ_SCAN_API}/${srcTxHash}`

		this.logger.info({ srcTxHash }, "Waiting for LayerZero delivery")

		while (Date.now() < deadline) {
			const res = await fetch(url)
			const json = (await res.json()) as { data?: Array<{ status?: { name?: string } }> }
			const msg = json.data?.[0]
			if (msg?.status?.name === "DELIVERED") {
				return
			}
			await new Promise((r) => setTimeout(r, DEST_DELIVERED_POLL_MS))
		}

		throw new Error(
			`LayerZero message not delivered within ${DEST_DELIVERED_TIMEOUT_MS / 60_000} min. tx: ${srcTxHash}`,
		)
	}

	async estimateUsdt0(options: RebalanceOptions): Promise<Usdt0EstimateResult> {
		const { amount, source, destination, recipientAddress } = options
		const destEid = this.configService.getLayerZeroEid(destination)
		if (!destEid) throw new Error(`Chain ${destination} not supported by USDT0`)
		const oftAddress = this.configService.getUsdt0OftAddress(source)
		if (!oftAddress) throw new Error(`Chain ${source} not supported by USDT0`)

		const publicClient = this.chainClientManager.getPublicClient(source)
		const account = privateKeyToAccount(this.privateKey as `0x${string}`)
		const recipient = recipientAddress || account.address
		const amountWei = parseUnits(amount, 6)
		const recipientBytes32 = padHex(recipient, { size: 32 })
		const sendParam = {
			dstEid: destEid,
			to: recipientBytes32,
			amountLD: amountWei,
			minAmountLD: 0n,
			extraOptions: "0x" as HexString,
			composeMsg: "0x" as HexString,
			oftCmd: "0x" as HexString,
		}

		const [oftLimit, , oftReceipt] = await publicClient.readContract({
			address: oftAddress,
			abi: OFT_ABI,
			functionName: "quoteOFT",
			args: [sendParam],
		})
		sendParam.minAmountLD = oftReceipt.amountReceivedLD
		const msgFee = await publicClient.readContract({
			address: oftAddress,
			abi: OFT_ABI,
			functionName: "quoteSend",
			args: [sendParam, false],
		})

		return {
			amountSent: oftReceipt.amountSentLD,
			amountReceived: oftReceipt.amountReceivedLD,
			nativeFee: msgFee.nativeFee,
			minAmount: oftLimit.minAmountLD,
			maxAmount: oftLimit.maxAmountLD,
		}
	}
}
