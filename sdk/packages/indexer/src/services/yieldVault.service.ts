import { ethers } from "ethers"

import Erc4626Abi from "@/configs/abis/Erc4626.abi.json"
import {
	VaultLedgerEvent,
	VaultLedgerEventType,
	VaultLpPosition,
	VaultPositionSnapshot,
	VaultSnapshot,
} from "@/configs/src/types"
import { YIELD_VAULT_ADDRESSES } from "@/yield-vault-addresses"
import { timestampToDate } from "@/utils/date.helpers"

const SECONDS_PER_DAY = 86400n

// Page size for streaming an LP set out of the store during a snapshot. The store caps a single
// getByFields page, so positions are read in batches and snapshotted per page.
const LP_PAGE_SIZE = 100

/** A single supported vault on a chain, paired with the underlying token it wraps. */
export interface ConfiguredVault {
	vault: string
	underlyingToken: string
}

/** Inputs for recording one ERC-4626 Deposit or Withdraw event. */
export interface VaultLedgerInput {
	chain: string
	/** The vault address the event was emitted by (event.address). */
	vault: string
	/** The share owner whose principal moved (Deposit.owner / Withdraw.owner). */
	lp: string
	/** The address that initiated the call (Deposit.sender / Withdraw.sender). */
	caller: string
	/** For withdrawals, who received the assets (Withdraw.receiver). Omitted for deposits. */
	receiver?: string
	assets: bigint
	shares: bigint
	eventType: VaultLedgerEventType
	blockNumber: bigint
	transactionHash: string
	logIndex: number
	/** Block timestamp in UNIX seconds. */
	timestamp: bigint
}

export class YieldVaultService {
	/**
	 * Resolve the underlying token a vault wraps from the generated config, case-insensitively.
	 * Returns undefined if the vault is not configured for the chain (e.g. a stale datasource).
	 */
	static underlyingTokenFor(chain: string, vault: string): string | undefined {
		const byToken = YIELD_VAULT_ADDRESSES[chain]
		if (!byToken) return undefined
		const target = vault.toLowerCase()
		for (const [token, vaults] of Object.entries(byToken)) {
			if (vaults.some((v) => v.toLowerCase() === target)) return token.toLowerCase()
		}
		return undefined
	}

	/** All supported vaults configured for a chain, lowercased. */
	static configuredVaults(chain: string): ConfiguredVault[] {
		const byToken = YIELD_VAULT_ADDRESSES[chain]
		if (!byToken) return []
		return Object.entries(byToken).flatMap(([token, vaults]) =>
			vaults.map((vault) => ({ vault: vault.toLowerCase(), underlyingToken: token.toLowerCase() })),
		)
	}

	/**
	 * Persist one ledger event and fold it into the LP's running position. The position's net
	 * principal (deposited - withdrawn) is the baseline the daily snapshot measures yield against.
	 */
	static async recordLedger(input: VaultLedgerInput): Promise<void> {
		const vault = input.vault.toLowerCase()
		const lp = input.lp.toLowerCase()
		const underlyingToken = this.underlyingTokenFor(input.chain, vault)
		if (!underlyingToken) {
			logger.warn(`[yield-vault] Unconfigured vault ${vault} on ${input.chain}, skipping ledger event`)
			return
		}

		// Idempotency guard: the position is folded with += / -=, so applying the same event twice would
		// corrupt principal. Skip if this exact log was already recorded. (A reorg rolls back both the
		// ledger row and the position together under historical indexing, so legitimate reprocessing
		// still re-applies cleanly — this only blocks true duplicate delivery.)
		const ledgerId = `${input.chain}-${input.transactionHash}-${input.logIndex}`
		if (await VaultLedgerEvent.get(ledgerId)) {
			logger.debug(`[yield-vault] Ledger event ${ledgerId} already recorded, skipping`)
			return
		}

		const eventTime = timestampToDate(input.timestamp)

		await VaultLedgerEvent.create({
			id: ledgerId,
			chain: input.chain,
			vault,
			underlyingToken,
			lp,
			caller: input.caller.toLowerCase(),
			receiver: input.receiver?.toLowerCase(),
			eventType: input.eventType,
			assets: input.assets,
			shares: input.shares,
			blockNumber: input.blockNumber,
			transactionHash: input.transactionHash,
			timestamp: eventTime,
		}).save()

		const positionId = `${input.chain}-${vault}-${lp}`
		let position = await VaultLpPosition.get(positionId)
		if (!position) {
			position = VaultLpPosition.create({
				id: positionId,
				chain: input.chain,
				vault,
				underlyingToken,
				lp,
				shares: 0n,
				totalAssetsDeposited: 0n,
				totalAssetsWithdrawn: 0n,
				depositCount: 0,
				withdrawCount: 0,
				createdAt: eventTime,
				lastUpdatedAt: eventTime,
			})
		}

		if (input.eventType === VaultLedgerEventType.DEPOSIT) {
			position.totalAssetsDeposited += input.assets
			position.shares += input.shares
			position.depositCount += 1
		} else {
			position.totalAssetsWithdrawn += input.assets
			position.shares -= input.shares
			position.withdrawCount += 1
		}
		position.lastUpdatedAt = eventTime

		await position.save()
	}

