interface IStateMachineDetails {
	chainId: string
	startBlock: number
	isEvm: boolean
	isEvmL2: boolean
	ethereumHostAddress: string
}

export const SUPPORTED_STATE_MACHINES: { [key: string]: IStateMachineDetails } = {
	"EVM-11155111": {
		chainId: "11155111",
		startBlock: 5659633,
		isEvm: true,
		isEvmL2: false,
		ethereumHostAddress: "0x92F217a5e965EAa2aD356678D537A0A9ccC0AF41",
	}, // Ethereum Sepolia
	"EVM-84532": {
		chainId: "84532",
		startBlock: 8464600,
		isEvm: true,
		isEvmL2: true,
		ethereumHostAddress: "0xB72759815CF029EFDb957A676C3593Ec762CFD4e",
	}, // Base Sepolia
	"EVM-11155420": {
		chainId: "11155420",
		startBlock: 8906802,
		isEvm: true,
		isEvmL2: true,
		ethereumHostAddress: "0x27D689e361ab92aCab04Ea21c1B3F507A94a9DAd",
	}, // Optimism Sepolia
	"EVM-421614": {
		chainId: "421614",
		startBlock: 20034995,
		isEvm: true,
		isEvmL2: true,
		ethereumHostAddress: "0x15Ba7e42BC2c3e8FeDEb30D13CEE611D97315E7F",
	}, // Arbitrum Sepolia
	"EVM-97": {
		chainId: "97",
		startBlock: 38301829,
		isEvm: true,
		isEvmL2: false,
		ethereumHostAddress: "0x0cac3dF856aD8939955086AADd243a28f35988BE",
	}, // BSC Chapel
}

export const GET_ETHEREUM_L2_STATE_MACHINES = () => {
	return Object.entries(SUPPORTED_STATE_MACHINES)
		.filter(([_, value]) => value.isEvmL2)
		.map(([key, _]) => key)
}

export const GET_HOST_ADDRESSES = () => {
	return Object.values(SUPPORTED_STATE_MACHINES).map((stateMachine) => stateMachine.ethereumHostAddress)
}
