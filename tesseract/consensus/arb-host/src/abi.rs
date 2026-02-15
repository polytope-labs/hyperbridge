#![allow(clippy::all)]
#![allow(non_snake_case)]

use alloy::sol;

sol! {
    #[derive(Debug, PartialEq, Eq)]
    struct GlobalState {
        bytes32[2] bytes32Vals;
        uint64[2] u64Vals;
    }

    #[derive(Debug, PartialEq, Eq)]
    struct ExecutionState {
        GlobalState globalState;
        uint8 machineStatus;
    }

    #[derive(Debug, PartialEq, Eq)]
    struct Assertion {
        ExecutionState beforeState;
        ExecutionState afterState;
        uint64 numBlocks;
    }

    #[derive(Debug, PartialEq, Eq)]
    struct AssertionState {
        GlobalState globalState;
        uint8 machineStatus;
        bytes32 endHistoryRoot;
    }

    #[derive(Debug, PartialEq, Eq)]
    struct AssertionInputs {
        AssertionState beforeState;
        AssertionState afterState;
    }

    #[sol(rpc)]
    interface IRollup {
        event NodeCreated(
            uint64 indexed nodeNum,
            bytes32 indexed parentNodeHash,
            bytes32 indexed nodeHash,
            bytes32 executionHash,
            Assertion assertion,
            bytes32 afterInboxBatchAcc,
            bytes32 wasmModuleRoot,
            uint256 inboxMaxCount
        );
    }

    #[sol(rpc)]
    interface IRollupBold {
        event AssertionCreated(
            bytes32 indexed assertionHash,
            bytes32 indexed parentAssertionHash,
            AssertionInputs assertion,
            bytes32 afterInboxBatchAcc,
            uint256 inboxMaxCount,
            uint256 proposedAtTimestamp,
            address proposer,
            uint256 createdAtBlock
        );
    }
}
