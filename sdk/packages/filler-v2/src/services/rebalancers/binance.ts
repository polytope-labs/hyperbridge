import { Wallet, WalletRestAPI } from "@binance/wallet"
import { parseUnits, type Hex } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { type HexString, parseStateMachineId } from "@hyperbridge/sdk"
import { ChainClientManager } from "../ChainClientManager"
import { FillerConfigService } from "../FillerConfigService"
import { getLogger, type Logger } from "../Logger"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { UnifiedRebalanceOptions } from "."

export interface BinanceCexConfig {
	apiKey: string
	apiSecret: string
	basePath?: string
	timeout?: number
	pollIntervalMs?: number
	/** Optional travel rule questionnaire payload (see Binance docs). */
	travelRuleQuestionnaire?: Record<string, unknown> | null
}

export interface CexRebalanceResult {
	success: boolean
	depositTxHash: HexString
	/** For travel rule: this is the trId. For non-travel-rule: this is the withdrawal id. */
	withdrawalId: string
	amountDeposited: string
	amountReceived: string
	withdrawalFee: string
	elapsedMs: number
}

export interface CexRebalanceEstimate {
	withdrawalFee: string
	minWithdrawal: string
	withdrawEnabled: boolean
	depositEnabled: boolean
	/** Source network info */
	source: {
		network: string
		/** Min block confirmations for deposit credit */
		minConfirm: number
		/** Block confirmations before funds are unlocked for withdrawal */
		unLockConfirm: number
		/** Binance estimated arrival time in minutes */
		estimatedArrivalTime: number
		/** Whether the network is currently congested */
		busy: boolean
	}
	/** Destination network info */
	destination: {
		network: string
		/** Binance estimated arrival time in minutes for withdrawal */
		estimatedArrivalTime: number
		busy: boolean
	}
}

type BinanceNetworkInfo = NonNullable<WalletRestAPI.AllCoinsInformationResponse[number]["networkList"]>[number] & {
	minConfirm?: number | bigint
	unLockConfirm?: number | bigint
	estimatedArrivalTime?: number | bigint
	busy?: boolean
}

// Binance deposit status codes (GET /sapi/v1/capital/deposit/hisrec)
const DEPOSIT_STATUS_SUCCESS = 1

// Binance withdrawal status codes
const WITHDRAW_STATUS_COMPLETED = 6
const WITHDRAW_STATUS_TERMINAL_FAILURES = new Set([1, 3, 5]) // cancelled, rejected, failure

const CHAIN_ID_TO_BINANCE_NETWORK: Record<number, string> = {
	1: "ETH",
	56: "BSC",
	137: "MATIC",
	42161: "ARBITRUM",
	8453: "BASE",
}

function getChainIdFromStateMachine(stateMachineId: string): number {
	const chainId = parseStateMachineId(stateMachineId).stateId.Evm
	if (chainId === undefined) {
		throw new Error(`${stateMachineId} is not an EVM chain`)
	}
	return chainId
}

function getBinanceNetwork(stateMachineId: string): string {
	const chainId = getChainIdFromStateMachine(stateMachineId)
	const network = CHAIN_ID_TO_BINANCE_NETWORK[chainId]
	if (!network) {
		throw new Error(
			`Chain ${stateMachineId} (chainId: ${chainId}) has no Binance network mapping. ` +
				`Supported: ${Object.entries(CHAIN_ID_TO_BINANCE_NETWORK)
					.map(([k, v]) => `${v}(${k})`)
					.join(", ")}`,
		)
	}
	return network
}

/** Handles cross-chain rebalancing via Binance CEX (with optional travel rule support). */
export class BinanceRebalancer {
	private readonly walletClient: Wallet
	private readonly chainClientManager: ChainClientManager
	private readonly configService: FillerConfigService
	private readonly privateKey: HexString
	private readonly config: BinanceCexConfig
	private readonly logger: Logger
	private readonly pollIntervalMs: number
	private readonly travelRuleQuestionnaire: Record<string, unknown> | null