	/**
	 * Take the daily snapshot for every configured vault on a chain: a vault-level snapshot
	 * (totalAssets / totalShares / assetsPerShare) plus one per-LP snapshot pricing each LP's live
	 * share balance into assets via `convertToAssets`. Idempotent per UTC day — a vault whose
	 * day-bucket snapshot already exists is skipped before any RPC, so re-attempts are cheap.
	 */
	static async snapshotChain(chain: string, blockNumber: bigint, timestamp: bigint): Promise<void> {
		const dayStart = (timestamp / SECONDS_PER_DAY) * SECONDS_PER_DAY
		const snapshotTime = timestampToDate(timestamp)

		for (const { vault, underlyingToken } of this.configuredVaults(chain)) {
			const vaultSnapshotId = `${chain}-${vault}-${dayStart}`
			if (await VaultSnapshot.get(vaultSnapshotId)) continue

			const contract = new ethers.Contract(vault, Erc4626Abi, api as any)

			let totalAssets: bigint
			let totalShares: bigint
			let assetsPerShare: bigint
			try {
				const [assetsRaw, supplyRaw, decimalsRaw] = await Promise.all([
					contract.totalAssets(),
					contract.totalSupply(),
					contract.decimals(),
				])
				const oneShare = ethers.BigNumber.from(10).pow(decimalsRaw)
				const perShareRaw = await contract.convertToAssets(oneShare)
				totalAssets = BigInt(assetsRaw.toString())
				totalShares = BigInt(supplyRaw.toString())
				assetsPerShare = BigInt(perShareRaw.toString())
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error)
				logger.error(`[yield-vault] Vault read failed for ${vault} on ${chain}: ${message}`)
				continue
			}

			// Snapshot the LPs first, then write the vault-level row last. The vault snapshot is the
			// per-day completion gate (checked at the top), so persisting it only after the LP loop
			// finishes means a mid-loop failure leaves the gate open and the next run retries — the
			// per-LP dedup skips LPs already done, so the remainder is filled in rather than lost.
			await this.snapshotLpPositions(chain, vault, underlyingToken, contract, dayStart, blockNumber, snapshotTime)

			await VaultSnapshot.create({
				id: vaultSnapshotId,
				chain,
				vault,
				underlyingToken,
				dayStartTimestamp: dayStart,
				totalAssets,
				totalShares,
				assetsPerShare,
				blockNumber,
				snapshotTime,
			}).save()
		}
	}

	private static async snapshotLpPositions(
		chain: string,
		vault: string,
		underlyingToken: string,
		contract: ethers.Contract,
		dayStart: bigint,
		blockNumber: bigint,
		snapshotTime: Date,
	): Promise<void> {
		let offset = 0
		// Stream the vault's LPs page by page so a vault with many positions doesn't load them all at once.
		for (;;) {
			const positions = await VaultLpPosition.getByFields(
				[
					["chain", "=", chain],
					["vault", "=", vault],
				],
				// Order by the primary key so offset paging is stable and never skips an LP across pages.
				{ limit: LP_PAGE_SIZE, offset, orderBy: "id", orderDirection: "ASC" },
			)
			if (positions.length === 0) break

			await Promise.all(
				positions.map((position) =>
					this.snapshotLpPosition(
						position,
						chain,
						vault,
						underlyingToken,
						contract,
						dayStart,
						blockNumber,
						snapshotTime,
					),
				),
			)

			if (positions.length < LP_PAGE_SIZE) break
			offset += LP_PAGE_SIZE
		}
	}

	/** Price one LP's live share balance into assets and persist its daily snapshot. */
	private static async snapshotLpPosition(
		position: VaultLpPosition,
		chain: string,
		vault: string,
		underlyingToken: string,
		contract: ethers.Contract,
		dayStart: bigint,
		blockNumber: bigint,
		snapshotTime: Date,
	): Promise<void> {
		const snapshotId = `${chain}-${vault}-${position.lp}-${dayStart}`
		if (await VaultPositionSnapshot.get(snapshotId)) return

		let shares: bigint
		let assetValue: bigint
		try {
			const sharesRaw = await contract.balanceOf(position.lp)
			const assetsRaw = await contract.convertToAssets(sharesRaw)
			shares = BigInt(sharesRaw.toString())
			assetValue = BigInt(assetsRaw.toString())
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error)
			logger.error(`[yield-vault] LP read failed for ${position.lp} on ${vault}: ${message}`)
			return
		}

		const netPrincipal = position.totalAssetsDeposited - position.totalAssetsWithdrawn
		const yieldEarned = assetValue - netPrincipal
		if (yieldEarned < 0n) {
			// Expected transiently (a fresh deposit before yield accrues, or a vesting/loss vault), but
			// a persistent negative can also signal ledger/contract drift, so surface it.
			logger.warn(
				`[yield-vault] Negative yield for ${chain}:${vault}:${position.lp} ` +
					`(assetValue=${assetValue}, netPrincipal=${netPrincipal})`,
			)
		}

		await VaultPositionSnapshot.create({
			id: snapshotId,
			chain,
			vault,
			underlyingToken,
			lp: position.lp,
			dayStartTimestamp: dayStart,
			shares,
			assetValue,
			netPrincipal,
			yieldEarned,
			blockNumber,
			snapshotTime,
		}).save()
	}
}
