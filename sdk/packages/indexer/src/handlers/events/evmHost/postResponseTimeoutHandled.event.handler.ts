import { Status } from "@/configs/src/types"
import { PostResponseTimeoutHandledLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { HyperBridgeService } from "@/services/hyperbridge.service"
import { ResponseService } from "@/services/response.service"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import stringify from "safe-stable-stringify"
import { Transfer } from "@/configs/src/types"
import { VolumeService } from "@/services/volume.service"
import { getPriceDataFromEthereumLog, isERC20TransferEvent, extractAddressFromTopic } from "@/utils/transfer.helpers"
import { TransferService } from "@/services/transfer.service"
import { safeArray } from "@/utils/data.helper"
import { PostResponseTimeoutMessage } from "@/types/ismp"
import HandlerV1Abi from "@/configs/abis/HandlerV1.abi.json"
import { Interface } from "@ethersproject/abi"

/**
 * Handles the PostResponseTimeoutHandled event
 */
export const handlePostResponseTimeoutHandledEvent = wrap(
	async (event: PostResponseTimeoutHandledLog): Promise<void> => {
		if (!event.args) return
		const { args, block, transaction, transactionHash, transactionIndex, blockHash, blockNumber, data } = event
		const { commitment, dest } = args

		logger.info(
			`Handling PostResponseTimeoutHandled Event: ${stringify({
				blockNumber,
				transactionHash,
			})}`,
		)

		const chain: string = getHostStateMachine(chainId)
		const blockTimestamp = await getBlockTimestamp(blockHash, chain)

		try {
			await HyperBridgeService.incrementNumberOfTimedOutMessagesSent(chain)

			await ResponseService.updateStatus({
				commitment,
				chain,
				blockNumber: blockNumber.toString(),
				blockHash: block.hash,
				blockTimestamp,
				status: Status.TIMED_OUT,
				transactionHash,
			})

			let toAddresses = [] as string[]

			if (transaction?.input) {
				const { name, args } = new Interface(HandlerV1Abi).parseTransaction({ data: transaction.input })
				if (name === "handlePostResponseTimeouts" && args && args.length > 1) {
					const { timeouts } = args[1] as PostResponseTimeoutMessage
					for (const timeout of timeouts) {
						const {
							post: { to },
						} = timeout
						toAddresses.push(to)
					}
				}
			}

			for (const [index, log] of safeArray(transaction?.logs).entries()) {
				if (!isERC20TransferEvent(log)) {
					continue
				}

				const value = BigInt(log.data)
				const transferId = `${log.transactionHash}-index-${index}`
				const transfer = await Transfer.get(transferId)

				if (!transfer) {
					const [_, fromTopic, toTopic] = log.topics
					const from = extractAddressFromTopic(fromTopic)
					const to = extractAddressFromTopic(toTopic)
					await TransferService.storeTransfer({
						transactionHash: transferId,
						chain,
						value,
						from,
						to,
					})

					const { symbol, amountValueInUSD } = await getPriceDataFromEthereumLog(
						log.address,
						value,
						blockTimestamp,
					)
					await VolumeService.updateVolume(`Transfer.${symbol}`, amountValueInUSD, blockTimestamp)

					const matchingContract = toAddresses.find(
						(addr) => addr.toLowerCase() === from.toLowerCase() || addr.toLowerCase() === to.toLowerCase(),
					)

					if (matchingContract) {
						await VolumeService.updateVolume(
							`Contract.${matchingContract}`,
							amountValueInUSD,
							blockTimestamp,
						)
					}
				}
			}
		} catch (error) {
			logger.error(`Error updating handling post response timeout: ${stringify(error)}`)
		}
	},
)
