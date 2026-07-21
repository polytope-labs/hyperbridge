import { encodePacked, maxUint256, erc20Abi, type PublicClient, type WalletClient } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import type { FillerConfigService } from "@/services/FillerConfigService"
import {
	RECOMMENDED_AMOUNT_USD,
	THRESHOLD_USD,
	VERIFICATION_GAS_LIMIT_PERMIT,
	VERIFICATION_GAS_LIMIT_APPROVE,
	POST_OP_GAS_LIMIT,
	type PaymasterResult,
} from "../types"
import { signEip2612Permit } from "../permit"

interface TokenOption {
	address: HexString
	decimals: number
}

/**
 * Builds the paymaster fields for a PackedUserOperation using the SimplexPaymaster.
 *
 * Selects the first configured stablecoin (USDC, then USDT) with a balance of at
 * least one token, then:
 * - if the token supports EIP-2612 permit, signs a permit and encodes PERMIT mode
 *   (0x00) so the paymaster executes it during validation
 * - otherwise ensures a capped on-chain approval exists and encodes APPROVE mode (0x01)
 *
 * Returns null when the solver has no balance in any configured token — the caller
 * decides whether to fall back to another paymaster or to the EntryPoint deposit.
 */
export async function buildSimplexPaymasterData(
	client: PublicClient,
	walletClient: WalletClient,
	signer: { signTypedData: (typedData: unknown, chainId?: number) => Promise<HexString> },
	solverAccount: HexString,
	paymasterAddress: HexString,
	chain: string,
	configService: FillerConfigService,
	forceApproveMode = false,
): Promise<(PaymasterResult & { token: HexString }) | null> {
	const chainId = configService.getChainId(chain)

	const tokens: TokenOption[] = []

	const usdcAddress = configService.getUsdcAsset(chain)
	if (isConfigured(usdcAddress)) {
		tokens.push({ address: usdcAddress, decimals: configService.getUsdcDecimals(chain) })
	}

	const usdtAddress = configService.getUsdtAsset(chain)
	if (isConfigured(usdtAddress)) {
		tokens.push({ address: usdtAddress, decimals: configService.getUsdtDecimals(chain) })
	}

	const selected = await selectToken(client, solverAccount, tokens)
	if (!selected) {
		return null
	}

	const { address: tokenAddress, decimals: tokenDecimals } = selected
	const recommended = RECOMMENDED_AMOUNT_USD * 10n ** BigInt(tokenDecimals)

	const hasPermit = !forceApproveMode && (await tokenSupportsPermit(client, tokenAddress))

	if (hasPermit) {
		const pm = await buildPermitMode(
			client,
			signer,
			solverAccount,
			paymasterAddress,
			tokenAddress,
			recommended,
			chainId,
		)
		return { ...pm, token: tokenAddress }
	}

	await ensureCappedApproval(client, walletClient, solverAccount, paymasterAddress, tokenAddress, tokenDecimals)

	const paymasterData = encodePacked(["uint8", "address"], [1, tokenAddress]) as HexString

	return {
		paymaster: paymasterAddress,
		paymasterData,
		paymasterVerificationGasLimit: VERIFICATION_GAS_LIMIT_APPROVE,
		paymasterPostOpGasLimit: POST_OP_GAS_LIMIT,
		token: tokenAddress,
	}
}

// ── Helpers ──────────────────────────────────────────────────────────

/** Missing assets come back from the config service as the literal "0x". */
function isConfigured(address: HexString): boolean {
	return !!address && address !== "0x" && address !== "0x0000000000000000000000000000000000000000"
}

async function selectToken(
	client: PublicClient,
	solverAccount: HexString,
	tokens: TokenOption[],
): Promise<TokenOption | null> {
	for (const token of tokens) {
		const balance = (await client.readContract({
			address: token.address,
			abi: erc20Abi,
			functionName: "balanceOf",
			args: [solverAccount],
		})) as bigint

		const minBalance = 10n ** BigInt(token.decimals)
		if (balance >= minBalance) {
			return token
		}
	}
	return null
}

