import { bytesToBigInt, bytesToHex, hexToBytes, keccak256, pad, toBytes, toHex } from "viem"
import { blake2AsU8a, xxhashAsU8a } from "@polkadot/util-crypto"
import { u64, Struct, Option, Bytes, u8, Vector, Enum, u32, u128, bool } from "scale-ts"
import type { ApiPromise } from "@polkadot/api"
import { Option as PolkadotOption } from "@polkadot/types"
import { logger } from "ethers"
import { TextEncoder } from "util"
import { CHAINS_BY_ISMP_HOST } from "@/constants"
import { Codec } from "@polkadot/types/types"
import { Provider } from "@ethersproject/providers"

// Define ConsensusStateId as 4-byte array
const ConsensusStateId = Vector(u8, 4)

// H256 is a 32-byte array
const H256 = Bytes(32)

// Define StateCommitment
const StateCommitment = Struct({
	timestamp: u64,
	overlay_root: Option(H256),
	state_root: H256,
})

export const RequestMetadata = Struct({
	offchain: Struct({
		leaf_index: u64,
		pos: u64,
	}),
	fee: Struct({
		payer: H256,
		fee: u128,
	}),
	claimed: bool,
})

// Define StateMachine
const StateMachine = Enum({
	Evm: u32,
	Polkadot: u32,
	Kusama: u32,
	Substrate: ConsensusStateId,
	Tendermint: ConsensusStateId,
})

// Define StateMachineId
const StateMachineId = Struct({
	state_id: StateMachine,
	consensus_state_id: ConsensusStateId,
})

// Define StateMachineHeight
const StateMachineHeight = Struct({
	id: StateMachineId,
	height: u64,
})

type StateMachineHeight = {
	id: {
		state_id:
			| { tag: "Evm"; value: number }
			| { tag: "Polkadot"; value: number }
			| { tag: "Kusama"; value: number }
			| { tag: "Substrate"; value: number[] }
			| { tag: "Tendermint"; value: number[] }
		consensus_state_id: number[]
	}
	height: bigint
}
type StateCommitment = {
	timestamp: bigint
	overlay_root: Uint8Array | undefined
	state_root: Uint8Array
}

export async function fetchStateCommitmentsSubstrate(params: {
	api: ApiPromise
	stateMachineId: string
	consensusStateId: string
	height: bigint
}): Promise<StateCommitment | null> {
	const { api, stateMachineId, consensusStateId, height } = params

	const state_machine_height: StateMachineHeight = {
		id: {
			state_id: getStateId(stateMachineId),
			consensus_state_id: Array.from(new TextEncoder().encode(consensusStateId)),
		},
		height: height,
	}

	logger.info(
		`Fetching state commitment for state machine height: ${JSON.stringify(state_machine_height, bigIntSerializer)}`,
	)

	const palletPrefix = xxhashAsU8a("Ismp", 128)
	const storagePrefix = xxhashAsU8a("StateCommitments", 128)

	const encodedStateMachineHeight = StateMachineHeight.enc(state_machine_height)
	const key = blake2AsU8a(encodedStateMachineHeight, 128)

	const full_key = new Uint8Array([...palletPrefix, ...storagePrefix, ...key, ...encodedStateMachineHeight])
	const hexKey = bytesToHex(full_key)

	const storageValue = await api.rpc.state.getStorage<PolkadotOption<Codec>>(hexKey)

	if (storageValue.isSome) {
		return StateCommitment.dec(hexToBytes(storageValue.value.toHex()))
	}

	return null
}

