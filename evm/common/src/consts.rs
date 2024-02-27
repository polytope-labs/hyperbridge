// EvmHost slot info
// | Name                        | Type                                                           | Slot | Offset | Bytes | Contract                |
// |-----------------------------|----------------------------------------------------------------|------|--------|-------|-------------------------|
// | _requestCommitments         | mapping(bytes32 => bool)                                       | 0    | 0      | 32    | src/EvmHost.sol:EvmHost |
// | _responseCommitments        | mapping(bytes32 => bool)                                       | 1    | 0      | 32    | src/EvmHost.sol:EvmHost |
// | _requestReceipts            | mapping(bytes32 => bool)                                       | 2    | 0      | 32    | src/EvmHost.sol:EvmHost |
// | _responseReceipts           | mapping(bytes32 => bool)                                       | 3    | 0      | 32    | src/EvmHost.sol:EvmHost |
// | _stateCommitments           | mapping(uint256 => mapping(uint256 => struct StateCommitment)) | 4    | 0      | 32    | src/EvmHost.sol:EvmHost |
// | _stateCommitmentsUpdateTime | mapping(uint256 => mapping(uint256 => uint256))                | 5    | 0      | 32    | src/EvmHost.sol:EvmHost |
// | _latestStateMachineHeight   | uint256                                                        | 6    | 0      | 32    | src/EvmHost.sol:EvmHost |
// | _hostParams                 | struct HostParams                                              | 7    | 0      | 288   | src/EvmHost.sol:EvmHost |
// | _nonce                      | uint256                                                        | 16   | 0      | 32    | src/EvmHost.sol:EvmHost |
// | _frozen                     | bool                                                           | 17   | 0      | 1     | src/EvmHost.sol:EvmHost |

pub const REQUEST_COMMITMENTS_SLOT: u64 = 0;
pub const RESPONSE_COMMITMENTS_SLOT: u64 = 1;
pub const REQUEST_RECEIPTS_SLOT: u64 = 2;
pub const RESPONSE_RECEIPTS_SLOT: u64 = 3;
pub const LATEST_STATE_MACHINE_HEIGHT_SLOT: u64 = 6;
pub const HOST_PARAMS_SLOT: u64 = 7;
