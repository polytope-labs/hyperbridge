import Decimal from "decimal.js"
import { ethers } from "ethers"
import type { Hex } from "viem"
import { keccak256, encodeAbiParameters, toHex, type AbiParameter } from "viem"
import { bytes32ToBytes20 } from "@/utils/transfer.helpers"
import IntentGatewayV3Abi from "@/configs/abis/IntentGatewayV3.abi.json"

// Derive the Order tuple type from the canonical ABI so this stays in sync with
// the on-chain struct (which the contract hashes via keccak256(abi.encode(order))).
const ORDER_TUPLE_TYPE: AbiParameter = (() => {
	const placeOrder = (IntentGatewayV3Abi as readonly any[]).find(
		(item) => item.type === "function" && item.name === "placeOrder",
	)
	const order = placeOrder?.inputs?.[0]
	if (!order) throw new Error("placeOrder.order not found in IntentGatewayV3 ABI")
	return order as AbiParameter
})()

import { OrderStatus, PendingStatusMetadata, ProtocolParticipantType, PointsActivityType } from "@/configs/src/types"
import { ERC6160Ext20Abi__factory } from "@/configs/src/types/contracts"
import { IOrderV3 as OrderV3Placed } from "@/configs/src/types/models/IOrderV3"
import { IOrderV3StatusMetadata } from "@/configs/src/types/models/IOrderV3StatusMetadata"
import { IOrderV3PredispatchAsset } from "@/configs/src/types/models/IOrderV3PredispatchAsset"
import { IOrderV3InputAsset } from "@/configs/src/types/models/IOrderV3InputAsset"
import { IOrderV3OutputAsset } from "@/configs/src/types/models/IOrderV3OutputAsset"
import { IOrderV3PartialFill } from "@/configs/src/types/models/IOrderV3PartialFill"
import { IOrderV3PartialFillInputAsset } from "@/configs/src/types/models/IOrderV3PartialFillInputAsset"
import { IOrderV3PartialFillOutputAsset } from "@/configs/src/types/models/IOrderV3PartialFillOutputAsset"
import { IOrderV3Fill } from "@/configs/src/types/models/IOrderV3Fill"
import { IOrderV3FillInputAsset } from "@/configs/src/types/models/IOrderV3FillInputAsset"
import { IOrderV3FillOutputAsset } from "@/configs/src/types/models/IOrderV3FillOutputAsset"
import { IOrderV3EscrowRelease } from "@/configs/src/types/models/IOrderV3EscrowRelease"
import { IOrderV3EscrowReleaseToken } from "@/configs/src/types/models/IOrderV3EscrowReleaseToken"
import { IOrderV3EscrowRefund } from "@/configs/src/types/models/IOrderV3EscrowRefund"
import { IOrderV3EscrowRefundToken } from "@/configs/src/types/models/IOrderV3EscrowRefundToken"
import { IntentGatewayTokenVolume } from "@/configs/src/types/models/IntentGatewayTokenVolume"
import { CumulativeIntentGatewayVolumeUSD } from "@/configs/src/types/models/CumulativeIntentGatewayVolumeUSD"
import { PhantomOrderPriceSnapshot } from "@/configs/src/types/models/PhantomOrderPriceSnapshot"
import { timestampToDate } from "@/utils/date.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { BASE_CNGN } from "@/addresses/fx-tokens.addresses"

import { PointsService } from "./points.service"
import { VolumeService, toScaledUsd } from "./volume.service"
import PriceHelper from "@/utils/price.helpers"
import stringify from "safe-stable-stringify"
import { getOrCreateUser } from "./userActivity.services"
export interface TokenInfo {
	token: Hex
	amount: bigint
}

const ENTITY_TYPE = "IOrderV3"

export type IntentVolumeType = "PLACED" | "FILLED"

// USDC and USDT are assumed to be worth exactly $1.
const STABLE_SYMBOLS = ["USDC", "USDT"]

// Phantom pairs are registered against the cNGN/USDC pool on Base, so snapshot
// prices are denominated in Base USDC: a $1 stable with 6 decimals.
const SNAPSHOT_QUOTE_DECIMALS = 6

const ZERO_ADDRESS = "0x0000000000000000000000000000000000000000"