async function buildPermitMode(
	client: PublicClient,
	signer: { signTypedData: (typedData: unknown, chainId?: number) => Promise<HexString> },
	solverAccount: HexString,
	paymasterAddress: HexString,
	tokenAddress: HexString,
	permitAmount: bigint,
	chainId: number,
): Promise<PaymasterResult> {
	// An existing allowance makes the permit redundant — use the cheaper approve mode.
	const existingAllowance = (await client.readContract({
		address: tokenAddress,
		abi: erc20Abi,
		functionName: "allowance",
		args: [solverAccount, paymasterAddress],
	})) as bigint

	if (existingAllowance >= permitAmount) {
		const paymasterData = encodePacked(["uint8", "address"], [1, tokenAddress]) as HexString
		return {
			paymaster: paymasterAddress,
			paymasterData,
			paymasterVerificationGasLimit: VERIFICATION_GAS_LIMIT_APPROVE,
			paymasterPostOpGasLimit: POST_OP_GAS_LIMIT,
		}
	}

	const permitSignature = await signEip2612Permit(
		client,
		signer,
		solverAccount,
		paymasterAddress,
		tokenAddress,
		permitAmount,
		chainId,
	)

	const r = `0x${permitSignature.slice(2, 66)}` as HexString
	const s = `0x${permitSignature.slice(66, 130)}` as HexString
	const v = parseInt(permitSignature.slice(130, 132), 16)

	// mode(1) + token(20) + permitAmount(32) + deadline(32) + v(1) + r(32) + s(32) = 150 bytes,
	// matching SimplexPaymaster._executePermit. Deadline is maxUint256 because paymasters
	// cannot read block.timestamp under ERC-4337 validation rules.
	const paymasterData = encodePacked(
		["uint8", "address", "uint256", "uint256", "uint8", "bytes32", "bytes32"],
		[0, tokenAddress, permitAmount, maxUint256, v, r, s],
	) as HexString

	return {
		paymaster: paymasterAddress,
		paymasterData,
		paymasterVerificationGasLimit: VERIFICATION_GAS_LIMIT_PERMIT,
		paymasterPostOpGasLimit: POST_OP_GAS_LIMIT,
	}
}

async function ensureCappedApproval(
	client: PublicClient,
	walletClient: WalletClient,
	solverAccount: HexString,
	paymasterAddress: HexString,
	tokenAddress: HexString,
	tokenDecimals: number,
): Promise<void> {
	const currentAllowance = (await client.readContract({
		address: tokenAddress,
		abi: erc20Abi,
		functionName: "allowance",
		args: [solverAccount, paymasterAddress],
	})) as bigint

	const threshold = THRESHOLD_USD * 10n ** BigInt(tokenDecimals)

	if (currentAllowance >= threshold) {
		return
	}

	const approvalAmount = RECOMMENDED_AMOUNT_USD * 10n ** BigInt(tokenDecimals)

	const hash = await walletClient.writeContract({
		address: tokenAddress,
		abi: erc20Abi,
		functionName: "approve",
		args: [paymasterAddress, approvalAmount],
		chain: walletClient.chain,
		account: walletClient.account!,
	})

	await client.waitForTransactionReceipt({ hash, confirmations: 1 })
}

/** Probes for EIP-2612 support via the version() getter permit tokens expose. */
async function tokenSupportsPermit(client: PublicClient, tokenAddress: HexString): Promise<boolean> {
	try {
		await client.readContract({
			address: tokenAddress,
			abi: [
				{
					inputs: [],
					name: "version",
					outputs: [{ type: "string" }],
					stateMutability: "view",
					type: "function",
				},
			] as const,
			functionName: "version",
		})
		return true
	} catch {
		return false
	}
}
