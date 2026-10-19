import { ERC20_ABI } from "@/config/abis/ERC20"
import { ERC4626_ABI } from "@/config/abis/Erc4626"
import type { VaultToml } from "@/config/filler-toml"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { paymasterReserveForToken } from "@/services/paymaster"
import { resolvePendingPermit2Approval } from "@/services/paymaster/provider/simplex"
import type { UserOpSender } from "@/services/UserOpSender"
import { type Logger , moduleLogger} from "@/services/Logger"
import { SubmittedUnconfirmedError, type SubmittedTx, type SubmittedUserOp } from "@/services/submission"
import { encodeERC7821ExecuteBatch, type ERC7821Call, type HexString } from "@hyperbridge/sdk"
import { encodeFunctionData, isAddress, maxUint256, parseUnits } from "viem"


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
		/** Only needed to size the paymaster's gas reserve; without it a send reserves nothing. */
		private readonly configService?: FillerConfigService,
		/**
		 * Whether the filler's account is EIP-7702-delegated on a chain. A native multi-call
		 * batch is addressed to the account itself, so an undelegated EOA would accept the
		 * transaction and do nothing; without this check the send is reported as done.
		 */
		private readonly isDelegated?: (chain: string) => Promise<boolean>,
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
				// A share balance can exceed what the vault will let out at the moment (a
				// liquidity-capped stataToken, a partially liquid market): `redeem(units)` then
				// reverts and takes the whole batch with it. Refuse up front with the number the
				// operator can actually send instead of submitting a tx that cannot succeed.
				const maxRedeem = (await publicClient.readContract({
					abi: ERC4626_ABI,
					address: token as HexString,
					functionName: "maxRedeem",
					args: [this.solver],
				})) as bigint
				if (units > maxRedeem) {
					throw new Error(
						`This vault will only redeem ${maxRedeem} shares right now, which is less than the ${units} requested`,
					)
				}
				redeemed = true
				calls.push({
					target: token as HexString,
					value: 0n,
					data: encodeFunctionData({ abi: ERC4626_ABI, functionName: "redeem", args: [units, to, this.solver] }),
				})
			} else {
				// Sizing the transfer to the last unit is what makes a send revert on
				// the sponsored path: the paymaster charges gas in this same token and
				// pulls it during validation, before the batch runs, so a wallet left
				// at exactly `units` is always short by that pull. Reserve for it, and
				// for the vault's own wallet floor, whichever is larger.
				const reserve =
					this.userOpSender?.canSponsor(chain) && this.configService
						? paymasterReserveForToken(chain, tokenLower, this.configService)
						: 0n
				if (balance < units + reserve) {
					const withdrawal = await this.vaultWithdrawalCall(
						chain,
						token as HexString,
						decimals,
						units - balance,
						reserve,
					)
					// A vault can only decline to top up the headroom; the transfer
					// itself still has to be funded.
					if (!withdrawal && balance < units) {
						throw new Error(`Insufficient token balance: have ${balance}, need ${units}`)
					}
					if (withdrawal) {
						redeemed = true
						calls.push(withdrawal)
					}
				}
				calls.push({
					target: token as HexString,
					value: 0n,
					data: encodeFunctionData({ abi: ERC20_ABI, functionName: "transfer", args: [to, units] }),
				})
			}
		}

		// Only the hash and mode are reported: the submission's bigints do not belong in the API response.
		const { txHash, sponsored } = await this.submitCalls(chain, calls)
		this.logger.warn({ chain, token, amount, to, redeemed, txHash, sponsored }, "Operator send executed")
		return { txHash, sponsored, redeemed }
	}

	/**
	 * A `withdraw(shortfall, solver, solver)` call against a configured vault on
	 * this chain whose underlying is the token being sent and that can cover the
	 * shortfall, or null when no vault can.
	 */
	/**
	 * A withdrawal that funds the transfer and, where the vault can afford it,
	 * leaves headroom in the wallet rather than nothing.
	 *
	 * `shortfall` is what the transfer cannot do without, and is zero or negative
	 * when the wallet already covers it. The headroom is the larger of the
	 * paymaster's gas `reserve` and the vault's `minBalance` — the first is what
	 * validation will pull out of this wallet, the second is the operator's own
	 * statement of the working balance this token keeps. A vault that can only
	 * cover the shortfall still funds the send: headroom is a preference, not a
	 * reason to refuse a transfer the operator asked for.
	 */
	private async vaultWithdrawalCall(
		chain: string,
		token: HexString,
		decimals: number,
		shortfall: bigint,
		reserve: bigint,
	): Promise<ERC7821Call | null> {
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
			const required = shortfall > 0n ? shortfall : 0n
			if (available < required) continue
			// Withdraw-only vaults declare no floor of their own, and fall back to
			// the paymaster reserve.
			const floor = vault.minBalance === undefined ? 0n : parseUnits(vault.minBalance, decimals)
			const headroom = reserve > floor ? reserve : floor
			const wanted = shortfall + headroom
			const amount = available >= wanted ? wanted : required
			if (amount <= 0n) return null
			return {
				target: vault.vault,
				value: 0n,
				data: encodeFunctionData({
					abi: ERC4626_ABI,
					functionName: "withdraw",
					args: [amount, this.solver, this.solver],
				}),
			}
		}
		return null
	}

	/**
	 * One bundler lookup for an op this sender submitted, for a caller that has to settle whether
	 * a placement landed. Null when the bundler does not know the op; throws when it could not be
	 * asked. See {@link UserOpSender.lookupUserOpReceipt}.
	 */
	async userOpReceipt(chain: string, userOpHash: HexString): Promise<{ txHash: HexString; success: boolean } | null> {
		if (!this.userOpSender) throw new Error(`No sponsored UserOp sender is configured for ${chain}`)
		return this.userOpSender.lookupUserOpReceipt(chain, userOpHash)
	}

	/**
	 * Executes a batch from the filler wallet: sponsored UserOp first, native tx
	 * when no sponsor is available. Shared by every operator-initiated outbound flow.
	 *
	 * `nonceKey` puts the sponsored op on its own EntryPoint nonce lane (default 0). Two ops on one
	 * key read the same `getNonce`, so a caller that can run alongside another key-0 sender passes
	 * its own. It has no effect on the native fallback.
	 */
	async submitCalls(
		chain: string,
		calls: ERC7821Call[],
		opts?: { nonceKey?: bigint },
	): Promise<{ txHash: HexString; sponsored: boolean; submission: SubmittedUserOp | SubmittedTx }> {
		if (this.userOpSender?.canSponsor(chain)) {
			const bootstrap = await this.permit2Bootstrap(chain)
			const batch = bootstrap ? [bootstrap, ...calls] : calls
			const result = await this.userOpSender.trySendSponsored({
				chain,
				callData: encodeERC7821ExecuteBatch(batch),
				nonceKey: opts?.nonceKey,
				gas: {
					verificationGasLimit: 60_000n,
					callGasLimit: 450_000n * BigInt(batch.length),
					preVerificationGas: 150_000n,
				},
				permitBootstrap: !!bootstrap,
			})
			if (result) return { txHash: result.txHash, sponsored: true, submission: result.userOp }
			this.logger.warn({ chain }, "Sponsored send unavailable, sending native tx")
		}

		const walletClient = this.clientManager.getWalletClient(chain)
		const publicClient = this.clientManager.getPublicClient(chain)
		// A batch goes to the account's own executeBatch (as vault sweeps do); a single call
		// goes straight to the target and works on a plain EOA.
		if (calls.length > 1 && this.isDelegated && !(await this.isDelegated(chain))) {
			throw new Error(
				`The wallet is not 7702-delegated on ${chain} (or its delegation could not be read); a batch sent to it would do nothing`,
			)
		}
		// Chosen here and sent explicitly, so the nonce is known whatever happens to the tx: a node
		// that no longer lists it proves nothing, the wallet's nonce moving past this one does.
		const nonce = await publicClient.getTransactionCount({ address: this.solver, blockTag: "pending" })
		const tx =
			calls.length === 1
				? await walletClient.sendTransaction({
						to: calls[0].target,
						data: calls[0].data,
						value: calls[0].value,
						nonce,
						chain: walletClient.chain,
					})
				: await walletClient.sendTransaction({
						to: this.solver,
						data: encodeERC7821ExecuteBatch(calls),
						value: 0n,
						nonce,
						chain: walletClient.chain,
					})
		const submittedAt = Date.now()
		let receipt: Awaited<ReturnType<typeof publicClient.waitForTransactionReceipt>>
		try {
			receipt = await publicClient.waitForTransactionReceipt({ hash: tx, confirmations: 1, timeout: 60_000 })
		} catch (error) {
			// The node accepted the tx, so any failure to read its receipt — a timeout, a dropped
			// connection — leaves its outcome unknown. The caller must not treat it as "nothing
			// happened" and send again; it may resume from the hash once the receipt is readable.
			throw new SubmittedUnconfirmedError(error instanceof Error ? error.message : String(error), chain, {
				kind: "tx",
				txHash: tx,
				nonce,
				submittedAt,
			})
		}
		// Mined and reverted is a definite failure, not an unknown outcome; keep it outside the wait.
		if (receipt.status !== "success") throw new Error(`Send tx reverted: ${tx}`)
		return { txHash: tx, sponsored: false, submission: { kind: "tx", txHash: tx, nonce, submittedAt } }
	}

	/**
	 * The `approve(Permit2, max)` to run first when the wallet has never approved Permit2 for its fee
	 * token on this chain, and that token has an EIP-2612 permit. The op then pays its gas with a permit
	 * and installs the approval, so a wallet with no native can still send; later ops use Permit2.
	 * Undefined otherwise, or when the check fails, and the send goes ahead as before.
	 */
	private async permit2Bootstrap(chain: string): Promise<ERC7821Call | undefined> {
		const paymaster = this.configService?.getSimplexPaymasterAddress(chain)
		if (!this.configService || !paymaster) return undefined
		try {
			const pending = await resolvePendingPermit2Approval(
				this.clientManager.getPublicClient(chain),
				this.solver,
				paymaster,
				chain,
				this.configService,
			)
			if (!pending?.permitCapable) return undefined
			return {
				target: pending.token,
				value: 0n,
				data: encodeFunctionData({
					abi: ERC20_ABI,
					functionName: "approve",
					args: [pending.spender, maxUint256],
				}),
			}
		} catch (error) {
			this.logger.warn({ chain, error }, "Could not check whether this send must install the Permit2 approval")
			return undefined
		}
	}
}
