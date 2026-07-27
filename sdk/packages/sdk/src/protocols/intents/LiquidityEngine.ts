import { LATEST_PHANTOM_ORDER_LIQUIDITY_SNAPSHOT, LIQUIDITY_PROVIDER_BALANCES } from "@/queries"
import { Chains } from "@/configs/chain"
import type { ChainConfigService } from "@/configs/ChainConfigService"
import type { AvailableLiquidityByChain, AvailableLiquiditySnapshot, HexString, IndexerQueryClient } from "@/types"
import { dateStringtoTimestamp, normalizeEvmAddress } from "@/utils"
import { formatUnits } from "viem"

interface LiquidityProviderBalancesResponse {
	liquidityProviderBalances: {
		aggregates: {
			sum: { balance: string | null }
			distinctCount: { providerId: string }
		} | null
		groupedAggregates: Array<{
			keys: string[]
			sum: { balance: string | null }
			distinctCount: { providerId: string }
		}>
	}
}

interface PhantomOrderLiquiditySnapshotsResponse {
	phantomOrderPriceSnapshots: {
		nodes: Array<{
			commitment: string
			tokenA: string
			tokenB: string
			snapshotTime: string
		}>
	}
}

const COMMITMENT_PATTERN = /^0x[0-9a-f]{64}$/i

/**
 * Owns Phantom liquidity snapshot retrieval and aggregation from Nexus.
 *
 * The indexer performs balance and distinct-provider aggregation; this class
 * validates and maps those aggregate results into the SDK response shape.
 */
export class LiquidityEngine {
	/**
	 * @param queryClient - Nexus GraphQL client attached to the gateway.
	 * @param chainConfigService - Resolves token decimals for formatted results.
	 */
	constructor(
		private readonly queryClient: IndexerQueryClient,
		private readonly chainConfigService: ChainConfigService,
	) {}

	/**
	 * Retrieves the newest directional Phantom snapshot for a pair and its
	 * indexed output-token liquidity.
	 *
	 * Nexus filters balances to the canonical output token, then aggregates them
	 * overall and by chain. The returned amounts are decimal strings: the total
	 * uses the canonical Base output-token decimals, while each chain group uses
	 * that chain's token decimals. They describe `snapshotTime`, not live
	 * reservations or fill guarantees.
	 *
	 * @param params - Canonical Phantom-market input and output token addresses.
	 * @returns The latest snapshot, or `undefined` if Nexus has no snapshot for
	 * the directional pair.
	 * @throws {InvalidAvailableLiquiditySnapshotError} If Nexus returns malformed
	 * or internally inconsistent snapshot data.
	 */
	async getAvailableLiquiditySnapshot(params: {
		tokenIn: HexString
		tokenOut: HexString
	}): Promise<AvailableLiquiditySnapshot | undefined> {
		const tokenIn = normalizeEvmAddress(params.tokenIn, "tokenIn")
		const tokenOut = normalizeEvmAddress(params.tokenOut, "tokenOut")
		const response = await this.queryClient.request<PhantomOrderLiquiditySnapshotsResponse>(
			LATEST_PHANTOM_ORDER_LIQUIDITY_SNAPSHOT,
			{ tokenA: tokenIn, tokenB: tokenOut },
		)
		const node = response?.phantomOrderPriceSnapshots?.nodes?.[0]
		if (!node) return

		const commitment = node.commitment.toLowerCase()
		if (!COMMITMENT_PATTERN.test(commitment)) {
			throw new InvalidAvailableLiquiditySnapshotError(commitment || "<missing>", "commitment is not bytes32 hex")
		}
		if (node.tokenA.toLowerCase() !== tokenIn || node.tokenB.toLowerCase() !== tokenOut) {
			throw new InvalidAvailableLiquiditySnapshotError(commitment, "snapshot token pair does not match the query")
		}

		const snapshotTime = new Date(dateStringtoTimestamp(node.snapshotTime))
		if (Number.isNaN(snapshotTime.getTime())) {
			throw new InvalidAvailableLiquiditySnapshotError(commitment, "snapshotTime is invalid")
		}

		const { totalLiquidity, providerCount, liquidityByChain } = await this.querySnapshotLiquidityAggregates({
			commitment,
			tokenAddress: tokenOut,
		})

		return {
			totalLiquidity: this.formatLiquidity(totalLiquidity, Chains.BASE_MAINNET, tokenOut, commitment),
			providerCount,
			tokenAddress: tokenOut,
			snapshotTime,
			liquidityByChain: liquidityByChain.map((group) => ({
				...group,
				totalLiquidity: this.formatLiquidity(group.totalLiquidity, group.chain, group.tokenAddress, commitment),
			})),
		}
	}

