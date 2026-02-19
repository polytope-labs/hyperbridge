import {
	Chain,
	bscTestnet,
	gnosisChiado,
	sepolia,
	mainnet,
	bsc,
	base,
	arbitrum,
	polygon,
	unichain,
	polygonAmoy,
	tron,
} from "viem/chains"
import { defineChain } from "viem"
import { TronWeb } from "tronweb"
import { HexString } from "@/types"

/** Convert a Tron base58 address to a 0x-prefixed 20-byte EVM hex address */
function tronAddress(base58: string): HexString {
	return `0x${TronWeb.address.toHex(base58).slice(2)}` as HexString
}

export enum Chains {
	BSC_CHAPEL = "EVM-97",
	GNOSIS_CHIADO = "EVM-10200",
	HYPERBRIDGE_GARGANTUA = "KUSAMA-4009",
	SEPOLIA = "EVM-11155111",
	MAINNET = "EVM-1",
	BSC_MAINNET = "EVM-56",
	ARBITRUM_MAINNET = "EVM-42161",
	BASE_MAINNET = "EVM-8453",
	POLYGON_MAINNET = "EVM-137",
	UNICHAIN_MAINNET = "EVM-130",
	POLYGON_AMOY = "EVM-80002",
	TRON_MAINNET = "EVM-728126428",
	TRON_NILE = "EVM-3448148188",
}

/** Tron Nile Testnet (chain ID 3448148188) — not in viem/chains */
export const tronNile = defineChain({
	id: 3448148188,
	name: "TRON Nile Testnet",
	nativeCurrency: { name: "TRX", symbol: "TRX", decimals: 6 },
	rpcUrls: {
		default: { http: ["https://nile.trongrid.io/jsonrpc"] },
	},
	blockExplorers: {
		default: { name: "Nile Tronscan", url: "https://nile.tronscan.org" },
	},
})

export interface ChainConfigData {
	chainId: number
	stateMachineId: Chains
	viemChain?: Chain
	wrappedNativeDecimals?: number
	assets?: {
		WETH: string
		DAI: string
		USDC: string
		USDT: string
	}
	tokenDecimals?: {
		USDC: number
		USDT: number
	}
	tokenStorageSlots?: {
		USDT?: { balanceSlot: number; allowanceSlot: number }
		USDC?: { balanceSlot: number; allowanceSlot: number }
		WETH?: { balanceSlot: number; allowanceSlot: number }
		DAI?: { balanceSlot: number; allowanceSlot: number }
	}
	addresses: {
		IntentGateway?: `0x${string}`
		IntentGatewayV2?: `0x${string}`
		TokenGateway?: `0x${string}`
		Host?: `0x${string}`
		UniswapRouter02?: `0x${string}`
		UniswapV2Factory?: `0x${string}`
		UniswapV3Factory?: `0x${string}`
		UniversalRouter?: `0x${string}`
		UniswapV3Quoter?: `0x${string}`
		UniswapV4Quoter?: `0x${string}`
		Calldispatcher?: `0x${string}`
		Permit2?: `0x${string}`
		/** ERC-4337 v0.8 EntryPoint address (canonical across all EVM chains) */
		EntryPointV08?: `0x${string}`
		/** USDT0 OFT contract address (OFT Adapter on Ethereum, OFT on other chains) */
		Usdt0Oft?: `0x${string}`
	}
	rpcEnvKey?: string
	defaultRpcUrl?: string
	consensusStateId: string
	coingeckoId: string
	popularTokens?: string[]
	/** LayerZero Endpoint ID for cross-chain messaging */
	layerZeroEid?: number
}