// Snapshots are scanned newest-first; a small window is enough to skip rows with no valid bids.
const SNAPSHOT_SCAN_LIMIT = 10

const decodeChain = (value: string): string =>
	value.startsWith("0x") ? ethers.utils.toUtf8String(value) : value

export interface DispatchInfo {
	assets: TokenInfo[]
	call: Hex
}
export interface PaymentInfoV3 {
	beneficiary: Hex
	assets: TokenInfo[]
	call: Hex
}
export interface OrderV3 {
	id?: string
	user: Hex
	sourceChain: string
	destChain: string
	deadline: bigint
	nonce: bigint
	fees: bigint
	session: Hex
	predispatch: DispatchInfo
	inputs: TokenInfo[]
	outputs: PaymentInfoV3
}

export const DEFAULT_REFERRER = "0x0000000000000000000000000000000000000000000000000000000000000000" as Hex

export class IntentGatewayV3Service {
	/**
	 * Create predispatch asset entities for an order
	 */
	private static async createPredispatchAssets(orderId: string, predispatch: DispatchInfo): Promise<void> {
		await Promise.all(
			predispatch.assets.map(async (asset, index) => {
				const assetId = `${orderId}-predispatch-${index}`
				let assetEntity = await IOrderV3PredispatchAsset.get(assetId)

				if (!assetEntity) {
					assetEntity = await IOrderV3PredispatchAsset.create({
						id: assetId,
						orderId,
						token: asset.token,
						amount: asset.amount,
						index,
					})
				} else {
					assetEntity.token = asset.token
					assetEntity.amount = asset.amount
					assetEntity.index = index
				}
				await assetEntity.save()
			}),
		)
	}

	/**
	 * Create input asset entities for an order
	 */
	private static async createInputAssets(orderId: string, inputs: TokenInfo[]): Promise<void> {
		await Promise.all(
			inputs.map(async (input, index) => {
				const assetId = `${orderId}-input-${index}`
				let assetEntity = await IOrderV3InputAsset.get(assetId)

				if (!assetEntity) {
					assetEntity = await IOrderV3InputAsset.create({
						id: assetId,
						orderId,
						token: input.token,
						amount: input.amount,
						index,
					})
				} else {
					assetEntity.token = input.token
					assetEntity.amount = input.amount
					assetEntity.index = index
				}
				await assetEntity.save()
			}),
		)
	}

	/**
	 * Create output asset entities for an order
	 */
	private static async createOutputAssets(orderId: string, outputs: PaymentInfoV3): Promise<void> {
		// Create/update output asset entities
		await Promise.all(
			outputs.assets.map(async (asset, index) => {
				const assetId = `${orderId}-output-${index}`
				let assetEntity = await IOrderV3OutputAsset.get(assetId)

				if (!assetEntity) {
					assetEntity = await IOrderV3OutputAsset.create({
						id: assetId,
						orderId,
						token: asset.token,
						amount: asset.amount,
						index,
						beneficiary: outputs.beneficiary as string,
					})
				} else {
					assetEntity.token = asset.token
					assetEntity.amount = asset.amount
					assetEntity.index = index
					assetEntity.beneficiary = outputs.beneficiary as string
				}
				await assetEntity.save()
			}),
		)
	}