	/**
	 * Requests server-side sums and distinct provider counts for one immutable
	 * snapshot/output-token pair, including chain-level aggregate groups.
	 *
	 * `commitment` uniquely identifies the selected snapshot
	 */
	private async querySnapshotLiquidityAggregates(params: { commitment: string; tokenAddress: HexString }): Promise<{
		totalLiquidity: bigint
		providerCount: number
		liquidityByChain: RawAvailableLiquidityByChain[]
	}> {
		const response = await this.queryClient.request<LiquidityProviderBalancesResponse>(
			LIQUIDITY_PROVIDER_BALANCES,
			{ commitment: params.commitment, tokenAddress: params.tokenAddress },
		)
		const connection = response?.liquidityProviderBalances
		const aggregates = connection?.aggregates
		if (!connection || !aggregates) {
			throw new InvalidAvailableLiquiditySnapshotError(
				params.commitment,
				"liquidityProviderBalances aggregates are missing",
			)
		}

		const totalLiquidity = parseSnapshotBigInt(aggregates.sum.balance ?? "0", params.commitment, "total balance")
		const providerCount = parseProviderCount(aggregates.distinctCount.providerId, params.commitment, "total")
		const liquidityByChain = connection.groupedAggregates.map((group, index) => {
			const [chain, tokenAddress] = group.keys
			if (!chain?.trim() || !tokenAddress) {
				throw new InvalidAvailableLiquiditySnapshotError(
					params.commitment,
					`liquidity group ${index} has invalid keys`,
				)
			}
			const normalizedTokenAddress = normalizeIndexedLiquidityAddress(
				tokenAddress,
				params.commitment,
				`liquidity group ${index} tokenAddress`,
			)
			if (normalizedTokenAddress !== params.tokenAddress) {
				throw new InvalidAvailableLiquiditySnapshotError(
					params.commitment,
					`liquidity group ${index} tokenAddress does not match the snapshot output token`,
				)
			}

			return {
				chain: chain.trim(),
				tokenAddress: normalizedTokenAddress,
				totalLiquidity: parseSnapshotBigInt(
					group.sum.balance ?? "0",
					params.commitment,
					`liquidity group ${index} balance`,
				),
				providerCount: parseProviderCount(
					group.distinctCount.providerId,
					params.commitment,
					`liquidity group ${index}`,
				),
			}
		})
		const groupedTotal = liquidityByChain.reduce((sum, group) => sum + group.totalLiquidity, 0n)
		if (groupedTotal !== totalLiquidity) {
			throw new InvalidAvailableLiquiditySnapshotError(
				params.commitment,
				"grouped liquidity does not match the total liquidity",
			)
		}

		return { totalLiquidity, providerCount, liquidityByChain }
	}

	/** Formats a raw amount using the configured decimals for its chain/token. */
	private formatLiquidity(amount: bigint, chain: string, tokenAddress: HexString, commitment: string): string {
		const decimals = this.chainConfigService.getAssetMetadataByAddress(chain, tokenAddress)?.decimals
		if (decimals === undefined) {
			throw new InvalidAvailableLiquiditySnapshotError(
				commitment,
				`token decimals are not configured for ${tokenAddress} on ${chain}`,
			)
		}
		return formatUnits(amount, decimals)
	}
}

type RawAvailableLiquidityByChain = Omit<AvailableLiquidityByChain, "totalLiquidity"> & {
	totalLiquidity: bigint
}

class InvalidAvailableLiquiditySnapshotError extends Error {
	/** Creates an error that identifies the invalid snapshot and field/reason. */
	constructor(commitment: string, reason: string) {
		super(`Invalid available-liquidity snapshot ${commitment}: ${reason}`)
		this.name = "InvalidAvailableLiquiditySnapshotError"
	}
}

/** Parses a non-negative smallest-unit amount returned by the indexer. */
function parseSnapshotBigInt(value: string, commitment: string, field: string): bigint {
	try {
		const amount = BigInt(value)
		if (amount < 0n) {
			throw new InvalidAvailableLiquiditySnapshotError(commitment, `${field} cannot be negative`)
		}
		return amount
	} catch (error) {
		if (error instanceof InvalidAvailableLiquiditySnapshotError) throw error
		throw new InvalidAvailableLiquiditySnapshotError(commitment, `${field} is not an integer`)
	}
}

/** Parses a safe, non-negative distinct-provider count returned by the indexer. */
function parseProviderCount(value: string, commitment: string, field: string): number {
	const count = Number(value)
	if (!Number.isSafeInteger(count) || count < 0) {
		throw new InvalidAvailableLiquiditySnapshotError(commitment, `${field} provider count is invalid`)
	}
	return count
}

/** Validates and canonicalizes an EVM address from a Nexus aggregate group. */
function normalizeIndexedLiquidityAddress(address: string, commitment: string, field: string): HexString {
	try {
		return normalizeEvmAddress(address, field)
	} catch {
		throw new InvalidAvailableLiquiditySnapshotError(commitment, `${field} is not a valid EVM address`)
	}
}