	constructor(
		chainClientManager: ChainClientManager,
		configService: FillerConfigService,
		privateKey: HexString,
		config: BinanceCexConfig,
	) {
		this.chainClientManager = chainClientManager
		this.configService = configService
		this.privateKey = privateKey
		this.config = config
		this.logger = getLogger("BinanceRebalancer")

		this.pollIntervalMs = config.pollIntervalMs ?? 15_000
		this.travelRuleQuestionnaire = config.travelRuleQuestionnaire ?? null

		this.walletClient = new Wallet({
			configurationRestAPI: {
				apiKey: config.apiKey,
				apiSecret: config.apiSecret,
				basePath: config.basePath,
				timeout: config.timeout ?? 5_000,
			},
		})
	}

	private get useTravelRule(): boolean {
		return this.travelRuleQuestionnaire !== null
	}

	/** Full rebalance: on-chain deposit → Binance credit → withdrawal to destination chain. */
	async sendViaCex(options: UnifiedRebalanceOptions): Promise<CexRebalanceResult> {
		const startTime = Date.now()
		const { amount, coin, source, destination } = options

		const sourceNetwork = getBinanceNetwork(source)
		const destNetwork = getBinanceNetwork(destination)

		const { sourceNetworkInfo, destNetworkInfo } = await this.fetchNetworkInfo(coin, sourceNetwork, destNetwork)

		if (!sourceNetworkInfo.depositEnable) {
			throw new Error(
				`Deposits paused for ${coin} on ${sourceNetwork}.` +
					(sourceNetworkInfo.depositDesc ? ` Reason: ${sourceNetworkInfo.depositDesc}` : ""),
			)
		}
		if (!destNetworkInfo.withdrawEnable) {
			throw new Error(
				`Withdrawals paused for ${coin} on ${destNetwork}.` +
					(destNetworkInfo.withdrawDesc ? ` Reason: ${destNetworkInfo.withdrawDesc}` : ""),
			)
		}

		const sourceEta = Number(sourceNetworkInfo.estimatedArrivalTime ?? 20) // minutes
		const destEta = Number(destNetworkInfo.estimatedArrivalTime ?? 20)

		// Prefer Binance's configured confirmation blocks over plain time-based ETA
		const sourceMinConfirm = Number(sourceNetworkInfo.minConfirm ?? 0)

		// For deposits, use confirmation blocks as the primary stopping condition.
		// We still keep a generous global time cap inside waitForDepositCredit as a safety guard.
		const depositTimeoutBlocks = sourceMinConfirm || 120 // sensible default if Binance doesn't specify
		const withdrawTimeoutMs = destEta * 60_000 + 30_000 // 30 seconds buffer

		this.logger.info(
			{
				amount,
				coin,
				source: sourceNetwork,
				destination: destNetwork,
				travelRule: this.useTravelRule,
				depositTimeoutMin: sourceEta,
				withdrawTimeoutMin: destEta,
				// confirmation / ETA details
				sourceMinConfirm,
				sourceUnLockConfirm: Number(sourceNetworkInfo.unLockConfirm ?? 0),
				sourceEstimatedArrivalTime: sourceEta,
				destEstimatedArrivalTime: destEta,
			},
			"Starting CEX rebalance",
		)

		const depositResp = await this.walletClient.restAPI
			.depositAddress({ coin, network: sourceNetwork })
			.then((res) => res.data())

		const depositAddress = depositResp.address
		if (!depositAddress) {
			throw new Error(`Binance did not return a deposit address for ${coin} on network ${sourceNetwork}`)
		}
		this.logger.info({ depositAddress, network: sourceNetwork }, "Got deposit address")

		const depositTxHash = await this.transferOnChain(source, coin, amount, depositAddress)
		this.logger.info({ depositTxHash }, "On-chain deposit transfer sent")

		await this.waitForDepositCredit(coin, depositTxHash, depositTimeoutBlocks)
		this.logger.info("Deposit credited on Binance")

		const account = privateKeyToAccount(this.privateKey as `0x${string}`)
		let withdrawalId: string

		if (this.useTravelRule) {
			withdrawalId = await this.withdrawWithTravelRule(coin, account.address, amount, destNetwork)
		} else {
			withdrawalId = await this.withdrawStandard(coin, account.address, amount, destNetwork)
		}

		this.logger.info(
			{
				withdrawalId,
				travelRule: this.useTravelRule,
				destNetwork,
				destEstimatedArrivalTime: destEta,
				sourceMinConfirm: Number(sourceNetworkInfo.minConfirm ?? 0),
				sourceUnLockConfirm: Number(sourceNetworkInfo.unLockConfirm ?? 0),
			},
			"Withdrawal initiated",
		)

		const finalWithdrawal = this.useTravelRule
			? await this.waitForTravelRuleWithdrawalComplete(withdrawalId, withdrawTimeoutMs)
			: await this.waitForStandardWithdrawalComplete(withdrawalId, withdrawTimeoutMs)

		const elapsedMs = Date.now() - startTime
		const result: CexRebalanceResult = {
			success: true,
			depositTxHash,
			withdrawalId,
			amountDeposited: amount,
			amountReceived: finalWithdrawal.amount,
			withdrawalFee: finalWithdrawal.transactionFee,
			elapsedMs,
		}

		this.logger.info({ ...result, elapsedSeconds: Math.round(elapsedMs / 1000) }, "CEX rebalance completed")

		return result
	}

