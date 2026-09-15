// Contract reads batched through Multicall3, pinned to the handler's block like any other read
// through SubQuery's provider.
import { ethers } from "ethers"

import Multicall3Abi from "@/configs/abis/Multicall3.abi.json"

/** Multicall3's deterministic address, the same on every chain it is deployed to. */
export const MULTICALL3_ADDRESS = "0xcA11bde05977b3631167028862bE2a173976CA11"

/** Reads per `aggregate3` call, which keeps one eth_call well inside providers' gas and response limits. */
export const MULTICALL_BATCH_SIZE = 250

const multicall3 = new ethers.utils.Interface(Multicall3Abi)

export interface ContractRead {
	target: string
	abi: ethers.utils.Interface
	method: string
	args: readonly unknown[]
}

/** One read's outcome: a revert or an undecodable answer fails that read alone. */
export type Settled<T> = { ok: true; value: T } | { ok: false; error: string }

/** The value of a settled read, or its failure thrown. */
export function unwrap<T>(settled: Settled<T>): T {
	if (!settled.ok) throw new Error(settled.error)
	return settled.value
}

const errorMessage = (error: unknown): string => (error instanceof Error ? error.message : String(error))

/** Settles `promise` into an outcome, prefixing a failure with what was being read. */
export async function settle<T>(label: string, promise: Promise<T>): Promise<Settled<T>> {
	try {
		return { ok: true, value: await promise }
	} catch (error) {
		return { ok: false, error: `${label}: ${errorMessage(error)}` }
	}
}

// Whether Multicall3 has code, per chain, for the life of the process. The indexer's start blocks all
// postdate its deployment, so the answer at the first block asked holds for every later one.
const deployed = new Map<string, boolean>()

async function hasMulticall3(chain: string): Promise<boolean> {
	const known = deployed.get(chain)
	if (known !== undefined) return known
	const code: string = await (api as any).getCode(MULTICALL3_ADDRESS)
	const present = !!code && code !== "0x"
	deployed.set(chain, present)
	return present
}

/** Forgets which chains have Multicall3. For tests. */
export function resetMulticallCache(): void {
	deployed.clear()
}

function decode(read: ContractRead, returnData: string): Settled<ethers.utils.Result> {
	try {
		return { ok: true, value: read.abi.decodeFunctionResult(read.method, returnData) }
	} catch (error) {
		return { ok: false, error: `${read.method} on ${read.target}: ${errorMessage(error)}` }
	}
}

const callData = (read: ContractRead): string => read.abi.encodeFunctionData(read.method, read.args)

async function aggregate(reads: ContractRead[]): Promise<Settled<ethers.utils.Result>[]> {
	const data = multicall3.encodeFunctionData("aggregate3", [
		reads.map((read) => ({ target: read.target, allowFailure: true, callData: callData(read) })),
	])
	const [results] = multicall3.decodeFunctionResult(
		"aggregate3",
		await (api as any).call({ to: MULTICALL3_ADDRESS, data }),
	)
	return reads.map((read, i) =>
		results[i].success
			? decode(read, results[i].returnData)
			: { ok: false, error: `${read.method} on ${read.target} reverted` },
	)
}

/**
 * Runs `reads` at the handler's block and returns their outcomes in order: one eth_call per
 * MULTICALL_BATCH_SIZE reads. A single read, or a chain without Multicall3, is called directly
 * instead, with every call sent at once. A read that reverts or cannot be decoded settles as failed
 * on its own; a failed eth_call to Multicall3 itself throws.
 */
export async function readContracts(chain: string, reads: ContractRead[]): Promise<Settled<ethers.utils.Result>[]> {
	if (reads.length === 0) return []
	if (reads.length === 1 || !(await hasMulticall3(chain))) {
		return Promise.all(
			reads.map(async (read) => {
				const answer = await settle<string>(
					`${read.method} on ${read.target}`,
					(api as any).call({ to: read.target, data: callData(read) }),
				)
				return answer.ok ? decode(read, answer.value) : answer
			}),
		)
	}
	const batches: ContractRead[][] = []
	for (let start = 0; start < reads.length; start += MULTICALL_BATCH_SIZE) {
		batches.push(reads.slice(start, start + MULTICALL_BATCH_SIZE))
	}
	return (await Promise.all(batches.map(aggregate))).flat()
}