	static async getOrCreateOrder(
		order: OrderV3,
		referrer: string,
		logsData: {
			transactionHash: string
			blockNumber: number
			timestamp: bigint
		},
	): Promise<OrderV3Placed> {
		const { transactionHash, blockNumber, timestamp } = logsData

		let orderPlaced = await OrderV3Placed.get(order.id!)

		if (!orderPlaced) {
			const { inputUSD } = await this.getOrderValue(order)
			orderPlaced = await OrderV3Placed.create({
				id: order.id!,
				user: order.user,
				sourceChain: order.sourceChain,
				destChain: order.destChain,
				commitment: order.id!,
				deadline: order.deadline,
				nonce: order.nonce,
				fees: order.fees,
				session: order.session,
				inputUSD: BigInt(new Decimal(inputUSD).truncated().toString()),
				predispatchCalldata: order.predispatch.call as string,
				postDispatchCalldata: order.outputs.call as string,
				status: OrderStatus.PLACED,
				referrer,
				createdAt: timestampToDate(timestamp),
				blockNumber: BigInt(blockNumber),
				blockTimestamp: timestamp,
				transactionHash,
			})
			await orderPlaced.save()

			// Create asset entities
			await this.createPredispatchAssets(order.id!, order.predispatch)
			await this.createInputAssets(order.id!, order.inputs)
			await this.createOutputAssets(order.id!, order.outputs)

			logger.info(
				`OrderV3 Placed Event successfully saved: ${stringify({
					orderPlaced,
				})}`,
			)

			await this.flushPendingStatuses(order.id!)

			logger.info("Now awarding points for the OrderV3 Placed Event")

			// Award points for order placement - using USD value directly
			const orderValue = new Decimal(inputUSD)
			const pointsToAward = orderValue.floor().toNumber()

			await PointsService.awardPoints(
				order.user,
				decodeChain(order.sourceChain),
				BigInt(pointsToAward),
				ProtocolParticipantType.USER,
				PointsActivityType.ORDER_PLACED_POINTS,
				transactionHash,
				`Points awarded for placing orderV3 ${order.id} with value ${inputUSD} USD`,
				timestamp,
			)

			await VolumeService.updateVolume("IntentGatewayV3.USER", inputUSD, timestamp)

			// Convert user to 20 bytes for UserActivityV2 ID, but keep referrer as 32 bytes
			const userAddress20 = bytes32ToBytes20(order.user)
			let user = await getOrCreateUser(userAddress20, referrer, timestamp)
			user.totalOrdersPlaced = user.totalOrdersPlaced + BigInt(1)
			user.totalOrderPlacedVolumeUSD = new Decimal(user.totalOrderPlacedVolumeUSD)
				.plus(new Decimal(inputUSD))
				.toString()
			user.createdAt = user.createdAt === timestampToDate(BigInt(0)) ? timestampToDate(timestamp) : user.createdAt
			await user.save()
		} else {
			// Handle race condition: Order already exists (e.g., was filled first)
			// Update all fields except status and status-related metadata
			logger.info(
				`OrderV3 ${stringify({ order: order.id })} already exists with status ${stringify({ status: orderPlaced.status })}. Updating order details while preserving status.`,
			)

			const existingStatus = orderPlaced.status
			const { inputUSD } = await this.getOrderValue(order)

			orderPlaced.user = order.user
			orderPlaced.sourceChain = order.sourceChain
			orderPlaced.destChain = order.destChain
			orderPlaced.deadline = order.deadline
			orderPlaced.nonce = order.nonce
			orderPlaced.fees = order.fees
			orderPlaced.session = order.session
			orderPlaced.inputUSD = BigInt(new Decimal(inputUSD).truncated().toString())
			orderPlaced.predispatchCalldata = order.predispatch.call as string
			orderPlaced.postDispatchCalldata = order.outputs.call as string
			orderPlaced.referrer = referrer
			// Keep existing status - don't overwrite it

			await orderPlaced.save()

			// Update asset entities
			await this.createPredispatchAssets(order.id!, order.predispatch)
			await this.createInputAssets(order.id!, order.inputs)
			await this.createOutputAssets(order.id!, order.outputs)

			logger.info(
				`OrderV3 ${stringify({ order })} updated with actual data. Status remains: ${stringify({ existingStatus })}`,
			)

			// Award points for order placement - using USD value directly
			// Only award if status is not already FILLED (to avoid double awarding)
			if (existingStatus !== OrderStatus.FILLED) {
				logger.info("Now awarding points for the OrderV3 Placed Event")

				const orderValue = new Decimal(inputUSD)
				const pointsToAward = orderValue.floor().toNumber()

				await PointsService.awardPoints(
					order.user,
					decodeChain(order.sourceChain),
					BigInt(pointsToAward),
					ProtocolParticipantType.USER,
					PointsActivityType.ORDER_PLACED_POINTS,
					transactionHash,
					`Points awarded for placing orderV3 ${order.id} with value ${inputUSD} USD`,
					timestamp,
				)

				await VolumeService.updateVolume("IntentGatewayV3.USER", inputUSD, timestamp)
			}
		}

		return orderPlaced
	}