	/** Estimate fees and timing for a CEX rebalance using live Binance network data. */
	async estimateCexRebalance(options: UnifiedRebalanceOptions): Promise<CexRebalanceEstimate> {
		const { coin, source, destination } = options
		const destNetwork = getBinanceNetwork(destination)
		const sourceNetwork = getBinanceNetwork(source)

		const { sourceNetworkInfo, destNetworkInfo } = await this.fetchNetworkInfo(coin, sourceNetwork, destNetwork)

		return {
			withdrawalFee: destNetworkInfo.withdrawFee ?? "0",
			minWithdrawal: destNetworkInfo.withdrawMin ?? "0",
			withdrawEnabled: destNetworkInfo.withdrawEnable ?? false,
			depositEnabled: sourceNetworkInfo.depositEnable ?? false,
			source: {
				network: sourceNetwork,
				minConfirm: Number(sourceNetworkInfo.minConfirm ?? 0),
				unLockConfirm: Number(sourceNetworkInfo.unLockConfirm ?? 0),
				estimatedArrivalTime: Number(sourceNetworkInfo.estimatedArrivalTime ?? 0),
				busy: sourceNetworkInfo.busy ?? false,
			},
			destination: {
				network: destNetwork,
				estimatedArrivalTime: Number(destNetworkInfo.estimatedArrivalTime ?? 0),
				busy: destNetworkInfo.busy ?? false,
			},
		}
	}

	/** Fetch and validate network config for a coin on source and destination networks. */
	private async fetchNetworkInfo(
		coin: string,
		sourceNetwork: string,
		destNetwork: string,
	): Promise<{ sourceNetworkInfo: BinanceNetworkInfo; destNetworkInfo: BinanceNetworkInfo }> {
		const allCoins: WalletRestAPI.AllCoinsInformationResponse = await this.walletClient.restAPI
			.allCoinsInformation()
			.then((res) => res.data())

		const coinInfo = allCoins.find((c) => c.coin === coin)
		if (!coinInfo) {
			throw new Error(`Coin ${coin} not found on Binance`)
		}

		const sourceNetworkInfo = coinInfo.networkList?.find((n) => n.network === sourceNetwork)
		const destNetworkInfo = coinInfo.networkList?.find((n) => n.network === destNetwork)

		if (!sourceNetworkInfo || !destNetworkInfo) {
			throw new Error(
				`Network not found for ${coin} on Binance. ` +
					`source=${sourceNetwork} (${sourceNetworkInfo ? "found" : "missing"}), ` +
					`dest=${destNetwork} (${destNetworkInfo ? "found" : "missing"})`,
			)
		}

		if (
			destNetworkInfo.withdrawFee === undefined ||
			destNetworkInfo.withdrawMin === undefined ||
			destNetworkInfo.withdrawEnable === undefined ||
			sourceNetworkInfo.depositEnable === undefined
		) {
			throw new Error(
				`Incomplete network configuration for ${coin} on Binance. ` +
					`destNetwork=${destNetwork}, sourceNetwork=${sourceNetwork}`,
			)
		}

		return { sourceNetworkInfo, destNetworkInfo }
	}

