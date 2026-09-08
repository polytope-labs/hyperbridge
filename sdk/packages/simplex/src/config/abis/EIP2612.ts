import { erc20Abi } from "viem"

export const EIP2612_ABI = [
	...erc20Abi,
	{
		inputs: [{ internalType: "address", name: "owner", type: "address" }],
		stateMutability: "view",
		type: "function",
		name: "nonces",
		outputs: [{ internalType: "uint256", name: "", type: "uint256" }],
	},
	{
		inputs: [],
		name: "version",
		outputs: [{ internalType: "string", name: "", type: "string" }],
		stateMutability: "view",
		type: "function",
	},
] as const
