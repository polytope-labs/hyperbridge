///`AuthoritySetCommitment(uint256,uint256,bytes32)`
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
pub struct AuthoritySetCommitment {
	pub id: ::ethers::core::types::U256,
	pub len: ::ethers::core::types::U256,
	pub root: [u8; 32],
}
///`BeefyMmrLeaf(uint256,uint256,bytes32,(uint256,uint256,bytes32),bytes32,uint256,uint256)`
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
pub struct BeefyMmrLeaf {
	pub version: ::ethers::core::types::U256,
	pub parent_number: ::ethers::core::types::U256,
	pub parent_hash: [u8; 32],
	pub next_authority_set: AuthoritySetCommitment,
	pub extra: [u8; 32],
	pub k_index: ::ethers::core::types::U256,
	pub leaf_index: ::ethers::core::types::U256,
}
///`Commitment((bytes2,bytes)[],uint256,uint256)`
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
pub struct Commitment {
	pub payload: ::std::vec::Vec<Payload>,
	pub block_number: ::ethers::core::types::U256,
	pub validator_set_id: ::ethers::core::types::U256,
}
///`DispatchGet(bytes,uint64,bytes[],uint64,uint256,bytes)`
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
pub struct DispatchGet {
	pub dest: ::ethers::core::types::Bytes,
	pub height: u64,
	pub keys: ::std::vec::Vec<::ethers::core::types::Bytes>,
	pub timeout: u64,
	pub fee: ::ethers::core::types::U256,
	pub context: ::ethers::core::types::Bytes,
}
///`DispatchPost(bytes,bytes,bytes,uint64,uint256,address)`
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
pub struct DispatchPost {
	pub dest: ::ethers::core::types::Bytes,
	pub to: ::ethers::core::types::Bytes,
	pub body: ::ethers::core::types::Bytes,
	pub timeout: u64,
	pub fee: ::ethers::core::types::U256,
	pub payer: ::ethers::core::types::Address,
}
///`DispatchPostResponse((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64,uint256,
/// address)`
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
pub struct DispatchPostResponse {
	pub request: PostRequest,
	pub response: ::ethers::core::types::Bytes,
	pub timeout: u64,
	pub fee: ::ethers::core::types::U256,
	pub payer: ::ethers::core::types::Address,
}
///`GetRequest(bytes,bytes,uint64,address,uint64,bytes[],uint64,bytes)`
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
pub struct GetRequest {
	pub source: ::ethers::core::types::Bytes,
	pub dest: ::ethers::core::types::Bytes,
	pub nonce: u64,
	pub from: ::ethers::core::types::Address,
	pub timeout_timestamp: u64,
	pub keys: ::std::vec::Vec<::ethers::core::types::Bytes>,
	pub height: u64,
	pub context: ::ethers::core::types::Bytes,
}
///`GetResponse((bytes,bytes,uint64,address,uint64,bytes[],uint64,bytes),(bytes,bytes)[])`
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
pub struct GetResponse {
	pub request: GetRequest,
	pub values: ::std::vec::Vec<StorageValue>,
}
///`IncomingGetResponse(((bytes,bytes,uint64,address,uint64,bytes[],uint64,bytes),(bytes,bytes)[]),
/// address)`
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
pub struct IncomingGetResponse {
	pub response: GetResponse,
	pub relayer: ::ethers::core::types::Address,
}
///`IncomingPostRequest((bytes,bytes,uint64,bytes,bytes,uint64,bytes),address)`
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
pub struct IncomingPostRequest {
	pub request: PostRequest,
	pub relayer: ::ethers::core::types::Address,
}
///`IncomingPostResponse(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64),address)`
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
pub struct IncomingPostResponse {
	pub response: PostResponse,
	pub relayer: ::ethers::core::types::Address,
}
///`IntermediateState(uint256,uint256,(uint256,bytes32,bytes32))`
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
pub struct IntermediateState {
	pub state_machine_id: ::ethers::core::types::U256,
	pub height: ::ethers::core::types::U256,
	pub commitment: StateCommitment,
}
///`Payload(bytes2,bytes)`
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
pub struct Payload {
	pub id: [u8; 2],
	pub data: ::ethers::core::types::Bytes,
}
///`PostRequest(bytes,bytes,uint64,bytes,bytes,uint64,bytes)`
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
pub struct PostRequest {
	pub source: ::ethers::core::types::Bytes,
	pub dest: ::ethers::core::types::Bytes,
	pub nonce: u64,
	pub from: ::ethers::core::types::Bytes,
	pub to: ::ethers::core::types::Bytes,
	pub timeout_timestamp: u64,
	pub body: ::ethers::core::types::Bytes,
}
///`PostResponse((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64)`
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
pub struct PostResponse {
	pub request: PostRequest,
	pub response: ::ethers::core::types::Bytes,
	pub timeout_timestamp: u64,
}
///`StateCommitment(uint256,bytes32,bytes32)`
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
pub struct StateCommitment {
	pub timestamp: ::ethers::core::types::U256,
	pub overlay_root: [u8; 32],
	pub state_root: [u8; 32],
}
///`StateMachineHeight(uint256,uint256)`
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
pub struct StateMachineHeight {
	pub state_machine_id: ::ethers::core::types::U256,
	pub height: ::ethers::core::types::U256,
}
///`StorageValue(bytes,bytes)`
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
pub struct StorageValue {
	pub key: ::ethers::core::types::Bytes,
	pub value: ::ethers::core::types::Bytes,
}
