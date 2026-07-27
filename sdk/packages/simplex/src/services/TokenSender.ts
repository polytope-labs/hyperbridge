import { ERC20_ABI } from "@/config/abis/ERC20"
import { ERC4626_ABI } from "@/config/abis/Erc4626"
import type { VaultToml } from "@/config/filler-toml"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { UserOpSender } from "@/services/UserOpSender"
import { getLogger } from "@/services/Logger"
import { encodeERC7821ExecuteBatch, type ERC7821Call, type HexString } from "@hyperbridge/sdk"
import { encodeFunctionData, isAddress, parseUnits } from "viem"

const logger = getLogger("token-sender")

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
	/** Set when the token was a configured vault's shares: the send redeemed them and delivered the underlying. */
	redeemed: boolean
}

/**
 * Operator-initiated outbound transfers from the filler wallet. Uses the same
 * sponsored UserOp path as fills (gas paid by the paymaster) with a native-tx
 * fallback. Sending a configured ERC-4626 vault's share token issues
 * `redeem(shares, recipient, solver)` — redeem and delivery of the underlying
 * in one atomic call.
 */
export class TokenSender {
	constructor(
		private readonly clientManager: ChainClientManager,
		private readonly solver: HexString,
		/** Live view of the configured vaults so runtime vault edits are honored. */
		private readonly getVaults: () => VaultToml[],
		private readonly userOpSender?: UserOpSender,
	) {}

	async send(params: SendTokenParams): Promise<SendTokenResult> {
		const { chain, token, amount, to } = params
		if (!isAddress(to)) throw new Error(`Invalid recipient address: ${to}`)
		const publicClient = this.clientManager.getPublicClient(chain)

		let call: ERC7821Call
		let redeemed = false
		if (token === "native") {
			const value = parseUnits(amount, 18)
			if (value <= 0n) throw new Error("Amount must be greater than 0")
			const balance = await publicClient.getBalance({ address: this.solver })
			if (balance < value) throw new Error(`Insufficient native balance: have ${balance}, need ${value}`)
			call = { target: to, value, data: "0x" }
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
			if (balance < units) throw new Error(`Insufficient token balance: have ${balance}, need ${units}`)

			const isVaultShare = this.getVaults().some(
				(v) => v.chain === chain && v.vault.toLowerCase() === tokenLower,
			)
			redeemed = isVaultShare
			call = {
				target: token as HexString,
				value: 0n,
				data: isVaultShare
					? encodeFunctionData({ abi: ERC4626_ABI, functionName: "redeem", args: [units, to, this.solver] })
					: encodeFunctionData({ abi: ERC20_ABI, functionName: "transfer", args: [to, units] }),
			}
		}

		const result = await this.submit(chain, call)
		logger.warn({ chain, token, amount, to, redeemed, ...result }, "Operator send executed")
		return { ...result, redeemed }
	}

	private async submit(chain: string, call: ERC7821Call): Promise<{ txHash: HexString; sponsored: boolean }> {
		if (this.userOpSender?.canSponsor(chain)) {
			const result = await this.userOpSender.trySendSponsored({
				chain,
				callData: encodeERC7821ExecuteBatch([call]),
				gas: {
					verificationGasLimit: 60_000n,
					callGasLimit: 450_000n,
					preVerificationGas: 150_000n,
				},
				paymasterVerificationGasLimit: 140_000n,
			})
			if (result) return { txHash: result.txHash, sponsored: true }
			logger.warn({ chain }, "Sponsored send unavailable, sending native tx")
		}

		const walletClient = this.clientManager.getWalletClient(chain)
		const publicClient = this.clientManager.getPublicClient(chain)
		const tx = await walletClient.sendTransaction({
			to: call.target,
			data: call.data,
			value: call.value,
			chain: walletClient.chain,
		})
		const receipt = await publicClient.waitForTransactionReceipt({ hash: tx, confirmations: 1, timeout: 60_000 })
		if (receipt.status !== "success") throw new Error(`Send tx reverted: ${tx}`)
		return { txHash: tx, sponsored: false }
	}
}