// All chain configuration in one place - add new chains here
export const chainConfigs: Record<number, ChainConfigData> = {
	97: {
		chainId: 97,
		stateMachineId: Chains.BSC_CHAPEL,
		viemChain: bscTestnet,
		wrappedNativeDecimals: 18,
		assets: {
			WETH: "0xae13d989dac2f0debff460ac112a837c89baa7cd",
			DAI: "0x1938165569a5463327fb206be06d8d9253aa06b7",
			USDC: "0xA801da100bF16D07F668F4A49E1f71fc54D05177",
			USDT: "0xc043f483373072f7f27420d6e7d7ad269c018e18",
		},
		tokenDecimals: {
			USDC: 18,
			USDT: 18,
		},
		addresses: {
			IntentGateway: "0x016b6ffC9f890d1e28f9Fdb9eaDA776b02F89509",
			IntentGatewayV2: "0xFbF50B2b32768127603cC9eF4b871574b881b8eD",
			TokenGateway: "0xFcDa26cA021d5535C3059547390E6cCd8De7acA6",
			Host: "0x8Aa0Dea6D675d785A882967Bf38183f6117C09b7",
			UniswapRouter02: "0x9639379819420704457B07A0C33B678D9E0F8Df0",
			UniswapV2Factory: "0x12e036669DA18F4A2777853d6e2136b32AceEC86",
			UniswapV3Factory: "0x0000000000000000000000000000000000000000",
			UniversalRouter: "0xcc6d5ece3d4a57245bf5a2f64f3ed9179b81f714",
			UniswapV3Quoter: "0x0000000000000000000000000000000000000000",
			UniswapV4Quoter: "0x0000000000000000000000000000000000000000",
			EntryPointV08: "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108",
		},
		rpcEnvKey: "BSC_CHAPEL",
		defaultRpcUrl: "https://bnb-testnet.api.onfinality.io/public",
		consensusStateId: "BSC0",
		coingeckoId: "binance-smart-chain",
		popularTokens: [
			"0xae13d989dac2f0debff460ac112a837c89baa7cd",
			"0xC625ec7D30A4b1AAEfb1304610CdAcD0d606aC92",
			"0xc043f483373072f7f27420d6e7d7ad269c018e18",
			"0x1938165569A5463327fb206bE06d8D9253aa06b7",
		],
	},
	10200: {
		chainId: 10200,
		stateMachineId: Chains.GNOSIS_CHIADO,
		viemChain: gnosisChiado,
		wrappedNativeDecimals: 18,
		assets: {
			WETH: "0x0000000000000000000000000000000000000000",
			USDC: "0x50b1d3c7c073c9caa1ef207365a2c9c976bd70b9",
			DAI: "0x0000000000000000000000000000000000000000",
			USDT: "0x0000000000000000000000000000000000000000",
		},
		tokenDecimals: {
			USDC: 6,
			USDT: 6,
		},
		addresses: {
			IntentGateway: "0x016b6ffC9f890d1e28f9Fdb9eaDA776b02F89509",
			TokenGateway: "0xFcDa26cA021d5535C3059547390E6cCd8De7acA6",
			Host: "0x58a41b89f4871725e5d898d98ef4bf917601c5eb",
			UniswapRouter02: "0x0000000000000000000000000000000000000000",
			UniswapV2Factory: "0x0000000000000000000000000000000000000000",
			UniswapV3Factory: "0x0000000000000000000000000000000000000000",
			EntryPointV08: "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108",
		},
		rpcEnvKey: "GNOSIS_CHIADO",
		defaultRpcUrl: "https://gnosis-chiado-rpc.publicnode.com",
		consensusStateId: "GNO0",
		coingeckoId: "gnosis",
		popularTokens: ["0x50B1d3c7c073c9caa1Ef207365A2c9C976bD70b9"],
	},
	4009: {
		chainId: 4009,
		stateMachineId: Chains.HYPERBRIDGE_GARGANTUA,
		addresses: {},
		rpcEnvKey: "HYPERBRIDGE_GARGANTUA",
		defaultRpcUrl: "",
		consensusStateId: "PAS0",
		coingeckoId: "hyperbridge",
	},
	11155111: {
		chainId: 11155111,
		stateMachineId: Chains.SEPOLIA,
		viemChain: sepolia,
		wrappedNativeDecimals: 18,
		assets: {
			WETH: "0x7b79995e5f793a07bc00c21412e50ecae098e7f9",
			USDC: "0x0000000000000000000000000000000000000000",
			USDT: "0x0000000000000000000000000000000000000000",
			DAI: "0x0000000000000000000000000000000000000000",
		},
		tokenDecimals: {
			USDC: 6,
			USDT: 6,
		},
		addresses: {
			IntentGateway: "0x016b6ffC9f890d1e28f9Fdb9eaDA776b02F89509",
			TokenGateway: "0xFcDa26cA021d5535C3059547390E6cCd8De7acA6",
			Host: "0x2EdB74C269948b60ec1000040E104cef0eABaae8",
			UniswapRouter02: "0x0000000000000000000000000000000000000000",
			UniswapV2Factory: "0x0000000000000000000000000000000000000000",
			UniswapV3Factory: "0x0000000000000000000000000000000000000000",
			Calldispatcher: "0xC7f13b6D03A0A7F3239d38897503E90553ABe155",
			EntryPointV08: "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108",
		},
		rpcEnvKey: "SEPOLIA",
		defaultRpcUrl: "https://1rpc.io/sepolia",
		consensusStateId: "ETH0",
		coingeckoId: "ethereum",
		popularTokens: ["0x7b79995e5f793A07Bc00c21412e50Ecae098E7f9"],
	},
	1: {
		chainId: 1,
		stateMachineId: Chains.MAINNET,
		viemChain: mainnet,
		wrappedNativeDecimals: 18,
		assets: {
			WETH: "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
			DAI: "0x6b175474e89094c44da98b954eedeac495271d0f",
			USDC: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
			USDT: "0xdac17f958d2ee523a2206206994597c13d831ec7",
		},
		tokenDecimals: {
			USDC: 6,
			USDT: 6,
		},
		tokenStorageSlots: {
			USDT: { balanceSlot: 2, allowanceSlot: 5 },
			USDC: { balanceSlot: 9, allowanceSlot: 10 },
			WETH: { balanceSlot: 3, allowanceSlot: 4 },
			DAI: { balanceSlot: 0, allowanceSlot: 0 },
		},
		addresses: {
			IntentGateway: "0x1a4ee689a004b10210a1df9f24a387ea13359acf",
			TokenGateway: "0xFd413e3AFe560182C4471F4d143A96d3e259B6dE",
			Host: "0x792A6236AF69787C40cF76b69B4c8c7B28c4cA20",
			UniswapRouter02: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",
			UniswapV2Factory: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f",
			UniswapV3Factory: "0x1F98431c8aD98523631AE4a59f267346ea31F984",
			UniversalRouter: "0x66a9893cc07d91d95644aedd05d03f95e1dba8af",
			UniswapV3Quoter: "0x61fFE014bA17989E743c5F6cB21bF9697530B21e",
			UniswapV4Quoter: "0x52f0e24d1c21c8a0cb1e5a5dd6198556bd9e1203",
			Calldispatcher: "0xc71251c8b3e7b02697a84363eef6dce8dfbdf333",
			Permit2: "0x000000000022D473030F116dDEE9F6B43aC78BA3",
			EntryPointV08: "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108",
			Usdt0Oft: "0x6C96dE32CEa08842dcc4058c14d3aaAD7Fa41dee",
		},
		rpcEnvKey: "ETH_MAINNET",
		defaultRpcUrl: "https://eth-mainnet.g.alchemy.com/v2/demo",
		consensusStateId: "ETH0",
		coingeckoId: "ethereum",
		layerZeroEid: 30101,
		popularTokens: [
			"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
			"0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
			"0xdAC17F958D2ee523a2206206994597C13D831ec7",
			"0x6B175474E89094C44Da98b954EedeAC495271d0F",
		],
	},
	56: {
		chainId: 56,
		stateMachineId: Chains.BSC_MAINNET,
		viemChain: bsc,
		wrappedNativeDecimals: 18,
		assets: {
			WETH: "0xbb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c",
			DAI: "0x1af3f329e8be154074d8769d1ffa4ee058b1dbc3",
			USDC: "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d",
			USDT: "0x55d398326f99059ff775485246999027b3197955",
		},
		tokenDecimals: {
			USDC: 18,
			USDT: 18,
		},
		tokenStorageSlots: {
			USDT: { balanceSlot: 1, allowanceSlot: 2 },
			USDC: { balanceSlot: 1, allowanceSlot: 2 },
			WETH: { balanceSlot: 3, allowanceSlot: 4 },
			DAI: { balanceSlot: 0, allowanceSlot: 0 },
		},
		addresses: {
			IntentGateway: "0x1a4ee689a004b10210a1df9f24a387ea13359acf",
			TokenGateway: "0xFd413e3AFe560182C4471F4d143A96d3e259B6dE",
			Host: "0x24B5d421Ec373FcA57325dd2F0C074009Af021F7",
			UniswapRouter02: "0x10ED43C718714eb63d5aA57B78B54704E256024E",
			UniswapV2Factory: "0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73",
			UniswapV3Factory: "0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865",
			UniversalRouter: "0xd9C500DfF816a1Da21A48A732d3498Bf09dc9AEB",
			UniswapV3Quoter: "0xB048Bbc1Ee6b733FFfCFb9e9CeF7375518e25997",
			UniswapV4Quoter: "0xd0737C9762912dD34c3271197E362Aa736Df0926",
			Calldispatcher: "0xc71251c8b3e7b02697a84363eef6dce8dfbdf333",
			Permit2: "0x000000000022D473030F116dDEE9F6B43aC78BA3",
			EntryPointV08: "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108",
			// "Usdt0Oft": Not available on BSC
		},
		rpcEnvKey: "BSC_MAINNET",
		defaultRpcUrl: "https://binance.llamarpc.com",
		consensusStateId: "BSC0",
		coingeckoId: "binance-smart-chain",
		popularTokens: [
			"0x0E09FaBB73Bd3Ade0a17ECC321fD13a19e81cE82",
			"0x000Ae314E2A2172a039B26378814C252734f556A",
			"0x8d0d000ee44948fc98c9b98a4fa4921476f08b0d",
			"0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c",
			"0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d",
			"0x55d398326f99059fF775485246999027B3197955",
			"0x1AF3F329e8BE154074D8769D1FFa4eE058B1DBc3",
		],
	},
	42161: {
		chainId: 42161,
		stateMachineId: Chains.ARBITRUM_MAINNET,
		viemChain: arbitrum,
		wrappedNativeDecimals: 18,
		assets: {
			WETH: "0x82af49447d8a07e3bd95bd0d56f35241523fbab1",
			DAI: "0xda10009cbd5d07dd0cecc66161fc93d7c9000da1",
			USDC: "0xaf88d065e77c8cc2239327c5edb3a432268e5831",
			USDT: "0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9",
		},
		tokenDecimals: {
			USDC: 6,
			USDT: 6,
		},
		tokenStorageSlots: {
			USDT: { balanceSlot: 51, allowanceSlot: 52 },
			USDC: { balanceSlot: 9, allowanceSlot: 10 },
			WETH: { balanceSlot: 51, allowanceSlot: 52 },
			DAI: { balanceSlot: 0, allowanceSlot: 0 },
		},
		addresses: {
			IntentGateway: "0x1a4ee689a004b10210a1df9f24a387ea13359acf",
			TokenGateway: "0xFd413e3AFe560182C4471F4d143A96d3e259B6dE",
			Host: "0xE05AFD4Eb2ce6d65c40e1048381BD0Ef8b4B299e",
			UniswapRouter02: "0x4752ba5DBc23f44D87826276BF6Fd6b1C372aD24",
			UniswapV2Factory: "0xf1D7CC64Fb4452F05c498126312eBE29f30Fbcf9",
			UniswapV3Factory: "0x1F98431c8aD98523631AE4a59f267346ea31F984",
			UniversalRouter: "0xa51afafe0263b40edaef0df8781ea9aa03e381a3",
			UniswapV3Quoter: "0x61fFE014bA17989E743c5F6cB21bF9697530B21e",
			UniswapV4Quoter: "0x3972c00f7ed4885e145823eb7c655375d275a1c5",
			Calldispatcher: "0xc71251c8b3e7b02697a84363eef6dce8dfbdf333",
			Permit2: "0x000000000022D473030F116dDEE9F6B43aC78BA3",
			EntryPointV08: "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108",
			Usdt0Oft: "0x14E4A1B13bf7F943c8ff7C51fb60FA964A298D92",
		},
		rpcEnvKey: "ARBITRUM_MAINNET",
		defaultRpcUrl: "https://arbitrum-one.public.blastapi.io",
		consensusStateId: "ETH0",
		coingeckoId: "arbitrum-one",
		layerZeroEid: 30110,
		popularTokens: [
			"0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
			"0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
			"0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9",
			"0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1",
		],
	},
	8453: {
		chainId: 8453,
		stateMachineId: Chains.BASE_MAINNET,
		viemChain: base,
		wrappedNativeDecimals: 18,
		assets: {
			WETH: "0x4200000000000000000000000000000000000006",
			DAI: "0x50c5725949a6f0c72e6c4a641f24049a917db0cb",
			USDC: "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
			USDT: "0xfde4c96c8593536e31f229ea8f37b2ada2699bb2",
		},
		tokenDecimals: {
			USDC: 6,
			USDT: 6,
		},
		tokenStorageSlots: {
			USDT: { balanceSlot: 0, allowanceSlot: 1 },
			USDC: { balanceSlot: 9, allowanceSlot: 10 },
			WETH: { balanceSlot: 3, allowanceSlot: 4 },
			DAI: { balanceSlot: 0, allowanceSlot: 0 },
		},
		addresses: {
			IntentGateway: "0x1a4ee689a004b10210a1df9f24a387ea13359acf",
			TokenGateway: "0xFd413e3AFe560182C4471F4d143A96d3e259B6dE",
			Host: "0x6FFe92e4d7a9D589549644544780e6725E84b248",
			UniswapRouter02: "0x4752ba5DBc23f44D87826276BF6Fd6b1C372aD24",
			UniswapV2Factory: "0x8909Dc15e40173Ff4699343b6eB8132c65e18eC6",
			UniswapV3Factory: "0x33128a8fC17869897dcE68Ed026d694621f6FDfD",
			UniversalRouter: "0x6ff5693b99212da76ad316178a184ab56d299b43",
			UniswapV3Quoter: "0x3d4e44Eb1374240CE5F1B871ab261CD16335B76a",
			UniswapV4Quoter: "0x0d5e0f971ed27fbff6c2837bf31316121532048d",
			Calldispatcher: "0xc71251c8b3e7b02697a84363eef6dce8dfbdf333",
			Permit2: "0x000000000022D473030F116dDEE9F6B43aC78BA3",
			EntryPointV08: "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108",
			// Usdt0Oft: Not available on Base
		},
		rpcEnvKey: "BASE_MAINNET",
		defaultRpcUrl: "https://base-mainnet.public.blastapi.io",
		consensusStateId: "ETH0",
		coingeckoId: "base",
		layerZeroEid: 30184,
		popularTokens: [
			"0x4200000000000000000000000000000000000006",
			"0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
			"0xfde4C96c8593536E31F229EA8f37b2ADa2699bb2",
			"0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb",
		],
	},
	137: {
		chainId: 137,
		stateMachineId: Chains.POLYGON_MAINNET,
		viemChain: polygon,
		wrappedNativeDecimals: 18,
		assets: {
			WETH: "0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270",
			DAI: "0x8f3cf7ad23cd3cadbd9735aff958023239c6a063",
			USDC: "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359",
			USDT: "0xc2132d05d31c914a87c6611c10748aeb04b58e8f",
		},
		tokenDecimals: {
			USDC: 6,
			USDT: 6,
		},
		tokenStorageSlots: {
			USDT: { balanceSlot: 0, allowanceSlot: 1 },
			USDC: { balanceSlot: 9, allowanceSlot: 10 },
			WETH: { balanceSlot: 3, allowanceSlot: 4 },
			DAI: { balanceSlot: 0, allowanceSlot: 0 },
		},
		addresses: {
			IntentGateway: "0x1a4ee689a004b10210a1df9f24a387ea13359acf",
			TokenGateway: "0x8b536105b6Fae2aE9199f5146D3C57Dfe53b614E",
			Host: "0xD8d3db17C1dF65b301D45C84405CcAC1395C559a",
			UniswapRouter02: "0xd2f9496824951D5237cC71245D659E48d0d5f9E8",
			UniswapV2Factory: "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32",
			UniswapV3Factory: "0x1F98431c8aD98523631AE4a59f267346ea31F984",
			UniversalRouter: "0x1095692a6237d83c6a72f3f5efedb9a670c49223",
			UniswapV3Quoter: "0x61fFE014bA17989E743c5F6cB21bF9697530B21e",
			UniswapV4Quoter: "0xb3d5c3dfc3a7aebff71895a7191796bffc2c81b9",
			Calldispatcher: "0xc71251c8b3e7b02697a84363eef6dce8dfbdf333",
			Permit2: "0x000000000022D473030F116dDEE9F6B43aC78BA3",
			EntryPointV08: "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108",
			Usdt0Oft: "0x6BA10300f0DC58B7a1e4c0e41f5daBb7D7829e13",
		},
		rpcEnvKey: "POLYGON_MAINNET",
		defaultRpcUrl: "https://polygon-bor-rpc.publicnode.com",
		consensusStateId: "POLY",
		coingeckoId: "polygon-pos",
		layerZeroEid: 30109,
		popularTokens: [
			"0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270",
			"0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359",
			"0xc2132D05D31c914a87C6611C10748AEb04B58e8F",
			"0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063",
		],
	},
	130: {
		chainId: 130,
		stateMachineId: Chains.UNICHAIN_MAINNET,
		viemChain: unichain,
		wrappedNativeDecimals: 18,
		assets: {
			WETH: "0x4200000000000000000000000000000000000006",
			DAI: "0x0000000000000000000000000000000000000000",
			USDC: "0x078d782b760474a361dda0af3839290b0ef57ad6",
			USDT: "0x9151434b16b9763660705744891fa906f660ecc5",
		},
		tokenDecimals: {
			USDC: 6,
			USDT: 6,
		},
		addresses: {
			IntentGateway: "0x1a4ee689a004b10210a1df9f24a387ea13359acf",
			TokenGateway: "0x8b536105b6Fae2aE9199f5146D3C57Dfe53b614E",
			Host: "0x2A17C1c3616Bbc33FCe5aF5B965F166ba76cEDAf",
			UniswapRouter02: "0x284f11109359a7e1306c3e447ef14d38400063ff",
			UniswapV2Factory: "0x1F98400000000000000000000000000000000002",
			UniswapV3Factory: "0x1F98431c8aD98523631AE4a59f267346ea31F984",
			UniversalRouter: "0xef740bf23acae26f6492b10de645d6b98dc8eaf3",
			UniswapV3Quoter: "0x385a5cf5f83e99f7bb2852b6a19c3538b9fa7658",
			UniswapV4Quoter: "0x52f0e24d1c21c8a0cb1e5a5dd6198556bd9e1203",
			Calldispatcher: "0xc71251c8b3e7b02697a84363eef6dce8dfbdf333",
			Permit2: "0x000000000022D473030F116dDEE9F6B43aC78BA3",
			EntryPointV08: "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108",
			Usdt0Oft: "0xc07be8994d035631c36fb4a89c918cefb2f03ec3",
		},
		rpcEnvKey: "UNICHAIN_MAINNET",
		defaultRpcUrl: "https://unichain.api.onfinality.io/public",
		consensusStateId: "ETH0",
		coingeckoId: "ethereum",
		popularTokens: [
			"0x4200000000000000000000000000000000000006",
			"0x078d782b760474a361dda0af3839290b0ef57ad6",
			"0x9151434b16b9763660705744891fa906f660ecc5",
			"0x0000000000000000000000000000000000000000",
		],
	},

	3448148188: {
		chainId: 3448148188,
		stateMachineId: Chains.TRON_NILE,
		viemChain: tronNile,
		wrappedNativeDecimals: 6,
		assets: {
			WETH: "0x0000000000000000000000000000000000000000", // WTRX — TODO: fill in
			DAI: "0x0000000000000000000000000000000000000000",
			USDC: tronAddress("TNuoKL1ni8aoshfFL1ASca1Gou9RXwAzfn"),
			USDT: tronAddress("TXYZopYRdj2D9XRtbG411XZZ3kM5VkAeBf"),
		},
		tokenDecimals: {
			USDC: 6,
			USDT: 6,
		},
		addresses: {
			IntentGatewayV2: tronAddress("TT4CjjHw7QgLbE9wKtYEopid1YqePkbAfb"),
			Host: tronAddress("TNduR7v184pMWv2oTamRxxzsmz7oHrKqJc"),
			Calldispatcher: tronAddress("TA9XyBPuXL9ecXcLpcFV1g778fzstke2Eh"),
			UniswapRouter02: tronAddress("TLXGSird23Ww5FZrtbTYisrZNARUmjwmcy"),
		},
		rpcEnvKey: "TRON_NILE",
		defaultRpcUrl: "https://nile.trongrid.io/jsonrpc",
		consensusStateId: "TRON",
		coingeckoId: "tron",
	},

	80002: {
		chainId: 80002,
		stateMachineId: Chains.POLYGON_AMOY,
		viemChain: polygonAmoy,
		wrappedNativeDecimals: 18,
		assets: {
			WETH: "0x360ad4f9a9A8EFe9A8DCB5f461c4Cc1047E1Dcf9", //wmatic, change it to wpol
			DAI: "0x0000000000000000000000000000000000000000",
			USDC: "0x693b854d6965ffeaae21c74049dea644b56fcacb",
			USDT: "0x693B854D6965ffEAaE21C74049deA644b56FCaCB",
		},
		tokenDecimals: {
			USDC: 18,
			USDT: 18,
		},
		addresses: {
			IntentGatewayV2: "0xFbF50B2b32768127603cC9eF4b871574b881b8eD",
			TokenGateway: "0x8b536105b6Fae2aE9199f5146D3C57Dfe53b614E",
			Host: "0x9a2840D050e64Db89c90Ac5857536E4ec66641DE",
			Calldispatcher: "0x876F1891982E260026630c233A4897160A281Fb8",
			Permit2: "0x000000000022D473030F116dDEE9F6B43aC78BA3",
			EntryPointV08: "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108",
		},
		rpcEnvKey: "POLYGON_AMOY",
		defaultRpcUrl: "https://rpc-amoy.polygon.technology",
		consensusStateId: "POLY",
		coingeckoId: "polygon-pos",
	},
}

// Lookup by state machine ID
const configsByStateMachineId = Object.fromEntries(
	Object.values(chainConfigs).map((c) => [c.stateMachineId, c]),
) as Record<Chains, ChainConfigData>

export const getConfigByStateMachineId = (id: Chains): ChainConfigData | undefined => configsByStateMachineId[id]

export const getChainId = (stateMachineId: string): number | undefined =>
	configsByStateMachineId[stateMachineId as Chains]?.chainId

export const getViemChain = (chainId: number): Chain | undefined => chainConfigs[chainId]?.viemChain

export const hyperbridgeAddress = ""
