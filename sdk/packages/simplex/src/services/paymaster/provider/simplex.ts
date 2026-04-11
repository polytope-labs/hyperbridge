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

// ── Types ────────────────────────────────────────────────────────────

interface TokenOption {
	address: HexString
	decimals: number
}

// ── Main entry point ─────────────────────────────────────────────────

/**
 * Self-contained paymaster data builder for SimplexPaymaster.
 *
 * Resolves USDC/USDT token addresses from the config service, then:
 * 1. Selects the first token with ≥1 balance (priority: USDC → USDT)
 * 2. If the token supports EIP-2612 permit → signs permit, encodes PERMIT mode
 * 3. If not → ensures a capped on-chain approval exists, encodes APPROVE mode
 *
 * @param forceApproveMode  When true, skips permit detection and always uses APPROVE mode.
 *                          Useful for delegation UserOps where ERC-1271 may not be ready yet.
 */
export async function buildPaymasterData(
	client: PublicClient,
	walletClient: WalletClient,
	signer: { signTypedData: (typedData: unknown, chainId?: number) => Promise<HexString> },
	solverAccount: HexString,
	paymasterAddress: HexString,
	chain: string,
	configService: FillerConfigService,
	forceApproveMode = false,
): Promise<PaymasterResult> {
	const chainId = configService.getChainId(chain)

	const tokens: TokenOption[] = []

	const usdcAddress = configService.getUsdcAsset(chain)
	const usdcDecimals = configService.getUsdcDecimals(chain)
	if (usdcAddress) {
		tokens.push({ address: usdcAddress, decimals: usdcDecimals })
	}

	const usdtAddress = configService.getUsdtAsset(chain)
	const usdtDecimals = configService.getUsdtDecimals(chain)
	if (usdtAddress) {
		tokens.push({ address: usdtAddress, decimals: usdtDecimals })
	}

	// ── 1. Select token by balance ───────────────────────────────────
	const selected = await selectToken(client, solverAccount, tokens)
	if (!selected) {
		throw new Error(
			`SimplexPaymaster: solver ${solverAccount} has insufficient balance in all configured tokens ` +
				`(${tokens.map((t) => t.address).join(", ")}). Need ≥1 token in at least one.`,
		)
	}

	const { address: tokenAddress, decimals: tokenDecimals } = selected
	const recommended = RECOMMENDED_AMOUNT_USD * 10n ** BigInt(tokenDecimals)

	// ── 2. Check permit support ──────────────────────────────────────
	const hasPermit = !forceApproveMode && (await tokenSupportsPermit(client, tokenAddress))

	if (hasPermit) {
		return buildPermitMode(client, signer, solverAccount, paymasterAddress, tokenAddress, recommended, chainId)
	}

	// ── 3. Approve mode: ensure capped allowance ─────────────────────
	await ensureCappedApproval(client, walletClient, solverAccount, paymasterAddress, tokenAddress, tokenDecimals)

	const paymasterData = encodePacked(["uint8", "address"], [1, tokenAddress]) as HexString

	return {
		paymaster: paymasterAddress,
		paymasterData,
		paymasterVerificationGasLimit: VERIFICATION_GAS_LIMIT_APPROVE,
		paymasterPostOpGasLimit: POST_OP_GAS_LIMIT,
	}
}

// ── Token selection ──────────────────────────────────────────────────

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

// ── Permit mode ──────────────────────────────────────────────────────

async function buildPermitMode(
	client: PublicClient,
	signer: { signTypedData: (typedData: unknown, chainId?: number) => Promise<HexString> },
	solverAccount: HexString,
	paymasterAddress: HexString,
	tokenAddress: HexString,
	permitAmount: bigint,
	chainId: number,
): Promise<PaymasterResult> {
	// Check existing allowance — skip permit if already sufficient
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

// ── Approve mode: capped on-chain approval ───────────────────────────

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

// ── EIP-2612 support detection ───────────────────────────────────────

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
			],
			functionName: "version",
		})
		return true
	} catch {
		return false
	}
}
