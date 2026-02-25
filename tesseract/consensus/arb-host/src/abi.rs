#![allow(clippy::all)]
#![allow(non_snake_case)]

use alloy_sol_macro::sol;

sol!(
	#[allow(missing_docs)]
	#[derive(Debug, PartialEq, Eq)]
	IRollup,
	"abis/IRollupCore.json"
);

sol!(
	#[allow(missing_docs)]
	#[derive(Debug, PartialEq, Eq)]
	IRollupBold,
	"abis/IRollupCoreBold.json"
);