	/** Standard withdrawal via POST /sapi/v1/capital/withdraw/apply. */
	private async withdrawStandard(coin: string, address: string, amount: string, network: string): Promise<string> {
		const resp = await this.walletClient.restAPI
			.withdraw({
				coin,
				address,
				amount: Number(amount),
				network,
			})
			.then((res) => res.data())

		const id = resp.id
		if (!id) {
			throw new Error("Binance did not return a withdrawal id")
		}
		return id
	}

	/** Travel rule withdrawal via POST /sapi/v1/localentity/withdraw/apply. */
	private async withdrawWithTravelRule(
		coin: string,
		address: string,
		amount: string,
		network: string,
	): Promise<string> {
		const questionnaire = JSON.stringify(this.travelRuleQuestionnaire)

		this.logger.debug({ questionnaire, coin, network, address }, "Submitting travel rule withdrawal")

		const resp = await this.walletClient.restAPI
			.withdrawTravelRule({
				coin,
				address,
				amount: Number(amount),
				network,
				questionnaire,
			})
			.then((res) => res.data())

		this.logger.debug({ resp }, "Travel rule withdrawal response")

		if ((resp as any).accpted === false) {
			throw new Error(`Travel rule withdrawal rejected: ${(resp as any).info || "unknown reason"}`)
		}

		const trId = (resp as any).trId
		if (trId === undefined || trId === null) {
			throw new Error(
				`Binance did not return a trId for travel rule withdrawal. Response: ${JSON.stringify(resp)}`,
			)
		}

		return String(trId)
	}

	/** Poll GET /sapi/v1/capital/withdraw/history until a standard withdrawal completes or fails. */
	private async waitForStandardWithdrawalComplete(
		withdrawalId: string,
		timeoutMs: number,
	): Promise<{ amount: string; transactionFee: string; txId: string }> {
		const startTime = Date.now()

		while (Date.now() - startTime < timeoutMs) {
			try {
				const withdrawals: WalletRestAPI.WithdrawHistoryResponse = await this.walletClient.restAPI
					.withdrawHistory({})
					.then((res) => res.data())

				const match = withdrawals.find((w) => w.id === withdrawalId)

				if (match) {
					if (match.status === WITHDRAW_STATUS_COMPLETED) {
						const { amount, transactionFee, txId } = match
						if (!amount || !transactionFee || !txId) {
							throw new Error(
								`Withdrawal ${withdrawalId} completed but missing fields: ` +
									`amount=${amount}, transactionFee=${transactionFee}, txId=${txId}`,
							)
						}
						this.logger.debug({ txId }, "Standard withdrawal completed")
						return { amount, transactionFee, txId }
					}

					const status = match.status
					if (status !== undefined && WITHDRAW_STATUS_TERMINAL_FAILURES.has(Number(status))) {
						throw new Error(
							`Withdrawal ${withdrawalId} failed with status ${status}` +
								(match.info ? `: ${match.info}` : ""),
						)
					}

					this.logger.debug({ status }, "Standard withdrawal in progress")
				}
			} catch (error) {
				if (error instanceof Error && error.message.includes("failed with status")) {
					throw error
				}
				this.logger.warn({ error }, "Error polling standard withdrawal status, retrying...")
			}

			await this.sleep(this.pollIntervalMs)
		}

		throw new Error(`Withdrawal ${withdrawalId} not completed within ${timeoutMs / 60000} minutes.`)
	}

