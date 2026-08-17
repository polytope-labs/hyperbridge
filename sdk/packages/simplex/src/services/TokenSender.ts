import { ERC20_ABI } from "@/config/abis/ERC20"
import { ERC4626_ABI } from "@/config/abis/Erc4626"
import type { VaultToml } from "@/config/filler-toml"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { UserOpSender } from "@/services/UserOpSender"
import { type Logger , moduleLogger} from "@/services/Logger"
import { encodeERC7821ExecuteBatch, type ERC7821Call, type HexString } from "@hyperbridge/sdk"
import { encodeFunctionData, isAddress, parseUnits } from "viem"


export interface SendTokenParams {
	/** State machine id, e.g. "EVM-8453". */
	chain: string
	/** "native" or an ERC-20 address. Configured vault shares are redeemed to the recipient instead of transferred. */
	token: string
	/** Human-readable amount, parsed against the token's decimals. */
	amount: string
	to: HexString
}

export interface SendTokenResult {
	txHash: HexString
	sponsored: boolean
	/** Set when vault holdings were redeemed as part of the send. */
	redeemed: boolean
}

/**
 * Operator-initiated outbound transfers from the filler wallet. Uses the same
 * sponsored UserOp path as fills (gas paid by the paymaster) with a native-tx
 * fallback. When the wallet balance of an ERC-20 falls short and a configured
 * ERC-4626 vault holds it as underlying, the shortfall is withdrawn from the
 * vault in the same batch as the transfer. Sending a vault's share token
 * directly issues `redeem(shares, recipient, solver)` — shares only ever
 * leave as the underlying asset.
 */
export class TokenSender {
	private readonly logger: Logger

	constructor(
		private readonly clientManager: ChainClientManager,
		private readonly solver: HexString,
		/** Live view of the configured vaults so runtime vault edits are honored. */
		private readonly getVaults: () => VaultToml[],
		private readonly userOpSender?: UserOpSender,
	) {
		this.logger = moduleLogger(clientManager.loggers, "token-sender")
	}

	async send(params: SendTokenParams): Promise<SendTokenResult> {
		const { chain, token, amount, to } = params
		if (!isAddress(to)) throw new Error(`Invalid recipient address: ${to}`)
		const publicClient = this.clientManager.getPublicClient(chain)

		const calls: ERC7821Call[] = []
		let redeemed = false
		if (token === "native") {
			const value = parseUnits(amount, 18)
			if (value <= 0n) throw new Error("Amount must be greater than 0")
			const balance = await publicClient.getBalance({ address: this.solver })
			if (balance < value) throw new Error(`Insufficient native balance: have ${balance}, need ${value}`)
			calls.push({ target: to, value, data: "0x" })
		} else {
			if (!isAddress(token)) throw new Error(`Invalid token address: ${token}`)
			const tokenLower = token.toLowerCase()
			const [decimals, balance] = await Promise.all([
				publicClient.readContract({ abi: ERC20_ABI, address: token, functionName: "decimals" }) as Promise<number>,
				publicClient.readContract({
					abi: ERC20_ABI,
					address: token,
					functionName: "balanceOf",
					args: [this.solver],
				}) as Promise<bigint>,
			])
			const units = parseUnits(amount, decimals)
			if (units <= 0n) throw new Error("Amount must be greater than 0")

			const isVaultShare = this.getVaults().some(
				(v) => v.chain === chain && v.vault.toLowerCase() === tokenLower,
			)
			if (isVaultShare) {
				if (balance < units) throw new Error(`Insufficient token balance: have ${balance}, need ${units}`)
				redeemed = true
				calls.push({
					target: token as HexString,
					value: 0n,
					data: encodeFunctionData({ abi: ERC4626_ABI, functionName: "redeem", args: [units, to, this.solver] }),
				})
			} else {
				if (balance < units) {
					const withdrawal = await this.vaultWithdrawalCall(chain, token as HexString, units - balance)
					if (!withdrawal) throw new Error(`Insufficient token balance: have ${balance}, need ${units}`)
					redeemed = true
					calls.push(withdrawal)
				}
				calls.push({
					target: token as HexString,
					value: 0n,
					data: encodeFunctionData({ abi: ERC20_ABI, functionName: "transfer", args: [to, units] }),
				})
			}
		}

		const result = await this.submit(chain, calls)
		this.logger.warn({ chain, token, amount, to, redeemed, ...result }, "Operator send executed")
		return { ...result, redeemed }
	}

	/**
	 * A `withdraw(shortfall, solver, solver)` call against a configured vault on
	 * this chain whose underlying is the token being sent and that can cover the
	 * shortfall, or null when no vault can.
	 */
	private async vaultWithdrawalCall(chain: string, token: HexString, shortfall: bigint): Promise<ERC7821Call | null> {
		const publicClient = this.clientManager.getPublicClient(chain)
		for (const vault of this.getVaults()) {
			if (vault.chain !== chain) continue
			const asset = (await publicClient.readContract({
				abi: ERC4626_ABI,
				address: vault.vault,
				functionName: "asset",
			})) as HexString
			if (asset.toLowerCase() !== token.toLowerCase()) continue
			const available = (await publicClient.readContract({
				abi: ERC4626_ABI,
				address: vault.vault,
				functionName: "maxWithdraw",
				args: [this.solver],
			})) as bigint
			if (available < shortfall) continue
			return {
				target: vault.vault,
				value: 0n,
				data: encodeFunctionData({
					abi: ERC4626_ABI,
					functionName: "withdraw",
					args: [shortfall, this.solver, this.solver],
				}),
			}
		}
		return null
	}

	private async submit(chain: string, calls: ERC7821Call[]): Promise<{ txHash: HexString; sponsored: boolean }> {
		if (this.userOpSender?.canSponsor(chain)) {
			const result = await this.userOpSender.trySendSponsored({
				chain,
				callData: encodeERC7821ExecuteBatch(calls),
				gas: {
					verificationGasLimit: 60_000n,
					callGasLimit: 450_000n * BigInt(calls.length),
					preVerificationGas: 150_000n,
				},
				paymasterVerificationGasLimit: 140_000n,
			})
			if (result) return { txHash: result.txHash, sponsored: true }
			this.logger.warn({ chain }, "Sponsored send unavailable, sending native tx")
		}

		const walletClient = this.clientManager.getWalletClient(chain)
		const publicClient = this.clientManager.getPublicClient(chain)
		// Batches route through the delegated account's executeBatch (as vault
		// sweeps do); single calls go straight to the target.
		const tx =
			calls.length === 1
				? await walletClient.sendTransaction({
						to: calls[0].target,
						data: calls[0].data,
						value: calls[0].value,
						chain: walletClient.chain,
					})
				: await walletClient.sendTransaction({
						to: this.solver,
						data: encodeERC7821ExecuteBatch(calls),
						value: 0n,
						chain: walletClient.chain,
					})
		const receipt = await publicClient.waitForTransactionReceipt({ hash: tx, confirmations: 1, timeout: 60_000 })
		if (receipt.status !== "success") throw new Error(`Send tx reverted: ${tx}`)
		return { txHash: tx, sponsored: false }
	}
}