	static async getByCommitment(commitment: string): Promise<OrderV3Placed | null> {
		const orderPlaced = await OrderV3Placed.get(commitment)

		if (!orderPlaced) return null

		return orderPlaced
	}

	private static async getOrderValue(order: OrderV3): Promise<{ inputUSD: string }> {
		const inputValuesUSD = await this.getInputValuesUSD(order)

		return {
			inputUSD: inputValuesUSD.total,
		}
	}

	private static async getInputValuesUSD(order: OrderV3): Promise<{ total: string; values: string[] }> {
		return this.getTokenValuesUSD(order.inputs)
	}

	private static async getOutputValuesUSD(outputs: TokenInfo[]): Promise<{ total: string; values: string[] }> {
		return this.getTokenValuesUSD(outputs)
	}

	private static async getTokenValuesUSD(
		tokens: { token: string; amount: bigint }[],
	): Promise<{ total: string; values: string[] }> {
		const valuesUSD = await Promise.all(
			tokens.map(async (token) => {
				const tokenAddress = bytes32ToBytes20(token.token)
				const { symbol, decimals } = await this.getTokenMetadata(tokenAddress)

				const price = await this.getTokenUsdPriceWithFx(tokenAddress, symbol, decimals)
				return PriceHelper.getAmountValueInUSD(token.amount, decimals, price ? price.toFixed(18) : "0")
			}),
		)

		const total = valuesUSD.reduce((acc, curr) => {
			return acc.plus(new Decimal(curr.amountValueInUSD))
		}, new Decimal(0))

		return {
			total: total.toFixed(18),
			values: valuesUSD.map((value) => value.amountValueInUSD),
		}
	}

	/**
	 * Record intent gateway volume for a list of order tokens: cumulative raw amounts per
	 * chain-token, plus a per-chain USD rollup priced at $1 for stables and via the
	 * latest PhantomOrderPriceSnapshot exchange rate for FX tokens.
	 */
	static async recordOrderVolume(
		volumeType: IntentVolumeType,
		tokens: { token: string; amount: bigint }[],
		timestamp: bigint,
	): Promise<void> {
		const chain = getHostStateMachine(chainId)

		const amountByToken = new Map<string, bigint>()
		for (const { token, amount } of tokens) {
			const tokenAddress = bytes32ToBytes20(token).toLowerCase()
			amountByToken.set(tokenAddress, (amountByToken.get(tokenAddress) ?? 0n) + amount)
		}

		const values = await Promise.all(
			Array.from(amountByToken, async ([tokenAddress, amount]) => {
				const id = `${chain}-${tokenAddress}-${volumeType}`
				let tokenVolume = await IntentGatewayTokenVolume.get(id)
				let symbol: string
				let decimals: number

				if (tokenVolume) {
					symbol = tokenVolume.tokenSymbol
					decimals = tokenVolume.decimals
					tokenVolume.amount = tokenVolume.amount + amount
					tokenVolume.lastUpdatedAt = timestamp
				} else {
					;({ symbol, decimals } = await this.getTokenMetadata(tokenAddress))
					tokenVolume = IntentGatewayTokenVolume.create({
						id,
						chain,
						tokenAddress,
						tokenSymbol: symbol,
						decimals,
						volumeType,
						amount,
						lastUpdatedAt: timestamp,
					})
				}
				await tokenVolume.save()

				const price = await this.getTokenUsdPriceWithFx(tokenAddress, symbol, decimals)
				if (!price) {
					logger.warn(
						`[IntentGatewayV3Service.recordOrderVolume] No USD price for ${symbol} (${tokenAddress}) on ${chain}; skipping USD rollup, raw amount retained`,
					)
					return new Decimal(0)
				}

				return new Decimal(PriceHelper.getAmountValueInUSD(amount, decimals, price.toFixed(18)).amountValueInUSD)
			}),
		)

		const usdDelta = values.reduce((acc, curr) => acc.plus(curr), new Decimal(0))
		if (usdDelta.isZero()) return

		const cumulativeId = `${chain}-${volumeType}`
		const scaled = toScaledUsd(usdDelta.toFixed(18))
		let cumulative = await CumulativeIntentGatewayVolumeUSD.get(cumulativeId)
		if (!cumulative) {
			cumulative = CumulativeIntentGatewayVolumeUSD.create({
				id: cumulativeId,
				chain,
				volumeType,
				volumeUSD: scaled,
				lastUpdatedAt: timestamp,
			})
		} else {
			cumulative.volumeUSD = cumulative.volumeUSD + scaled
			cumulative.lastUpdatedAt = timestamp
		}
		await cumulative.save()
	}

