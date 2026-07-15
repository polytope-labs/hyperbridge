import { erc20Abi, formatEther, formatUnits, zeroAddress } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import { ChainClientManager } from "./ChainClientManager"
import { FillerConfigService } from "./FillerConfigService"
import { getLogger } from "./Logger"
import type { SigningAccount } from "./wallet"
import { SIMPLEX_PAYMASTER_ABI } from "@/config/abis/SimplexPaymaster"
import { ENTRYPOINT_ABI } from "@/config/abis/Entrypoint"

const DEFAULT_INTERVAL_MINUTES = 30
const DEFAULT_MIN_SWAP_USD = 1

export interface PaymasterKeeperConfig {
	/** Chains to monitor. Defaults to every configured chain with a SimplexPaymaster. */
	chains?: string[]
	intervalMinutes?: number
	/** Minimum accrued balance (in whole stablecoin units) worth recycling. */
	minSwapUsd?: number
}

/**
 * Keeps SimplexPaymasters funded by periodically calling their onchain
 * `swapAndDeposit` — the contract itself swaps accrued stablecoins to native
 * via its configured router (with an oracle-bounded minimum output) and
 * deposits the proceeds into the EntryPoint. This service only decides *when*
 * to trigger it: whenever a registered token's accrued balance is worth
 * swapping. The signer must be the paymaster's treasury.
 */
export class PaymasterKeeperService {
	private logger = getLogger("paymaster-keeper")
	private timer?: NodeJS.Timeout
	private running = false
	private readonly decimalsCache = new Map<string, number>()

	constructor(
		private clientManager: ChainClientManager,
		private configService: FillerConfigService,
		private signer: SigningAccount,
		private config: PaymasterKeeperConfig = {},
	) {}

	/** Idempotent. Runs one cycle shortly after start, then on an interval. */
	start(chains: string[]): void {
		if (this.timer) return
		const targets = this.config.chains ?? chains
		const intervalMs = (this.config.intervalMinutes ?? DEFAULT_INTERVAL_MINUTES) * 60_000

		setTimeout(() => {
			this.runCycle(targets).catch((error) => this.logger.error({ error }, "Initial keeper cycle failed"))
		}, 10_000)

		this.timer = setInterval(() => {
			this.runCycle(targets).catch((error) => this.logger.error({ error }, "Keeper cycle failed"))
		}, intervalMs)

		this.logger.info({ chains: targets, intervalMs }, "Paymaster keeper started")
	}

	stop(): void {
		if (this.timer) {
			clearInterval(this.timer)
			this.timer = undefined
		}
	}

	async runCycle(chains: string[]): Promise<void> {
		if (this.running) return
		this.running = true
		try {
			for (const chain of chains) {
				try {
					await this.runChain(chain)
				} catch (error) {
					this.logger.error({ chain, error }, "Keeper cycle failed for chain")
				}
			}
		} finally {
			this.running = false
		}
	}

	private async runChain(chain: string): Promise<void> {
		const paymaster = this.configService.getSimplexPaymasterAddress(chain)
		if (!paymaster) {
			this.logger.debug({ chain }, "No SimplexPaymaster configured, skipping")
			return
		}

		const publicClient = this.clientManager.getPublicClient(chain)
		const keeper = this.signer.account.address as HexString

		let treasury: HexString
		let router: HexString
		let tokens: readonly HexString[]
		try {
			;[treasury, router, tokens] = await Promise.all([
				publicClient.readContract({
					address: paymaster,
					abi: SIMPLEX_PAYMASTER_ABI,
					functionName: "treasury",
				}) as Promise<HexString>,
				publicClient.readContract({
					address: paymaster,
					abi: SIMPLEX_PAYMASTER_ABI,
					functionName: "uniswapV2Router",
				}) as Promise<HexString>,
				publicClient.readContract({
					address: paymaster,
					abi: SIMPLEX_PAYMASTER_ABI,
					functionName: "getRegisteredTokens",
				}) as Promise<readonly HexString[]>,
			])
		} catch (error) {
			this.logger.warn(
				{ chain, paymaster, error },
				"Paymaster not readable (deployment may predate swapAndDeposit), skipping",
			)
			return
		}

		if (treasury.toLowerCase() !== keeper.toLowerCase()) {
			this.logger.warn({ chain, paymaster, treasury, keeper }, "Signer is not the paymaster treasury, skipping")
			return
		}
		if (router.toLowerCase() === zeroAddress) {
			this.logger.debug({ chain, paymaster }, "Swap router unset, skipping")
			return
		}

		for (const token of tokens) {
			try {
				await this.recycleToken(chain, paymaster, token)
			} catch (error) {
				this.logger.error({ chain, paymaster, token, error }, "Failed to recycle token")
			}
		}
	}

	private async recycleToken(chain: string, paymaster: HexString, token: HexString): Promise<void> {
		const publicClient = this.clientManager.getPublicClient(chain)
		const decimals = await this.getDecimals(chain, token)

		const balance = await publicClient.readContract({
			address: token,
			abi: erc20Abi,
			functionName: "balanceOf",
			args: [paymaster],
		})

		const minSwapAmount = BigInt(this.config.minSwapUsd ?? DEFAULT_MIN_SWAP_USD) * 10n ** BigInt(decimals)
		if (balance < minSwapAmount) {
			this.logger.debug(
				{ chain, token, balance: formatUnits(balance, decimals) },
				"Accrued balance below swap minimum",
			)
			return
		}

		const depositBefore = await this.getEntryPointDeposit(chain, paymaster)
		this.logger.info(
			{
				chain,
				paymaster,
				token,
				balance: formatUnits(balance, decimals),
				deposit: depositBefore !== undefined ? formatEther(depositBefore) : undefined,
			},
			"Recycling accrued fees into the EntryPoint deposit",
		)

		const walletClient = this.clientManager.getWalletClient(chain)
		const hash = await walletClient.writeContract({
			address: paymaster,
			abi: SIMPLEX_PAYMASTER_ABI,
			functionName: "swapAndDeposit",
			args: [token, 0n],
			chain: walletClient.chain,
		})

		const receipt = await publicClient.waitForTransactionReceipt({ hash })
		if (receipt.status !== "success") {
			throw new Error(`swapAndDeposit reverted: ${hash}`)
		}

		const depositAfter = await this.getEntryPointDeposit(chain, paymaster)
		this.logger.info(
			{
				chain,
				token,
				txHash: hash,
				deposit: depositAfter !== undefined ? formatEther(depositAfter) : undefined,
			},
			"Fees recycled",
		)
	}

	private async getEntryPointDeposit(chain: string, paymaster: HexString): Promise<bigint | undefined> {
		const entryPoint = this.configService.getEntryPointAddress(chain)
		if (!entryPoint) return undefined

		return this.clientManager.getPublicClient(chain).readContract({
			address: entryPoint,
			abi: ENTRYPOINT_ABI,
			functionName: "balanceOf",
			args: [paymaster],
		})
	}

	private async getDecimals(chain: string, token: HexString): Promise<number> {
		const key = `${chain}:${token.toLowerCase()}`
		const cached = this.decimalsCache.get(key)
		if (cached !== undefined) return cached

		const decimals = await this.clientManager.getPublicClient(chain).readContract({
			address: token,
			abi: erc20Abi,
			functionName: "decimals",
		})
		this.decimalsCache.set(key, decimals)
		return decimals
	}
}
