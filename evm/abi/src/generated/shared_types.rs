///`GetRequest(bytes,bytes,uint64,bytes,uint64,bytes[],uint64)`
#[derive(
    Clone,
    ::ethers::contract::EthAbiType,
    ::ethers::contract::EthAbiCodec,
    Default,
    Debug,
    PartialEq,
    Eq,
    Hash
)]
pub struct GetRequest {
    pub source: ::ethers::core::types::Bytes,
    pub dest: ::ethers::core::types::Bytes,
    pub nonce: u64,
    pub from: ::ethers::core::types::Bytes,
    pub timeout_timestamp: u64,
    pub keys: ::std::vec::Vec<::ethers::core::types::Bytes>,
    pub height: u64,
}
///`GetResponse((bytes,bytes,uint64,bytes,uint64,bytes[],uint64),(bytes,bytes)[])`
#[derive(
    Clone,
    ::ethers::contract::EthAbiType,
    ::ethers::contract::EthAbiCodec,
    Default,
    Debug,
    PartialEq,
    Eq,
    Hash
)]
pub struct GetResponse {
    pub request: GetRequest,
    pub values: ::std::vec::Vec<StorageValue>,
}
///`IncomingGetResponse(((bytes,bytes,uint64,bytes,uint64,bytes[],uint64),(bytes,bytes)[]),address)`
#[derive(
    Clone,
    ::ethers::contract::EthAbiType,
    ::ethers::contract::EthAbiCodec,
    Default,
    Debug,
    PartialEq,
    Eq,
    Hash
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
    Hash
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
    Hash
)]
pub struct IncomingPostResponse {
    pub response: PostResponse,
    pub relayer: ::ethers::core::types::Address,
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
    Hash
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
    Hash
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
    Hash
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
    Hash
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
    Hash
)]
pub struct StorageValue {
    pub key: ::ethers::core::types::Bytes,
    pub value: ::ethers::core::types::Bytes,
}
