//! `ProtocolRevenueService`: dust the gateway collects and sweeps, per chain and token, priced
//! on-chain with Uniswap or the chain's Chainlink native feed.

use alloy::{
	eips::BlockId,
	primitives::{Address, Bytes, U256},
	sol,
};
use tokio_postgres::Transaction;

use super::orders::lower_hex;
use crate::{
	chains::Chain,
	config::Uniswap,
	errors::is_transient,
	services::{
		decimal::{amount_value_in_usd, to_scaled_usd, Dec},
		prices::native_currency_price,
	},
	store::{numeric, timestamp_to_date},
};

sol! {
	#[sol(rpc)]
	interface UniswapV2Factory {
		function getPair(address tokenA, address tokenB) external view returns (address pair);
	}

	#[sol(rpc)]
	interface UniswapV2Pair {
		function token0() external view returns (address);
		function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
	}

	#[sol(rpc)]
	interface UniswapV4Quoter {
		struct PoolKey { address currency0; address currency1; uint24 fee; int24 tickSpacing; address hooks; }
		struct QuoteExactSingleParams { PoolKey poolKey; bool zeroForOne; uint128 exactAmount; bytes hookData; }
		function quoteExactInputSingle(QuoteExactSingleParams params) external returns (uint256 amountOut, uint256 gasEstimate);
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dust {
	Collected,
	Swept,
}

impl Dust {
	fn tables(&self) -> (&'static str, &'static str) {
		match self {
			Dust::Collected => ("protocol_dust_collecteds", "cumulative_dust_collected_per_chains"),
			Dust::Swept => ("protocol_dust_swepts", "cumulative_dust_swept_per_chains"),
		}
	}
}

/**
 * `recordDustCollected` / `recordDustSwept`: adds `amount` to `<chain>-<token>` and, when the
 * dust has a USD value, its scaled value to the chain's total. `usd` is `amountValueInUSD`.
 * The symbol is only written with the row's first amount.
 */
#[allow(clippy::too_many_arguments)]
pub async fn record_dust(
	db: &Transaction<'_>,
	dust: Dust,
	state_machine: &str,
	token: Address,
	amount: U256,
	timestamp: u64,
	symbol: &str,
	usd: &str,
) -> anyhow::Result<()> {
	let (table, cumulative) = dust.tables();
	db.execute(
		&format!(
			"INSERT INTO app.{table} (id, chain, token_symbol, amount, last_updated) VALUES ($1, $2, $3, $4, $5)
			 ON CONFLICT (id) DO UPDATE SET amount = {table}.amount + EXCLUDED.amount,
			   last_updated = EXCLUDED.last_updated"
		),
		&[
			&format!("{state_machine}-{}", lower_hex(token)),
			&state_machine,
			&symbol,
			&numeric(amount),
			&timestamp_to_date(timestamp),
		],
	)
	.await?;

	if usd.is_empty() || Dec::parse(usd)?.is_zero() {
		tracing::warn!("no DEX price for {token} on {state_machine}; raw dust amount kept");
		return Ok(());
	}
	let scaled = to_scaled_usd(usd)?;
	if scaled.as_str() == "0" || scaled.as_str().starts_with('-') {
		return Ok(());
	}
	db.execute(
		&format!(
			"INSERT INTO app.{cumulative} (id, chain, amount_u_s_d, last_updated_at) VALUES ($1, $1, $2, $3)
			 ON CONFLICT (id) DO UPDATE SET amount_u_s_d = {cumulative}.amount_u_s_d + EXCLUDED.amount_u_s_d,
			   last_updated_at = EXCLUDED.last_updated_at"
		),
		&[&state_machine, &scaled, &numeric(timestamp)],
	)
	.await?;
	Ok(())
}

/**
 * A read that may fail for good: `None` when it does, as when a pool or a token's `symbol()`
 * doesn't exist. A transient failure is an error, so the batch retries instead of moving on.
 */
fn probe<T>(result: anyhow::Result<T>) -> anyhow::Result<Option<T>> {
	match result {
		Ok(value) => Ok(Some(value)),
		Err(err) if is_transient(&err) => Err(err),
		Err(_) => Ok(None),
	}
}

/**
 * The token's symbol and decimals for dust: `eth` with 18 for the native token, and `UNKNOWN`
 * with 18 when either read fails for good.
 */
pub async fn dust_token(chain: &Chain, token: Address, block: u64) -> anyhow::Result<(String, u8)> {
	if token == Address::ZERO {
		return Ok(("eth".into(), 18));
	}
	let metadata = crate::services::prices::token_metadata(chain, token, block).await;
	if let Err(err) = &metadata {
		tracing::warn!("dust token {token} metadata: {err:#}");
	}
	Ok(probe(metadata)?.unwrap_or_else(|| ("UNKNOWN".into(), 18)))
}

/** `getTokenPriceInUSDUniswap(...).amountValueInUSD`: "0" when no price can be read. */
pub async fn dust_usd_value(
	chain: &Chain,
	block: u64,
	token: Address,
	amount: U256,
	decimals: u8,
) -> anyhow::Result<String> {
	let price = if token == Address::ZERO {
		let native = native_currency_price(chain, block).await;
		if let Err(err) = &native {
			tracing::error!("native price on {}: {err:#}", chain.name);
		}
		match probe(native)?.map(|native| Dec::from_u256(native).div(&Dec::pow10(18))) {
			Some(Ok(price)) => price,
			_ => return Ok("0".into()),
		}
	} else {
		uniswap_price(chain, block, token, decimals).await?
	};
	Ok(amount_value_in_usd(amount, decimals, &price))
}

/**
 * `getUniswapPrice`: the first of V2 then V4 to quote the token against USDC, USDT, DAI or WETH
 * (WETH converted through WETH/USDC), else 0. The prices keep the TypeScript's unit errors: V2
 * divides raw reserves and V4 divides by the input token's decimals. Its V3 branch is skipped
 * because it could never return a price.
 */
pub async fn uniswap_price(
	chain: &Chain,
	block: u64,
	token: Address,
	decimals: u8,
) -> anyhow::Result<Dec> {
	let Some(uniswap) = chain.config.contracts.uniswap.as_ref() else {
		tracing::error!("no Uniswap addresses for {}", chain.name);
		return Ok(Dec::zero());
	};
	let quotes = [uniswap.usdc, uniswap.usdt, uniswap.dai, uniswap.weth];
	let at = BlockId::number(block);
	let factory = UniswapV2Factory::new(uniswap.v2_factory, &chain.provider);
	let pair_of = |a: Address, b: Address| {
		let call = factory.getPair(a, b);
		async move {
			let pair = probe(call.block(at).call().await.map_err(anyhow::Error::from))?;
			anyhow::Ok(pair.filter(|pair| *pair != Address::ZERO))
		}
	};

	for quote in quotes {
		let Some(pair) = pair_of(token, quote).await? else { continue };
		let Some(price) = probe(pair_price(chain, at, pair, quote).await)? else { continue };
		if quote == uniswap.weth {
			if let Some(weth_pair) = pair_of(uniswap.weth, uniswap.usdc).await? {
				match probe(pair_price(chain, at, weth_pair, uniswap.usdc).await)? {
					Some(weth_price) => return Ok(price.mul(&weth_price)),
					None => continue,
				}
			}
		}
		return Ok(price);
	}

	for quote in quotes {
		for fee in [500u32, 3000, 10000] {
			let price = v4_price(chain, at, uniswap, token, quote, fee, decimals).await;
			if let Some(price) = probe(price)? {
				return Ok(price);
			}
		}
	}
	tracing::error!("no Uniswap V2 or V4 price for {token} on {}", chain.name);
	Ok(Dec::zero())
}

/** `getPriceFromPair`: the quote token's raw reserve over the other's. */
async fn pair_price(
	chain: &Chain,
	at: BlockId,
	pair: Address,
	quote: Address,
) -> anyhow::Result<Dec> {
	let pair = UniswapV2Pair::new(pair, &chain.provider);
	let token0 = pair.token0().block(at).call().await?;
	let reserves = pair.getReserves().block(at).call().await?;
	let (reserve0, reserve1) = (U256::from(reserves.reserve0), U256::from(reserves.reserve1));
	let (reserve_in, reserve_out) =
		if token0 == quote { (reserve0, reserve1) } else { (reserve1, reserve0) };
	if reserve_in.is_zero() || reserve_out.is_zero() {
		anyhow::bail!("zero reserves");
	}
	Dec::from_u256(reserve_in).div(&Dec::from_u256(reserve_out))
}

async fn v4_quote(
	chain: &Chain,
	at: BlockId,
	quoter: Address,
	key: UniswapV4Quoter::PoolKey,
	zero_for_one: bool,
	amount: u128,
) -> anyhow::Result<U256> {
	let quoter = UniswapV4Quoter::new(quoter, &chain.provider);
	let params = UniswapV4Quoter::QuoteExactSingleParams {
		poolKey: key,
		zeroForOne: zero_for_one,
		exactAmount: amount,
		hookData: Bytes::new(),
	};
	Ok(quoter.quoteExactInputSingle(params).block(at).call().await?.amountOut)
}

fn pool_key(currency0: Address, currency1: Address, fee: u32) -> UniswapV4Quoter::PoolKey {
	let tick_spacing = match fee {
		500 => 10,
		3000 => 60,
		_ => 200,
	};
	UniswapV4Quoter::PoolKey {
		currency0,
		currency1,
		fee: alloy::primitives::Uint::from(fee),
		tickSpacing: alloy::primitives::Signed::try_from(tick_spacing).expect("fits int24"),
		hooks: Address::ZERO,
	}
}

#[allow(clippy::too_many_arguments)]
async fn v4_price(
	chain: &Chain,
	at: BlockId,
	uniswap: &Uniswap,
	token: Address,
	quote: Address,
	fee: u32,
	decimals: u8,
) -> anyhow::Result<Dec> {
	let unit = Dec::pow10(decimals as u32);
	let amount = 10u128.checked_pow(decimals as u32).ok_or_else(|| anyhow::anyhow!("decimals"))?;
	let (currency0, currency1) = if token < quote { (token, quote) } else { (quote, token) };
	let out = v4_quote(
		chain,
		at,
		uniswap.v4_quoter,
		pool_key(currency0, currency1, fee),
		token == currency0,
		amount,
	)
	.await?;
	let mut price = Dec::from_u256(out).div(&unit)?;
	if quote == uniswap.weth {
		let weth_key = pool_key(uniswap.weth, uniswap.usdc, 500);
		let weth_out = v4_quote(chain, at, uniswap.v4_quoter, weth_key, true, amount).await?;
		price = price.mul(&Dec::from_u256(weth_out).div(&unit)?);
	}
	Ok(price)
}
