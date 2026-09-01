// Sub-path entry for tools (e.g. SubQuery indexer) that need intents encoding
// helpers without the full SDK bundle, which includes TronWeb and crashes in VM2.
export { decodeERC7821ExecuteBatch, encodeERC7821ExecuteBatch } from "@/protocols/intents/decode-utils"
export { decodeUserOpScale, encodeUserOpScale } from "@/chains/intentsCoprocessor"
export { default as IntentGatewayV2 } from "@/abis/IntentGatewayV2"
export { poolSlug, sortPoolSymbols } from "@/protocols/intents/liquidity-pool"
// Only the ABI constant — `decodeFillOrder` itself is viem-based and unusable in VM2.
export { FILL_ORDER_V1_ABI } from "@/protocols/intents/fillOrderCodec"
export {
	aggregatePhantomBids,
	applyPhantomQuoteHaircut,
	applyUniswapQuoteHaircut,
	decodeAcceptedSourceChains,
	decodePhantomBidDeclaration,
	encodeAcceptedSourceChains,
	encodePhantomBidDeclaration,
	extractFillData,
	fetchBidsForOrder,
	getTotalSolverBalance,
	memoizedSolverBalance,
	readV4Position,
	orderCommitmentFromDecoded,
	recoverBidSignerViem,
	setAggregationFetch,
	splitBidSignature,
	weightedMedian,
	zipFillLegs,
	ENTRY_POINT_V08_ADDRESS,
	PHANTOM_QUOTE_HAIRCUT_BPS,
	UNISWAP_QUOTE_HAIRCUT_BPS,
	FILL_ORDER_ABI,
	type AggregationLogger,
	type BidNonceKeyFn,
	type BidSignature,
	type FetchLike,
	type FillData,
	type HexString,
	type LpBalance,
	type OrderCommitmentFn,
	type PhantomAggregation,
	type PhantomLegAggregation,
	type PhantomLegBidder,
	type RecoverBidSigner,
	type SolverBalanceReader,
	type RpcBidInfo,
	type UniswapV4Contracts,
	type V4PositionState,
	type YieldVaultMap,
} from "@/protocols/intents/phantom-aggregation"
// Pure position arithmetic, so a consumer re-valuing persisted positions between bid windows uses
// the same maths the aggregation weights them with.
export { positionAmountOfToken, type PoolAndPositionInfo } from "@/protocols/intents/uniswap-v4-position"
