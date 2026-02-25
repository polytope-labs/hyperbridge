#![allow(clippy::all)]
#![allow(non_snake_case)]

use alloy_sol_macro::sol;

sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	#[derive(Debug)]
	L2OutputOracle,
	"abis/L2OutputOracle.json"
);

sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	#[derive(Debug)]
	DisputeGameFactory,
	"abis/DisputeGameFactory.json"
);

sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	#[derive(Debug)]
	FaultDisputeGame,
	"abis/FaultDisputeGame.json"
);