	/** Poll GET /sapi/v2/localentity/withdraw/history until a travel rule withdrawal completes or fails. */
	private async waitForTravelRuleWithdrawalComplete(
		trId: string,
		timeoutMs: number,
	): Promise<{ amount: string; transactionFee: string; txId: string }> {
		const startTime = Date.now()
		const trIdNum = Number(trId)

		while (Date.now() - startTime < timeoutMs) {
			try {
				// GET /sapi/v2/localentity/withdraw/history
				const withdrawals: any[] = await this.signedRequest("GET", "/sapi/v2/localentity/withdraw/history", {})

				const match = withdrawals.find((w: any) => w.trId === trIdNum || String(w.trId) === trId)

				if (match) {
					const withdrawalStatus = match.withdrawalStatus
					const travelRuleStatus = match.travelRuleStatus

					this.logger.debug({ trId, withdrawalStatus, travelRuleStatus }, "Travel rule withdrawal status")

					// Travel rule rejection
					if (travelRuleStatus === 2) {
						throw new Error(
							`Travel rule rejected for trId=${trId}. ` +
								(match.info ? `Reason: ${match.info}` : "Check Binance UI for details."),
						)
					}

					// Withdrawal completed
					if (withdrawalStatus === WITHDRAW_STATUS_COMPLETED) {
						const amount = match.amount
						const transactionFee = match.transactionFee
						const txId = match.txId

						if (!amount || !txId) {
							throw new Error(
								`Travel rule withdrawal trId=${trId} completed but missing fields: ` +
									`amount=${amount}, transactionFee=${transactionFee}, txId=${txId}`,
							)
						}

						this.logger.debug({ txId, trId }, "Travel rule withdrawal completed")
						return {
							amount,
							transactionFee: transactionFee || "0",
							txId,
						}
					}

					// Terminal withdrawal failure
					if (
						withdrawalStatus !== undefined &&
						WITHDRAW_STATUS_TERMINAL_FAILURES.has(Number(withdrawalStatus))
					) {
						throw new Error(
							`Travel rule withdrawal trId=${trId} failed with withdrawalStatus=${withdrawalStatus}` +
								(match.info ? `: ${match.info}` : ""),
						)
					}
				} else {
					this.logger.debug({ trId }, "Travel rule withdrawal not yet in history")
				}
			} catch (error) {
				if (
					error instanceof Error &&
					(error.message.includes("failed with") || error.message.includes("rejected"))
				) {
					throw error
				}
				this.logger.warn({ error }, "Error polling travel rule withdrawal status, retrying...")
			}

			await this.sleep(this.pollIntervalMs)
		}

		throw new Error(`Travel rule withdrawal trId=${trId} not completed within ${timeoutMs / 60000} minutes.`)
	}

	private async transferOnChain(
		source: string,
		coin: "USDC" | "USDT",
		amount: string,
		toAddress: string,
	): Promise<HexString> {
		const publicClient = this.chainClientManager.getPublicClient(source)
		const walletClient = this.chainClientManager.getWalletClient(source)

		const tokenAddress =
			coin === "USDC" ? this.configService.getUsdcAsset(source) : this.configService.getUsdtAsset(source)

		if (!tokenAddress || tokenAddress === "0x") {
			throw new Error(`${coin} not configured for chain ${source}`)
		}

		const tokenDecimals = await publicClient.readContract({
			address: tokenAddress as `0x${string}`,
			abi: ERC20_ABI,
			functionName: "decimals",
		})

		const amountWei = parseUnits(amount, Number(tokenDecimals))

		const balance = await publicClient.readContract({
			address: tokenAddress as `0x${string}`,
			abi: ERC20_ABI,
			functionName: "balanceOf",
			args: [walletClient.account!.address],
		})

		if (balance < amountWei) {
			throw new Error(`Insufficient ${coin} balance on ${source}: have ${balance}, need ${amountWei}`)
		}

		const txHash = await walletClient.writeContract({
			address: tokenAddress as `0x${string}`,
			abi: ERC20_ABI,
			functionName: "transfer",
			args: [toAddress as `0x${string}`, amountWei],
			account: walletClient.account!,
			chain: walletClient.chain,
		})

		const receipt = await publicClient.waitForTransactionReceipt({
			hash: txHash,
			confirmations: 2,
		})

		if (receipt.status !== "success") {
			throw new Error(`On-chain transfer to Binance failed: tx ${txHash}`)
		}

		return txHash
	}

