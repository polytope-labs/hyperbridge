# The BRIDGE token's relayer gate, and why the base token has none

Verified against `contracts/apps/HyperFungibleToken.sol` and `evm/src/apps/BridgeToken.sol`, and
exercised by `evm/tests/foundry/BridgeTokenTest.t.sol`.

Steps 1 and 2 above are identical; the token is just another `IApp`. Timeouts take a parallel route:
`HandlerV2.handlePostRequestTimeouts` calls `host.dispatchTimeOut(PostRequestTimeout(request,
_msgSender()), ...)`, and the host calls `onPostRequestTimeout` on the module.

3. `HyperFungibleToken.onAccept` and `onPostRequestTimeout` are `public virtual`, run `onlyHost`
   and `whenNotPaused`, then check the source against `_supportedChains`, decode the body, and
   mint. The base token knows nothing about relayers: a third-party token accepts every relayer.

`BridgeToken` overrides both callbacks: `onlyHost`, then `_checkRelayer(incoming.relayer)`, then
`super`. `_checkRelayer` reverts with `BridgeToken.UnauthorizedRelayer` whenever the incoming relayer
differs from `_relayer`, so with none set nothing can mint; the deploy script calls `setRelayer`
before `configure`, and before `configure` the token cannot be reached at all since `onlyHost`
compares against an unset `_host`. `setRelayer` is `onlyOwner`; the host is not the owner and
never calls the token with anything but the callback selectors. The token is not behind a proxy,
so there is no upgrade transaction to arm it in and no host-only setter like the gateway's.