	private static async getTokenMetadata(tokenAddress: string): Promise<{ symbol: string; decimals: number }> {
		if (tokenAddress === ZERO_ADDRESS) {
			return { symbol: "ETH", decimals: 18 }
		}

		const tokenContract = ERC6160Ext20Abi__factory.connect(tokenAddress, api)
		const symbol = await tokenContract.symbol()
		const decimals = await tokenContract.decimals()
		return { symbol, decimals }
	}

	private static async getTokenUsdPriceWithFx(
		tokenAddress: string,
		symbol: string,
		decimals: number,
	): Promise<Decimal | null> {
		if (STABLE_SYMBOLS.includes(symbol.toUpperCase())) {
			return new Decimal(1)
		}

		// Snapshots reference Base token addresses, so cNGN on any chain is priced
		// via its Base representation.
		if (symbol.toUpperCase() === "CNGN") {
			return this.getFxPriceFromSnapshots(BASE_CNGN.address, BASE_CNGN.decimals)
		}

		return this.getFxPriceFromSnapshots(tokenAddress.toLowerCase(), decimals)
	}

	/**
	 * Price an FX token from the latest PhantomOrderPriceSnapshot referencing it as either
	 * leg of the pair. The snapshot rate is medianPrice / standardAmount adjusted for
	 * decimals; the quote leg is assumed to be a $1 stable with SNAPSHOT_QUOTE_DECIMALS.
	 * Snapshot addresses live on Base, so tokenAddress must be the Base representation.
	 */
	private static async getFxPriceFromSnapshots(tokenAddress: string, decimals: number): Promise<Decimal | null> {
		const [asInput, asOutput] = await Promise.all([
			PhantomOrderPriceSnapshot.getByFields([["tokenA", "=", tokenAddress]], {
				limit: SNAPSHOT_SCAN_LIMIT,
				orderBy: "blockNumber",
				orderDirection: "DESC",
			}),
			PhantomOrderPriceSnapshot.getByFields([["tokenB", "=", tokenAddress]], {
				limit: SNAPSHOT_SCAN_LIMIT,
				orderBy: "blockNumber",
				orderDirection: "DESC",
			}),
		])

		const snapshot = [...asInput, ...asOutput]
			.filter((s) => s.medianPrice && s.medianPrice > 0n && s.standardAmount > 0n)
			.sort((a, b) => (a.blockNumber > b.blockNumber ? -1 : 1))[0]
		if (!snapshot) return null

		const tokenIsInput = snapshot.tokenA.toLowerCase() === tokenAddress

		const median = new Decimal(snapshot.medianPrice!.toString())
		const standard = new Decimal(snapshot.standardAmount.toString())

		// tokenIsInput: medianPrice is tokenB units received per standardAmount of this token,
		// so its price in stable units is median/standard; otherwise the reciprocal.
		const rate = tokenIsInput
			? median
					.div(new Decimal(10).pow(SNAPSHOT_QUOTE_DECIMALS))
					.div(standard.div(new Decimal(10).pow(decimals)))
			: standard
					.div(new Decimal(10).pow(SNAPSHOT_QUOTE_DECIMALS))
					.div(median.div(new Decimal(10).pow(decimals)))

		return rate
	}

