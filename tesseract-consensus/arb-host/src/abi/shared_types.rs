///`GlobalState(bytes32[2],uint64[2])`
#[derive(
	Clone,
	::ethers::contract::EthAbiType,
	::ethers::contract::EthAbiCodec,
	Default,
	Debug,
	PartialEq,
	Eq,
	Hash,
)]
pub struct GlobalState {
	pub bytes_32_vals: [[u8; 32]; 2],
	pub u_64_vals: [u64; 2],
}
///`Staker(uint256,uint64,uint64,uint64,bool)`
#[derive(
	Clone,
	::ethers::contract::EthAbiType,
	::ethers::contract::EthAbiCodec,
	Default,
	Debug,
	PartialEq,
	Eq,
	Hash,
)]
pub struct Staker {
	pub amount_staked: ::ethers::core::types::U256,
	pub index: u64,
	pub latest_staked_node: u64,
	pub current_challenge: u64,
	pub is_staked: bool,
}
