import { ethers } from "ethers"

import Erc4626Abi from "@/configs/abis/Erc4626.abi.json"
import { ENV_CONFIG } from "@/constants"
import { replaceWebsocketWithHttp } from "@/utils/rpc.helpers"
import { safeFetch } from "@/utils/safeFetch"

const abi = new ethers.utils.Interface(Erc4626Abi)
const ZERO_ADDRESS = ethers.constants.AddressZero

/** Mint/burns are accounted by Deposit/Withdraw; self and zero-share transfers move no capital. */
export function isOrdinaryVaultTransfer(from: string, to: string, shares: bigint): boolean {
	return (
		from.toLowerCase() !== ZERO_ADDRESS &&
		to.toLowerCase() !== ZERO_ADDRESS &&
		from.toLowerCase() !== to.toLowerCase() &&
		shares !== 0n
	)
}

export interface VaultCapitalMovement {
	transactionHash: string
	logIndex: number
	lp: string
	shares: bigint
}

/** Share changes represented by Deposit/Withdraw and ordinary (non-mint/burn) Transfers. */
export function vaultCapitalMovements(log: {
	topics: string[]
	data: string
	transactionHash: string
	logIndex: number
}): VaultCapitalMovement[] {
	const event = abi.parseLog(log)
	const movement = (lp: string, shares: bigint): VaultCapitalMovement => ({
		transactionHash: log.transactionHash.toLowerCase(),
		logIndex: log.logIndex,
		lp: lp.toLowerCase(),
		shares,
	})
	if (event.name === "Deposit") return [movement(event.args.owner, BigInt(event.args.shares.toString()))]
	if (event.name === "Withdraw") return [movement(event.args.owner, -BigInt(event.args.shares.toString()))]
	const from = event.args.from.toLowerCase()
	const to = event.args.to.toLowerCase()
	const shares = BigInt(event.args.value.toString())
	if (!isOrdinaryVaultTransfer(from, to, shares)) return []
	return [movement(from, -shares), movement(to, shares)]
}

/**
 * SubQuery's safe provider pins calls to the handler block, but does not expose getLogs.
 * Read just that block through the configured HTTP RPC; never scan history or read latest.
 * Do not cache across handler invocations: retries/reorgs must re-read the canonical block.
 */
export async function readVaultBlockMovements(
	chain: string,
	vault: string,
	blockNumber: bigint,
): Promise<VaultCapitalMovement[]> {
	const url = replaceWebsocketWithHttp(ENV_CONFIG[chain] ?? "")
	if (!url) throw new Error(`No RPC configured for vault accounting on ${chain}`)
	const block = `0x${blockNumber.toString(16)}`
	const response = await safeFetch(url, {
		method: "POST",
		headers: { "content-type": "application/json" },
		body: JSON.stringify({
			jsonrpc: "2.0",
			id: 1,
			method: "eth_getLogs",
			params: [
				{
					address: vault,
					fromBlock: block,
					toBlock: block,
					topics: [
						[abi.getEventTopic("Deposit"), abi.getEventTopic("Withdraw"), abi.getEventTopic("Transfer")],
					],
				},
			],
		}),
	})
	const body = await response.json()
	if (!response.ok || body.error || !Array.isArray(body.result)) {
		throw new Error(`Could not read vault capital movements for ${chain}:${vault}:${blockNumber}`)
	}
	const seen = new Set<number>()
	return body.result.flatMap((log: any) => {
		const logIndex = Number(BigInt(log.logIndex))
		if (
			log.removed ||
			BigInt(log.blockNumber) !== blockNumber ||
			log.address.toLowerCase() !== vault.toLowerCase() ||
			!Number.isSafeInteger(logIndex) ||
			logIndex < 0
		) {
			throw new Error(`Unexpected vault log for ${chain}:${vault}:${blockNumber}`)
		}
		// An RPC returning a log twice must not double the share movement used to infer capital.
		if (seen.has(logIndex)) throw new Error(`Duplicate vault log for ${chain}:${vault}:${blockNumber}:${logIndex}`)
		seen.add(logIndex)
		return vaultCapitalMovements({ ...log, logIndex })
	})
}
