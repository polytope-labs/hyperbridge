# EVM OwnableHandler and OwnableHostManager

## OwnableHandler

`OwnableHandler` is a `HandlerV2` that adds one owner-only method. The owner ("admin") is
set in the constructor and can call `setConsensusState(host, state, height, commitment)` to
write a new consensus state into the host without a consensus proof. The call also stores
`commitment` at `height` and sets the latest height of `height.stateMachineId` to
`height.height`.

Resetting the latest height matters because `handleConsensus` only stores an intermediate
commitment when its height is above the latest height of that state machine. Without the
reset, proofs relayed at or below the previous height would be skipped. Writing the state
also refreshes the consensus update time, so a client that passed its unstaking period can
be made usable again.

A reset does not delete commitments already stored above the new height.
`handlePostRequests`, `handleGetResponses` and the timeout handlers read a commitment by its
exact height, so requests proven against those older commitments are still accepted.

The owner is fully trusted: it can write any state root and overlay root, so it can make the
host accept any incoming message.

Every other handler method is inherited from `HandlerV2` unchanged and stays permissionless,
including `handleConsensus`, `handlePostRequests`, `handleGetResponses`, the timeout
handlers, and `batchCall`. `batchCall` delegatecalls the handler itself, so the owner check
still applies to `setConsensusState` inside a batch.

## OwnableHostManager

`OwnableHostManager` is a `HostManager` whose owner can call `updateHostParams(params)` to
replace the host params without a governance request from Hyperbridge. The call forwards to
`IHostManager(host()).updateHostParams(params)`, which the host accepts because this contract
is its configured `hostManager`; the host still validates the new params.

The owner is a separate constructor argument from `HostManagerParams.admin`, the relayer that
delivers governance, and either can be rotated without affecting the other. There is no owner
`withdraw` method, but the owner can replace `hostManager`, `handler` and `consensusClient`
through `updateHostParams`, so it is fully trusted with the host; a `hostManager` it controls
could withdraw. The inherited `onAccept` (Withdraw, SetHostParam, SetAdmin) and `init` work as
before.

Neither contract is wired into any deploy script.