	static async updateOrderStatus(
		commitment: string,
		status: OrderStatus,
		logsData: {
			transactionHash: string
			blockNumber: number
			timestamp: bigint
		},
		filler?: string,
	): Promise<void> {
		const { transactionHash, blockNumber, timestamp } = logsData

		let orderPlaced = await OrderV3Placed.get(commitment)

		if (!orderPlaced) {
			logger.warn(
				`OrderV3 ${stringify({ commitment })} does not exist yet, storing in PendingStatusMetadata for status ${status}`,
			)

			let pending = PendingStatusMetadata.create({
				id: `${commitment}.${ENTITY_TYPE}.${status}`,
				commitment,
				entityType: ENTITY_TYPE,
				status,
				chain: chainId,
				timestamp,
				blockNumber: blockNumber.toString(),
				blockHash: "",
				transactionHash,
				filler,
				createdAt: timestampToDate(timestamp),
			})

			await pending.save()
			return
		}

		orderPlaced.status = status === OrderStatus.PLACED ? orderPlaced.status : status
		await orderPlaced.save()

		// Award points for order filling - using USD value directly
		if (status === OrderStatus.FILLED && filler) {
			// Get output assets from the new entity relationships
			const outputAssets: TokenInfo[] = []
			for (let index = 0; index < 100; index++) {
				const assetId = `${commitment}-output-${index}`
				const asset = await IOrderV3OutputAsset.get(assetId)
				if (!asset) break
				outputAssets.push({
					token: asset.token as Hex,
					amount: asset.amount,
				})
			}

			if (outputAssets.length > 0) {
				// Volume
				let outputUSD = await this.getOutputValuesUSD(outputAssets)

				await VolumeService.updateVolume(`IntentGatewayV3.FILLER.${filler}`, outputUSD.total, timestamp)

				const orderValue = new Decimal(orderPlaced.inputUSD.toString())
				const pointsToAward = orderValue.floor().toNumber()

				// Rewards
				await PointsService.awardPoints(
					filler,
					decodeChain(orderPlaced.destChain),
					BigInt(pointsToAward),
					ProtocolParticipantType.FILLER,
					PointsActivityType.ORDER_FILLED_POINTS,
					transactionHash,
					`Points awarded for filling orderV3 ${commitment} with value ${orderPlaced.inputUSD} USD`,
					timestamp,
				)

				// User - convert to 20 bytes for UserActivityV2 ID, referrer is already 32 bytes
				const userAddress20 = bytes32ToBytes20(orderPlaced.user)
				let user = await getOrCreateUser(userAddress20, orderPlaced.referrer)
				user.totalOrderFilledVolumeUSD = new Decimal(user.totalOrderFilledVolumeUSD)
					.plus(new Decimal(orderPlaced.inputUSD.toString()))
					.toString()
				user.totalFilledOrders = user.totalFilledOrders + BigInt(1)
				await user.save()

				// Referrer
				if (user.referrer) {
					const referrerPointsToAward = Math.floor(pointsToAward / 2)
					await PointsService.awardPoints(
						user.referrer,
						decodeChain(orderPlaced.sourceChain),
						BigInt(referrerPointsToAward),
						ProtocolParticipantType.REFERRER,
						PointsActivityType.ORDER_REFERRED_POINTS,
						transactionHash,
						`Points awarded for filling orderV3 ${commitment} with value ${orderPlaced.inputUSD} USD`,
						timestamp,
					)
				}
			}
		}

		const orderStatusMetadata = await IOrderV3StatusMetadata.create({
			id: `${commitment}.${status}`,
			orderId: commitment,
			status,
			chain: chainId,
			timestamp,
			blockNumber: blockNumber.toString(),
			filler,
			transactionHash,
			createdAt: timestampToDate(timestamp),
		})

		await orderStatusMetadata.save()
	}

	/**
	 * Flush any pending status metadata entries for an order that was just created
	 */
	static async flushPendingStatuses(commitment: string): Promise<void> {
		const pendingStatuses = await PendingStatusMetadata.getByCommitment(commitment, {
			limit: 10,
		})

		const matching = pendingStatuses.filter((p) => p.entityType === ENTITY_TYPE)

		for (const pending of matching) {
			const orderStatusMetadata = IOrderV3StatusMetadata.create({
				id: `${commitment}.${pending.status}`,
				orderId: commitment,
				status: pending.status as OrderStatus,
				chain: pending.chain,
				timestamp: pending.timestamp,
				blockNumber: pending.blockNumber,
				filler: pending.filler,
				transactionHash: pending.transactionHash,
				createdAt: pending.createdAt,
			})

			await orderStatusMetadata.save()
			await PendingStatusMetadata.remove(pending.id)

			logger.info(
				`Flushed pending status ${pending.status} for IOrderV3 ${commitment}`,
			)
		}
	}