	/** Raw HMAC-SHA256 signed request to Binance SAPI (used for travel rule history). */
	private async signedRequest(
		method: "GET" | "POST",
		path: string,
		params: Record<string, string | number>,
	): Promise<any> {
		const { createHmac } = await import("crypto")

		const basePath = this.config.basePath || "https://api.binance.com"
		const apiKey = this.config.apiKey
		const apiSecret = this.config.apiSecret

		const timestamp = Date.now().toString()
		const queryParams = new URLSearchParams()

		for (const [key, value] of Object.entries(params)) {
			if (value !== undefined && value !== null) {
				queryParams.append(key, String(value))
			}
		}
		queryParams.append("timestamp", timestamp)
		queryParams.append("recvWindow", "5000")

		const signature = createHmac("sha256", apiSecret).update(queryParams.toString()).digest("hex")

		queryParams.append("signature", signature)

		const url = method === "GET" ? `${basePath}${path}?${queryParams.toString()}` : `${basePath}${path}`

		const fetchOptions: RequestInit = {
			method,
			headers: {
				"X-MBX-APIKEY": apiKey,
				"Content-Type": "application/x-www-form-urlencoded",
			},
		}

		if (method === "POST") {
			fetchOptions.body = queryParams.toString()
		}

		const response = await fetch(url, fetchOptions)

		if (!response.ok) {
			const body = await response.text()
			let errorMsg: string
			try {
				const parsed = JSON.parse(body)
				errorMsg = `Binance API error [${path}]: ${parsed.code} - ${parsed.msg}`
			} catch {
				errorMsg = `Binance API error [${path}]: ${response.status} ${response.statusText} - ${body}`
			}
			throw new Error(errorMsg)
		}

		return response.json()
	}

	/** Wait for a Binance deposit to be credited, using confirmTimes and minConfirm as primary criteria. */
	async waitForDepositCredit(coin: string, txHash: string, requiredConfirmations: number): Promise<void> {
		const startTime = Date.now()
		const normalizedTxHash = txHash.toLowerCase()

		// Safety guard: do not poll forever even if confirmations never reach the target
		const maxTimeoutMs = 30 * 60_000 // 30 minutes hard cap

		while (Date.now() - startTime < maxTimeoutMs) {
			try {
				const deposits: WalletRestAPI.DepositHistoryResponse = await this.walletClient.restAPI
					.depositHistory({
						coin,
					})
					.then((res) => res.data())

				const match = deposits.find((d) => {
					const dTxId = (d.txId || "").toLowerCase()
					return (
						dTxId === normalizedTxHash ||
						dTxId === normalizedTxHash.replace("0x", "") ||
						`0x${dTxId}` === normalizedTxHash
					)
				})

				if (match) {
					const confirmTimes = Number(match.confirmTimes ?? 0)

					// Treat either explicit SUCCESS status or reaching the required
					// confirmation blocks as success.
					if (match.status === DEPOSIT_STATUS_SUCCESS || confirmTimes >= requiredConfirmations) {
						this.logger.debug(
							{ status: match.status, confirmTimes, requiredConfirmations },
							"Deposit fully confirmed",
						)
						return
					}
					this.logger.debug(
						{ status: match.status, confirmTimes, requiredConfirmations },
						"Deposit found, waiting for additional confirmations",
					)
				} else {
					this.logger.debug("Deposit not yet visible in Binance history")
				}
			} catch (error) {
				this.logger.warn({ error }, "Error polling deposit status, retrying...")
			}

			await this.sleep(this.pollIntervalMs)
		}

		throw new Error(
			`Deposit not confirmed within ${requiredConfirmations} blocks or ${maxTimeoutMs / 60000} minutes. ` +
				`txHash: ${txHash}. Check Binance deposit history manually.`,
		)
	}

	private sleep(ms: number): Promise<void> {
		return new Promise((resolve) => setTimeout(resolve, ms))
	}
}
