// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ERC4337Utils, PackedUserOperation} from "@openzeppelin/contracts/account/utils/draft-ERC4337Utils.sol";
import {PaymasterERC20} from "@openzeppelin/community-contracts/contracts/account/paymaster/PaymasterERC20.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {IERC20Metadata} from "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";
import {IERC20Permit} from "@openzeppelin/contracts/token/ERC20/extensions/IERC20Permit.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {Initializable} from "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import {ERC1967Utils} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Utils.sol";
import {HyperApp} from "@hyperbridge/core/apps/HyperApp.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";
import {IDispatcher} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {IUniswapV2Router02} from "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";

/// @notice Minimal Chainlink AggregatorV3 interface — no external dependency needed.
interface AggregatorV3Interface {
    function latestRoundData()
        external
        view
        returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound);

    function decimals() external view returns (uint8);
}

/// @title  SimplexPaymaster
/// @author Polytope Labs
/// @notice Fully onchain, permissionless ERC-4337 v0.8 paymaster that accepts
///         ERC-20 stablecoins (USDC, USDT, or any token with a Chainlink feed)
///         for gas payment. Deployed behind an ERC1967Proxy and administered
///         exclusively through Hyperbridge governance.
///
/// Modes (byte 0 of paymasterData):
///   0x00  PERMIT  — EIP-2612 permit signature included; the permit is executed
///                    during validation so the subsequent prefund transferFrom
///                    succeeds without a prior onchain approval.
///   0x01  APPROVE — Token must be pre-approved to this paymaster (the path for
///                    tokens without permit support, e.g. BSC stablecoins).
///
/// paymasterData encoding:
///   Mode 0x00 (permit):
///     abi.encodePacked(uint8(0), address(token), uint256(permitAmount),
///                      uint256(deadline), uint8(v), bytes32(r), bytes32(s))
///   Mode 0x01 (approve):
///     abi.encodePacked(uint8(1), address(token))
///
/// Price conversion uses two Chainlink feeds: token/USD and nativeAsset/USD.
/// The markup surplus accumulates in the contract and is withdrawable to the
/// treasury; unused gas is refunded to the sender by PaymasterERC20._postOp.
///
/// @dev Security model. Solvers grant this contract ERC-20 allowances, so a
///      compromise must never translate into large withdrawals from their
///      accounts. There is no privileged key: every administrative action —
///      upgrades, parameter changes, token registry, withdrawals — is an
///      onAccept request authenticated as originating from Hyperbridge
///      governance and delivered by the local host. Clients additionally keep
///      allowances and permit amounts small (a few dollars), bounding exposure
///      to the residual allowance even against a malicious oracle.
contract SimplexPaymaster is Initializable, HyperApp, PaymasterERC20 {
    using SafeERC20 for IERC20;
    using ERC4337Utils for PackedUserOperation;

    enum RequestKind {
        /// @dev Points the ERC-1967 proxy at a new implementation, optionally calling it.
        UpgradeContract,
        /// @dev Replaces the pricing and treasury parameters.
        UpdateParams,
        /// @dev Registers or updates a supported ERC-20 token and its token/USD feed.
        RegisterToken,
        /// @dev Deactivates a token (stops new UserOps from using it).
        DeactivateToken,
        /// @dev Sweeps accumulated ERC-20 surplus or the EntryPoint deposit to the treasury.
        WithdrawAssets
    }

    struct Params {
        /// @notice Native asset / USD oracle (BNB/USD on BSC, ETH/USD on Ethereum, etc.)
        AggregatorV3Interface nativeOracle;
        /// @notice Markup in basis points (100 = 1%). Applied on top of the oracle price.
        uint256 markupBps;
        /// @notice Receives markup surplus and EntryPoint deposit withdrawals.
        address treasury;
        /// @notice Maximum oracle staleness. Chainlink heartbeats vary per chain
        ///         (BSC stablecoins ~27s, Base/Ethereum stablecoins up to 24h).
        uint256 maxOracleAge;
        /// @notice V2-compatible router used by {swapAndDeposit} to recycle
        ///         accrued stablecoins into the EntryPoint deposit. Must
        ///         implement `swapExactTokensForETH` and `WETH()` (the host's
        ///         configured wrapper routers only sell ETH and won't work).
        ///         Zero address disables swapping.
        address uniswapV2Router;
        /// @notice Slippage tolerance in basis points applied to the
        ///         oracle-derived expected output in {swapAndDeposit}.
        uint256 swapSlippageBps;
    }

    struct TokenConfig {
        AggregatorV3Interface tokenOracle; // token/USD feed
        uint8 tokenOracleDecimals; // cached decimals() of tokenOracle
        uint8 tokenDecimals; // decimals() of the ERC-20
        bool active; // kill-switch per token
    }

    /// @dev Hard cap on the governance-configurable markup (50%).
    uint256 public constant MAX_MARKUP_BPS = 5_000;

    /// @dev Hard ceiling on the governance-configurable oracle staleness bound.
    uint256 public constant MAX_ORACLE_AGE = 7 days;

    /// @dev Hard cap on the governance-configurable swap slippage (10%).
    uint256 public constant MAX_SWAP_SLIPPAGE_BPS = 1_000;

    /// @dev Caps the caller-supplied postOp gas limit. Unbounded, the EntryPoint's
    ///      unused-gas penalty is drained from this contract's deposit to a
    ///      caller-chosen beneficiary; the cap keeps that penalty under the
    ///      `_postOpCost` cushion the user already pays.
    uint256 public constant MAX_POST_OP_GAS_LIMIT = 100_000;

    /// @notice The local Hyperbridge host; the only address allowed to deliver
    ///         governance requests.
    address private _hostAddr;

    AggregatorV3Interface public nativeOracle;
    uint8 public nativeOracleDecimals;
    uint256 public maxOracleAge;
    uint256 public markupBps;
    address public treasury;

    mapping(address => TokenConfig) public tokenConfigs;

    /// @notice Set of registered token addresses (for enumeration).
    address[] public registeredTokens;

    address public uniswapV2Router;
    uint256 public swapSlippageBps;

    uint256[48] private __gap;

    event TokenRegistered(address indexed token, address indexed oracle);
    event TokenDeactivated(address indexed token);
    event ParamsUpdated(Params previous, Params current);
    event PermitExecuted(address indexed token, address indexed owner, uint256 amount);
    event FeesRecycled(address indexed token, uint256 amountIn, uint256 nativeOut, uint256 deposited);

    error TokenNotRegistered(address token);
    error TokenNotActive(address token);
    error StaleOraclePrice(address oracle, uint256 updatedAt);
    error InvalidOraclePrice(address oracle, int256 price);
    error InvalidMarkup(uint256 bps);
    error InvalidOracleAge(uint256 age);
    error InvalidSlippage(uint256 bps);
    error InvalidRouter(address router);
    error InvalidMode(uint8 mode);
    error InvalidPaymasterData(uint256 length);
    error InvalidPostOpGasLimit(uint256 supplied, uint256 maximum);
    error PermitFailed(address token);
    error ZeroAddress();
    error InvalidHost();
    error LengthMismatch();

    constructor() {
        _disableInitializers();
    }

    /// @param host_    Local Hyperbridge host, sole deliverer of governance requests
    /// @param params_  Initial pricing and treasury parameters
    /// @param tokens_  Initially supported ERC-20 tokens
    /// @param oracles_ token/USD feed for each entry in tokens_
    function initialize(
        address host_,
        Params memory params_,
        address[] memory tokens_,
        AggregatorV3Interface[] memory oracles_
    ) external initializer {
        if (host_ == address(0) || host_.code.length == 0) revert InvalidHost();
        if (tokens_.length != oracles_.length) revert LengthMismatch();

        _hostAddr = host_;
        _setParams(params_);

        for (uint256 i = 0; i < tokens_.length; i++) {
            _registerToken(tokens_[i], oracles_[i]);
        }
    }

    // ── Governance ───────────────────────────────────────────────────

    function host() public view override returns (address) {
        return _hostAddr;
    }

    /// @dev Handles governance requests delivered by the local host. The first
    ///      byte of the request body encodes the `RequestKind`; only requests
    ///      originating from Hyperbridge itself are accepted.
    function onAccept(IncomingPostRequest calldata incoming) external override onlyHost {
        if (keccak256(incoming.request.source) != keccak256(IDispatcher(host()).hyperbridge())) {
            revert UnauthorizedCall();
        }

        RequestKind kind = RequestKind(uint8(incoming.request.body[0]));
        bytes calldata payload = incoming.request.body[1:];

        if (kind == RequestKind.UpgradeContract) {
            (address newImpl, bytes memory initData) = abi.decode(payload, (address, bytes));
            ERC1967Utils.upgradeToAndCall(newImpl, initData);
        } else if (kind == RequestKind.UpdateParams) {
            _setParams(abi.decode(payload, (Params)));
        } else if (kind == RequestKind.RegisterToken) {
            (address token, address oracle) = abi.decode(payload, (address, address));
            _registerToken(token, AggregatorV3Interface(oracle));
        } else if (kind == RequestKind.DeactivateToken) {
            _deactivateToken(abi.decode(payload, (address)));
        } else if (kind == RequestKind.WithdrawAssets) {
            (address token, uint256 amount) = abi.decode(payload, (address, uint256));
            _withdrawAssets(token, amount);
        }
    }

    /// @dev Validates and applies pricing/treasury parameters, re-caching the
    ///      native oracle decimals.
    function _setParams(Params memory p) internal {
        if (address(p.nativeOracle) == address(0)) revert ZeroAddress();
        if (p.treasury == address(0)) revert ZeroAddress();
        if (p.markupBps > MAX_MARKUP_BPS) revert InvalidMarkup(p.markupBps);
        if (p.maxOracleAge == 0 || p.maxOracleAge > MAX_ORACLE_AGE) revert InvalidOracleAge(p.maxOracleAge);
        if (p.uniswapV2Router != address(0) && p.uniswapV2Router.code.length == 0) {
            revert InvalidRouter(p.uniswapV2Router);
        }
        if (p.swapSlippageBps > MAX_SWAP_SLIPPAGE_BPS) revert InvalidSlippage(p.swapSlippageBps);

        emit ParamsUpdated(
            Params({
                nativeOracle: nativeOracle,
                markupBps: markupBps,
                treasury: treasury,
                maxOracleAge: maxOracleAge,
                uniswapV2Router: uniswapV2Router,
                swapSlippageBps: swapSlippageBps
            }),
            p
        );

        nativeOracle = p.nativeOracle;
        nativeOracleDecimals = p.nativeOracle.decimals();
        markupBps = p.markupBps;
        treasury = p.treasury;
        maxOracleAge = p.maxOracleAge;
        uniswapV2Router = p.uniswapV2Router;
        swapSlippageBps = p.swapSlippageBps;
    }

    /// @dev Registers or updates a supported ERC-20 token with its token/USD feed.
    ///      Re-registering is also the recovery path for a misbehaving oracle.
    function _registerToken(address token, AggregatorV3Interface oracle) internal {
        if (token == address(0) || address(oracle) == address(0)) revert ZeroAddress();

        bool isNew = !tokenConfigs[token].active && address(tokenConfigs[token].tokenOracle) == address(0);

        tokenConfigs[token] = TokenConfig({
            tokenOracle: oracle,
            tokenOracleDecimals: oracle.decimals(),
            tokenDecimals: IERC20Metadata(token).decimals(),
            active: true
        });

        if (isNew) {
            registeredTokens.push(token);
        }

        emit TokenRegistered(token, address(oracle));
    }

    function _deactivateToken(address token) internal {
        tokenConfigs[token].active = false;
        emit TokenDeactivated(token);
    }

    /// @dev Sweeps assets to the treasury: ERC-20 surplus, or the EntryPoint
    ///      deposit when `token` is the zero address.
    function _withdrawAssets(address token, uint256 amount) internal {
        if (token == address(0)) {
            entryPoint().withdrawTo(payable(treasury), amount);
        } else {
            IERC20(token).safeTransfer(treasury, amount);
        }
    }

    /// @dev Withdrawals only happen through governance (see {_withdrawAssets});
    ///      the inherited public withdraw and stake entry points are disabled.
    function _authorizeWithdraw() internal pure override {
        revert UnauthorizedCall();
    }

    // ── Fee recycling ────────────────────────────────────────────────

    /// @notice Swaps accrued stablecoins to the native asset through the
    ///         configured V2-style router and deposits the contract's entire
    ///         native balance into the EntryPoint, so collected fees keep the
    ///         paymaster funded without a governance round-trip.
    /// @param token    A registered token; deactivated tokens remain recyclable.
    /// @param amountIn Token amount to swap; 0 (or more than the balance)
    ///                 swaps the full balance.
    /// @dev The minimum output is derived onchain from the Chainlink oracles
    ///      (markup-free price minus `swapSlippageBps`), so the caller cannot
    ///      influence the execution price. Still treasury-gated: were this
    ///      permissionless, a UserOp's calldata could invoke it mid-bundle and
    ///      swap away other ops' pending prefunds, breaking their postOp
    ///      refunds. The treasury sends ordinary transactions, which can never
    ///      execute mid-bundle.
    function swapAndDeposit(address token, uint256 amountIn) external {
        if (msg.sender != treasury) revert UnauthorizedCall();
        address router = uniswapV2Router;
        if (router == address(0)) revert InvalidRouter(router);
        TokenConfig memory cfg = tokenConfigs[token];
        if (address(cfg.tokenOracle) == address(0)) revert TokenNotRegistered(token);

        uint256 balance = IERC20(token).balanceOf(address(this));
        if (amountIn == 0 || amountIn > balance) amountIn = balance;

        uint256 nativeUsd = _getOraclePrice(nativeOracle, nativeOracleDecimals);
        uint256 tokenUsd = _getOraclePrice(cfg.tokenOracle, cfg.tokenOracleDecimals);
        uint256 expectedWei = (amountIn * tokenUsd * 1e18) / (nativeUsd * (10 ** cfg.tokenDecimals));
        uint256 amountOutMin = (expectedWei * (10_000 - swapSlippageBps)) / 10_000;

        address[] memory path = new address[](2);
        path[0] = token;
        path[1] = IUniswapV2Router02(router).WETH();

        IERC20(token).forceApprove(router, amountIn);
        uint256[] memory amounts = IUniswapV2Router02(router).swapExactTokensForETH(
            amountIn,
            amountOutMin,
            path,
            address(this),
            block.timestamp
        );

        uint256 deposited = address(this).balance;
        entryPoint().depositTo{value: deposited}(address(this));
        emit FeesRecycled(token, amountIn, amounts[1], deposited);
    }

    /// @dev Accepts the router's native output in {swapAndDeposit}.
    receive() external payable {}

    // ── PaymasterERC20 hooks ─────────────────────────────────────────

    /// @dev Executes the EIP-2612 permit (mode 0x00) before the base
    ///      implementation runs _fetchDetails and prefunds via transferFrom.
    ///      The token is validated as registered and active *before* the permit
    ///      runs, so the validation-phase external call only ever targets a
    ///      governance-approved token rather than an attacker-chosen address.
    function _validatePaymasterUserOp(
        PackedUserOperation calldata userOp,
        bytes32 userOpHash,
        uint256 maxCost
    ) internal override returns (bytes memory context, uint256 validationData) {
        uint256 postOpGasLimit = userOp.paymasterPostOpGasLimit();
        if (postOpGasLimit > MAX_POST_OP_GAS_LIMIT) {
            revert InvalidPostOpGasLimit(postOpGasLimit, MAX_POST_OP_GAS_LIMIT);
        }

        bytes calldata data = userOp.paymasterData();
        if (data.length == 0) revert InvalidPaymasterData(0);
        if (uint8(data[0]) == 0x00) {
            if (data.length < 21) revert InvalidPaymasterData(data.length);
            address tokenAddr = address(bytes20(data[1:21]));
            TokenConfig memory cfg = tokenConfigs[tokenAddr];
            if (address(cfg.tokenOracle) == address(0)) revert TokenNotRegistered(tokenAddr);
            if (!cfg.active) revert TokenNotActive(tokenAddr);
            _executePermit(userOp);
        }

        return super._validatePaymasterUserOp(userOp, userOpHash, maxCost);
    }

    /// @dev Returns the token to charge and its price relative to native gas.
    ///
    ///      PaymasterERC20 computes `erc20Cost = weiCost * tokenPrice / 1e18`,
    ///      so tokenPrice must be token base units per wei, scaled by 1e18:
    ///        tokenPrice = (nativeUsd * 10^tokenDecimals) / tokenUsd
    ///      e.g. BNB at $600, USDC at $1 with 6 decimals: 0.001 BNB (1e15 wei)
    ///      should cost 0.60 USDC (600000 units), giving tokenPrice = 6e8, which
    ///      is exactly (600e8 * 1e6) / 1e8. Markup is applied on top.
    function _fetchDetails(
        PackedUserOperation calldata userOp,
        bytes32 /* userOpHash */
    ) internal view override returns (uint256 validationData, IERC20 token, uint256 tokenPrice) {
        bytes calldata data = userOp.paymasterData();
        if (data.length < 21) revert InvalidPaymasterData(data.length);

        uint8 mode = uint8(data[0]);
        if (mode > 0x01) revert InvalidMode(mode);

        address tokenAddr = address(bytes20(data[1:21]));

        TokenConfig memory cfg = tokenConfigs[tokenAddr];
        if (address(cfg.tokenOracle) == address(0)) revert TokenNotRegistered(tokenAddr);
        if (!cfg.active) revert TokenNotActive(tokenAddr);

        tokenPrice = _tokenPrice(cfg);
        token = IERC20(tokenAddr);
        validationData = 0; // no time-range restriction
    }

    /// @dev Parse and execute the EIP-2612 permit from paymasterData.
    ///      Layout: mode(1) + token(20) + permitAmount(32) + deadline(32) + v(1) + r(32) + s(32) = 150 bytes
    function _executePermit(PackedUserOperation calldata userOp) internal {
        bytes calldata data = userOp.paymasterData();
        if (data.length != 150) revert InvalidPaymasterData(data.length);

        address tokenAddr = address(bytes20(data[1:21]));
        uint256 permitAmount = uint256(bytes32(data[21:53]));
        uint256 deadline = uint256(bytes32(data[53:85]));
        uint8 v = uint8(data[85]);
        bytes32 r = bytes32(data[86:118]);
        bytes32 s = bytes32(data[118:150]);

        address owner = userOp.sender;

        try IERC20Permit(tokenAddr).permit(owner, address(this), permitAmount, deadline, v, r, s) {
            emit PermitExecuted(tokenAddr, owner, permitAmount);
        } catch {
            revert PermitFailed(tokenAddr);
        }
    }

    // ── Pricing ──────────────────────────────────────────────────────

    function _tokenPrice(TokenConfig memory cfg) internal view returns (uint256) {
        uint256 nativeUsd = _getOraclePrice(nativeOracle, nativeOracleDecimals);
        uint256 tokenUsd = _getOraclePrice(cfg.tokenOracle, cfg.tokenOracleDecimals);

        return (nativeUsd * (10 ** cfg.tokenDecimals) * (10_000 + markupBps)) / (tokenUsd * 10_000);
    }

    /// @dev Fetch a Chainlink price normalized to 8 decimals.
    ///      Reverts on stale or non-positive answers.
    function _getOraclePrice(AggregatorV3Interface oracle, uint8 oracleDecimals) internal view returns (uint256) {
        (, int256 answer, , uint256 updatedAt, ) = oracle.latestRoundData();

        if (answer <= 0) revert InvalidOraclePrice(address(oracle), answer);
        if (block.timestamp - updatedAt > maxOracleAge) {
            revert StaleOraclePrice(address(oracle), updatedAt);
        }

        if (oracleDecimals < 8) {
            return uint256(answer) * (10 ** (8 - oracleDecimals));
        } else if (oracleDecimals > 8) {
            return uint256(answer) / (10 ** (oracleDecimals - 8));
        }
        return uint256(answer);
    }

    // ── Views ────────────────────────────────────────────────────────

    /// @notice Current price in token base units per wei of gas (scaled by 1e18),
    ///         markup included. For offchain gas estimation.
    function getTokenPrice(address token) external view returns (uint256) {
        TokenConfig memory cfg = tokenConfigs[token];
        if (address(cfg.tokenOracle) == address(0)) revert TokenNotRegistered(token);

        return _tokenPrice(cfg);
    }

    /// @notice Estimate the token cost for a given gas amount and fee, mirroring
    ///         PaymasterERC20._erc20Cost (including its postOp gas cushion).
    function estimateTokenCost(
        address token,
        uint256 gasAmount,
        uint256 maxFeePerGas
    ) external view returns (uint256) {
        TokenConfig memory cfg = tokenConfigs[token];
        if (address(cfg.tokenOracle) == address(0)) revert TokenNotRegistered(token);

        uint256 weiCost = gasAmount * maxFeePerGas + _postOpCost() * maxFeePerGas;
        return (weiCost * _tokenPrice(cfg)) / _tokenPriceDenominator();
    }

    /// @notice List all registered tokens.
    function getRegisteredTokens() external view returns (address[] memory) {
        return registeredTokens;
    }
}