	static async recordPartialFill(
		commitment: string,
		filler: string,
		outputs: TokenInfo[],
		inputs: TokenInfo[],
		logsData: {
			transactionHash: string
			blockNumber: number
			timestamp: bigint
			logIndex: number
		},
	): Promise<void> {
		const { transactionHash, blockNumber, timestamp, logIndex } = logsData

		// Ensure we at least log if the order doesn't exist yet (race between PLACED and PartialFill)
		const orderPlaced = await OrderV3Placed.get(commitment)
		if (!orderPlaced) {
			logger.warn(
				`OrderV3 ${stringify({
					commitment,
				})} does not exist yet but PartialFill event received. Recording partial fill linked by commitment.`,
			)
		}

		const partialFillId = `${transactionHash}.${logIndex}`

		let partialFill = await IOrderV3PartialFill.get(partialFillId)
		if (!partialFill) {
			partialFill = await IOrderV3PartialFill.create({
				id: partialFillId,
				orderId: commitment,
				chain: chainId,
				filler,
				timestamp,
				blockNumber: blockNumber.toString(),
				transactionHash,
				createdAt: timestampToDate(timestamp),
			})
		}

		await partialFill.save()

		// Create/update input assets for this partial fill
		await Promise.all(
			inputs.map(async (input, index) => {
				const assetId = `${partialFillId}-input-${index}`
				let assetEntity = await IOrderV3PartialFillInputAsset.get(assetId)

				if (!assetEntity) {
					assetEntity = await IOrderV3PartialFillInputAsset.create({
						id: assetId,
						partialFillId,
						token: input.token,
						amount: input.amount,
						index,
					})
				}

				await assetEntity.save()
			}),
		)

		// Create/update output assets for this partial fill
		await Promise.all(
			outputs.map(async (output, index) => {
				const assetId = `${partialFillId}-output-${index}`
				let assetEntity = await IOrderV3PartialFillOutputAsset.get(assetId)

				// Try to reuse the beneficiary from the original order's output asset
				const orderOutputAssetId = `${commitment}-output-${index}`
				const orderOutputAsset = await IOrderV3OutputAsset.get(orderOutputAssetId)
				const beneficiary = orderOutputAsset?.beneficiary ?? "0x0000000000000000000000000000000000000000"

				if (!assetEntity) {
					assetEntity = await IOrderV3PartialFillOutputAsset.create({
						id: assetId,
						partialFillId,
						token: output.token,
						amount: output.amount,
						index,
						beneficiary,
					})
				}

				await assetEntity.save()
			}),
		)

		logger.info(
			`OrderV3 PartialFill recorded: ${stringify({
				commitment,
				partialFillId,
				filler,
			})}`,
		)
	}

	static async recordFill(
		commitment: string,
		filler: string,
		outputs: TokenInfo[],
		inputs: TokenInfo[],
		logsData: {
			transactionHash: string
			blockNumber: number
			timestamp: bigint
			logIndex: number
		},
	): Promise<void> {
		const { transactionHash, blockNumber, timestamp, logIndex } = logsData

		const orderPlaced = await OrderV3Placed.get(commitment)
		if (!orderPlaced) {
			logger.warn(
				`OrderV3 ${stringify({
					commitment,
				})} does not exist yet but OrderFilled event received. Recording fill linked by commitment.`,
			)
		}

		const fillId = `${transactionHash}.${logIndex}`

		let fill = await IOrderV3Fill.get(fillId)
		if (!fill) {
			fill = await IOrderV3Fill.create({
				id: fillId,
				orderId: commitment,
				chain: chainId,
				filler,
				timestamp,
				blockNumber: blockNumber.toString(),
				transactionHash,
				createdAt: timestampToDate(timestamp),
			})
		}
		await fill.save()

		await Promise.all(
			inputs.map(async (input, index) => {
				const assetId = `${fillId}-input-${index}`
				let assetEntity = await IOrderV3FillInputAsset.get(assetId)
				if (!assetEntity) {
					assetEntity = await IOrderV3FillInputAsset.create({
						id: assetId,
						fillId,
						token: input.token,
						amount: input.amount,
						index,
					})
				}
				await assetEntity.save()
			}),
		)

		await Promise.all(
			outputs.map(async (output, index) => {
				const assetId = `${fillId}-output-${index}`
				let assetEntity = await IOrderV3FillOutputAsset.get(assetId)
				if (!assetEntity) {
					assetEntity = await IOrderV3FillOutputAsset.create({
						id: assetId,
						fillId,
						token: output.token,
						amount: output.amount,
						index,
					})
				}
				await assetEntity.save()
			}),
		)

		logger.info(
			`OrderV3 Fill recorded: ${stringify({
				commitment,
				fillId,
				filler,
			})}`,
		)
	}

