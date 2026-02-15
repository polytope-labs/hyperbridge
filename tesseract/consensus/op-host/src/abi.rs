#![allow(clippy::all)]
#![allow(non_snake_case)]

use alloy::sol;

sol! {
    #[sol(rpc)]
    #[derive(Debug)]
    interface L2OutputOracle {
        event OutputProposed(
            bytes32 indexed outputRoot,
            uint256 indexed l2OutputIndex,
            uint256 indexed l2BlockNumber,
            uint256 l1Timestamp
        );
    }

    #[sol(rpc)]
    #[derive(Debug)]
    interface DisputeGameFactory {
        event DisputeGameCreated(
            address indexed disputeProxy,
            uint32 indexed gameType,
            bytes32 indexed rootClaim
        );

        function gameCount() external view returns (uint256);
        function gameAtIndex(uint256 _index) external view returns (uint32 gameType_, uint64 timestamp_, address proxy_);
        function games(uint32 _gameType, bytes32 _rootClaim, bytes extraData) external view returns (address proxy_, uint64 timestamp_);
        function initBonds(uint32 _gameType) external view returns (uint256);
        function create(uint32 _gameType, bytes32 _rootClaim, bytes extraData) external payable returns (address proxy_);
    }

    #[sol(rpc)]
    interface FaultDisputeGame {
        function extraData() external pure returns (bytes memory extraData_);
        function createdAt() external view returns (uint64);
        function l2BlockNumber() external pure returns (uint256 l2BlockNumber_);
        function gameType() external view returns (uint32 gameType_);
        function rootClaim() external pure returns (bytes32 rootClaim_);
        function gameCreator() external pure returns (address creator_);
    }
}
