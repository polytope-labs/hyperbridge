import { ChainConfigService } from "@/configs/ChainConfigService"
import type { HexString } from "@/types"
import { UniswapQuoteEngine, type UniswapQuoteAdapter, type UniswapQuoteToken } from "@/utils/uniswapQuote"
import type { PublicClient } from "viem"
import { describe, expect, it } from "vitest"

const TOKEN_IN: UniswapQuoteToken = {
	address: "0x1111111111111111111111111111111111111111",
	decimals: 6,
	symbol: "USDC",
	chainId: 8453,
}

const TOKEN_OUT: UniswapQuoteToken = {
	address: "0x2222222222222222222222222222222222222222",
	decimals: 6,
	symbol: "cNGN",
	chainId: 8453,
}

class FixedQuoteAdapter implements UniswapQuoteAdapter {
	constructor(private readonly expectedClient: PublicClient) {}

	async findBestProtocolWithAmountIn(
		client: PublicClient,
		_tokenIn: HexString,
		_tokenOut: HexString,
		_amountIn: bigint,
		_evmChainID: string,
		options?: { selectedProtocol?: "v2" | "v3" | "v4"; generateCalldata?: boolean; recipient?: HexString },
	) {
		expect(client).toBe(this.expectedClient)

		switch (options?.selectedProtocol) {
			case "v2":
				return { protocol: "v2" as const, amountOut: 95n }
			case "v3":
				return { protocol: "v3" as const, amountOut: 101n, fee: 500 }
			case "v4":
				return { protocol: "v4" as const, amountOut: 103n, fee: 1500 }
			default:
				return { protocol: null, amountOut: 0n }
		}
	}

	async findBestProtocolWithAmountOut(): Promise<never> {
		throw new Error("Unused by exact-input quotes")
	}

	createV2SwapCalldataExactIn(): never {
		throw new Error("Unused without recipient")
	}

	createV2SwapCalldataExactOut(): never {
		throw new Error("Unused by exact-input quotes")
	}

	createV3SwapCalldataExactIn(): never {
		throw new Error("Unused without recipient")
	}

	createV3SwapCalldataExactOut(): never {
		throw new Error("Unused by exact-input quotes")
	}

	createV4SwapCalldataExactIn(): never {
		throw new Error("Unused without recipient")
	}

	createV4SwapCalldataExactOut(): never {
		throw new Error("Unused by exact-input quotes")
	}
}

describe("UniswapQuoteEngine", () => {
	it("returns the best exact-input quote across selected protocols", async () => {
		const client = { name: "uniswap-quote-test-client" } as unknown as PublicClient
		const quoteEngine = new UniswapQuoteEngine(new FixedQuoteAdapter(client), new ChainConfigService({}))

		const result = await quoteEngine.quote(
			{
				chainId: 8453,
				tokenIn: TOKEN_IN,
				tokenOut: TOKEN_OUT,
				amountIn: 100n,
				tradeType: "EXACT_INPUT",
				protocols: ["v2", "v3", "v4"],
			},
			{ client },
		)

		expect(result.quotes.map((quote) => quote.protocol)).toEqual(["v2", "v3", "v4"])
		expect(result.bestQuote?.protocol).toBe("v4")
		expect(result.bestQuote?.amountOut).toBe(103n)
		expect(result.bestQuote?.fee).toBe(1500)
	})
})
