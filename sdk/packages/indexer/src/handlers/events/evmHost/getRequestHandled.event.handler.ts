import { HyperBridgeService } from "@/services/hyperbridge.service"
import { Status, Transfer } from "@/configs/src/types"
import { GetRequestHandledLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { GetRequestService } from "@/services/getRequest.service"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { wrap } from "@/utils/event.utils"
import { getPriceDataFromEthereumLog, isERC20TransferEvent, extractAddressFromTopic } from "@/utils/transfer.helpers"
import { TransferService } from "@/services/transfer.service"
import { VolumeService } from "@/services/volume.service"
import { safeArray } from "@/utils/data.helper"
import HandlerV1Abi from "@/configs/abis/HandlerV1.abi.json"
import { GetResponseMessage } from "@/types/ismp"
import { Interface } from "@ethersproject/abi"

/**
 * Handles the GetRequestHandled event from EVMHost
 */
export const handleGetRequestHandledEvent = wrap(async (event: GetRequestHandledLog): Promise<void> => {
	if (!event.args) return

	const { args, block, transaction, transactionHash, blockNumber, blockHash } = event

	const { relayer: relayer_id, commitment } = args

	logger.info(
		`Handling GetRequestHandled Event: ${stringify({
			blockNumber,
			transactionHash,
		})}`,
	)

	const chain = getHostStateMachine(chainId)
	const blockTimestamp = await getBlockTimestamp(blockHash, chain)

	try {
		await HyperBridgeService.handlePostRequestOrResponseHandledEvent(relayer_id, chain, blockTimestamp)

		await GetRequestService.updateStatus({
			commitment,
			chain,
			blockNumber: blockNumber.toString(),
			blockHash: block.hash,
			blockTimestamp,
			status: Status.DESTINATION,
			transactionHash,
		})

		let fromAddresses = [] as string[]

		if (transaction?.input) {
			const { name, args } = new Interface(HandlerV1Abi).parseTransaction({ data: transaction.input })

			if (name === "handleGetResponses" && args && args.length > 1) {
				const getResponses = args[1] as GetResponseMessage
				for (const getResponse of getResponses.responses) {
					const { get } = getResponse.response
					const { from: getRequestFrom } = get
					fromAddresses.push(getRequestFrom)
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

				// Compute USD value first; skip zero-USD transfers
				const { symbol, amountValueInUSD } = await getPriceDataFromEthereumLog(
					log.address,
					value,
					blockTimestamp,
				)
				if (amountValueInUSD === "0") {
					continue
				}

				await TransferService.storeTransfer({
					transactionHash: transferId,
					chain,
					value,
					from,
					to,
				})

				await VolumeService.updateVolume(`Transfer.${symbol}`, amountValueInUSD, blockTimestamp)

				const matchingContract = fromAddresses.find(
					(addr) => addr.toLowerCase() === from.toLowerCase() || addr.toLowerCase() === to.toLowerCase(),
				)

				if (matchingContract) {
					await VolumeService.updateVolume(`Contract.${matchingContract}`, amountValueInUSD, blockTimestamp)
				}
			}
		}
	} catch (error) {
		logger.error(`Error handling GetRequestHandled Event: ${error}`)
	}
})