export async function fetchStateCommitmentsEVM(params: {
	client: Provider
	stateMachineId: string
	consensusStateId: string
	height: bigint
}): Promise<StateCommitment | null> {
	const { client, stateMachineId, consensusStateId, height } = params

	const state_machine_height: StateMachineHeight = {
		id: {
			state_id: getStateId(stateMachineId),
			consensus_state_id: Array.from(new TextEncoder().encode(consensusStateId)),
		},
		height: height,
	}

	logger.info(
		`Fetching EVM state commitment for state machine height: ${JSON.stringify(state_machine_height, bigIntSerializer)}`,
	)

	// Add check for Kusama or Polkadot state machine type
	const stateIdType = state_machine_height.id.state_id.tag
	if (stateIdType !== "Kusama" && stateIdType !== "Polkadot") {
		logger.info(`Unknown State Machine: ${stateIdType}. Expected Polkadot or Kusama state machine`)
		return null
	}

	const hostContractKey = `EVM-${chainId}`
	const hostContract = CHAINS_BY_ISMP_HOST[hostContractKey]

	// Extract the paraId from the state machine ID
	const paraId = BigInt(state_machine_height.id.state_id.value)

	// Generate keys for timestamp, overlay, and state root
	const [timestampKey, overlayKey, stateRootKey] = generateStateCommitmentKeys(paraId, height)

	// Query the three storage values
	const timestampValue = await client.getStorageAt(hostContract, bytesToHex(timestampKey))

	if (!timestampValue) {
		return null
	}

	const overlayRootValue = await client.getStorageAt(hostContract, bytesToHex(overlayKey))

	const stateRootValue = await client.getStorageAt(hostContract, bytesToHex(stateRootKey))

	// Parse timestamp from big-endian bytes to BigInt
	const timestamp = BigInt(timestampValue) / 1000n

	// Create the StateCommitment object
	return {
		timestamp,
		overlay_root: overlayRootValue ? hexToBytes(overlayRootValue as `0x${string}`) : undefined,
		state_root: stateRootValue ? hexToBytes(stateRootValue as `0x${string}`) : new Uint8Array(),
	}
}

function generateStateCommitmentKeys(paraId: bigint, height: bigint): [Uint8Array, Uint8Array, Uint8Array] {
	// Constants
	const STATE_COMMITMENT_SLOT = 5n

	// Convert to bytes using viem utilities
	const stateIdBytes = toBytes(pad(`0x${paraId.toString(16)}`, { size: 32 }))
	const slotBytes = toBytes(pad(`0x${STATE_COMMITMENT_SLOT.toString(16)}`, { size: 32 }))

	// Generate parent map key
	const parentMapKeyData = new Uint8Array([...stateIdBytes, ...slotBytes])
	const parentMapKey = hexToBytes(keccak256(toHex(parentMapKeyData)))

	// Generate commitment key
	const heightBytes = toBytes(pad(`0x${height.toString(16)}`, { size: 32 }))
	const commitmentKeyData = new Uint8Array([...heightBytes, ...parentMapKey])

	// Generate base slot
	const baseSlotHash = keccak256(toHex(commitmentKeyData))
	const baseSlot = hexToBytes(baseSlotHash)

	// Calculate overlay and state root slots
	const baseSlotBigInt = bytesToBigInt(baseSlot)
	const overlaySlot = hexToBytes(pad(`0x${(baseSlotBigInt + 1n).toString(16)}`, { size: 32 }))
	const stateRootSlot = hexToBytes(pad(`0x${(baseSlotBigInt + 2n).toString(16)}`, { size: 32 }))

	return [baseSlot, overlaySlot, stateRootSlot]
}

const bigIntSerializer = (key: string, value: any) => {
	if (typeof value === "bigint") {
		return value.toString()
	}
	return value
}

export const getStateMachineTag = (id: string): "Evm" | "Polkadot" | "Kusama" | "Substrate" | "Tendermint" => {
	switch (id) {
		case "EVM":
			return "Evm"
		case "POLKADOT":
			return "Polkadot"
		case "KUSAMA":
			return "Kusama"
		case "SUBSTRATE":
			return "Substrate"
		case "TENDERMINT":
			return "Tendermint"
		default:
			throw new Error(`Unknown state machine type: ${id}`)
	}
}

export const getStateId = (id: string) => {
	const [type, value] = id.split("-")
	const tag = getStateMachineTag(type)

	switch (tag) {
		case "Evm":
		case "Polkadot":
		case "Kusama":
			return {
				tag,
				value: Number(value),
			}
		case "Substrate":
		case "Tendermint":
			return {
				tag,
				value: Array.from(new TextEncoder().encode(value)),
			}
		default:
			throw new Error(`Unknown state machine type: ${type}`)
	}
}

export { StateMachineHeight, StateMachineId, StateMachine, StateCommitment }
