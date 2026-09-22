// Wire contracts and config types come straight from src/ — type-only
// imports, erased at build, so the server and the browser cannot drift.

export type { InitChainMeta as ChainDefault, InitNetwork as Network } from "@/cli/init/chains"
// The wizard writes the binary's config file, signer block included — that is
// `FillerConfigFile`, not the library's `FillerTomlConfig`.
export type { FillerConfigFile as FillerConfig } from "@/config/filler-toml"
export type { CurvePoint } from "@/config/interpolated-curve"
export type { PairConfig } from "@/config/pairs"
// The limit-order routes answer with the stored rows themselves rather than a
// DTO, so the browser reads the same shape the store writes.
export type { LimitOrder, LimitOrderFill, LimitOrderSide, LimitOrderStatus, StoredBid } from "@/data/types"
export type { CreateLimitOrderRequest } from "@/orderbook/limit-orders"
export type {
	ActivityEventDto,
	OrderHistoryDto,
	OrderLeg,
	OrderSummary,
	AdminStrategyDto,
	BalanceSnapshot,
	BidDto,
	BidStatsDto,
	ChainRowDto,
	ChainsDto,
	ConfigDto,
	KnownToken,
	KnownVault,
	LedgerLeg,
	LogRecordDto,
	LogRecordLevel,
	LogsDto,
	SendTokenOption,
	SetupDefaults,
	SolverWork,
	Status,
	StatusInit,
	StatusOperator,
	TunnelConnectionDto,
	TunnelDeviceDto,
	TunnelNewDeviceDto,
	TunnelStatusDto,
	VaultSweepDto,
	WalletTxDto,
} from "@/services/server/dto"