	static async recordEscrowRelease(
		commitment: string,
		tokens: TokenInfo[],
		logsData: {
			transactionHash: string
			blockNumber: number
			timestamp: bigint
			logIndex: number
		},
	): Promise<void> {
		const { transactionHash, blockNumber, timestamp, logIndex } = logsData
		const releaseId = `${transactionHash}.${logIndex}`

		let release = await IOrderV3EscrowRelease.get(releaseId)
		if (!release) {
			release = await IOrderV3EscrowRelease.create({
				id: releaseId,
				orderId: commitment,
				chain: chainId,
				timestamp,
				blockNumber: blockNumber.toString(),
				transactionHash,
				createdAt: timestampToDate(timestamp),
			})
		}
		await release.save()

		await Promise.all(
			tokens.map(async (token, index) => {
				const tokenId = `${releaseId}-token-${index}`
				let tokenEntity = await IOrderV3EscrowReleaseToken.get(tokenId)
				if (!tokenEntity) {
					tokenEntity = await IOrderV3EscrowReleaseToken.create({
						id: tokenId,
						releaseId,
						token: token.token,
						amount: token.amount,
						index,
					})
				}
				await tokenEntity.save()
			}),
		)
	}

	static async recordEscrowRefund(
		commitment: string,
		tokens: TokenInfo[],
		logsData: {
			transactionHash: string
			blockNumber: number
			timestamp: bigint
			logIndex: number
		},
	): Promise<void> {
		const { transactionHash, blockNumber, timestamp, logIndex } = logsData
		const refundId = `${transactionHash}.${logIndex}`

		let refund = await IOrderV3EscrowRefund.get(refundId)
		if (!refund) {
			refund = await IOrderV3EscrowRefund.create({
				id: refundId,
				orderId: commitment,
				chain: chainId,
				timestamp,
				blockNumber: blockNumber.toString(),
				transactionHash,
				createdAt: timestampToDate(timestamp),
			})
		}
		await refund.save()

		await Promise.all(
			tokens.map(async (token, index) => {
				const tokenId = `${refundId}-token-${index}`
				let tokenEntity = await IOrderV3EscrowRefundToken.get(tokenId)
				if (!tokenEntity) {
					tokenEntity = await IOrderV3EscrowRefundToken.create({
						id: tokenId,
						refundId,
						token: token.token,
						amount: token.amount,
						index,
					})
				}
				await tokenEntity.save()
			}),
		)
	}

	static computeOrderCommitment(order: OrderV3): string {
		// Legacy DB rows store state-machine ids as hex bytes; new event data
		// arrives as plain strings (e.g. "EVM-97"). Normalise both to the hex
		// form the Order struct's `bytes source/destination` fields expect.
		const toBytes = (value: string): `0x${string}` =>
			value.startsWith("0x") ? (value as `0x${string}`) : toHex(value)

		const encoded = encodeAbiParameters(
			[ORDER_TUPLE_TYPE],
			[
				{
					user: order.user,
					source: toBytes(order.sourceChain),
					destination: toBytes(order.destChain),
					deadline: order.deadline,
					nonce: order.nonce,
					fees: order.fees,
					session: (
						order.session || "0x0000000000000000000000000000000000000000"
					).toLowerCase() as `0x${string}`,
					predispatch: {
						assets: order.predispatch.assets.map((a) => ({ token: a.token, amount: a.amount })),
						call: order.predispatch.call,
					},
					inputs: order.inputs.map((i) => ({ token: i.token, amount: i.amount })),
					output: {
						beneficiary: order.outputs.beneficiary,
						assets: order.outputs.assets.map((a) => ({ token: a.token, amount: a.amount })),
						call: order.outputs.call,
					},
				} as any,
			],
		)

		return keccak256(encoded)
	}
}
