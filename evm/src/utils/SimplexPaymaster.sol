// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ERC4337Utils, PackedUserOperation} from "@openzeppelin/contracts/account/utils/draft-ERC4337Utils.sol";
import {PaymasterERC20} from "@openzeppelin/community-contracts/contracts/account/paymaster/PaymasterERC20.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {IERC20Metadata} from "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";
import {IERC20Permit} from "@openzeppelin/contracts/token/ERC20/extensions/IERC20Permit.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

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
///         for gas payment.
///
/// Modes (byte 0 of paymasterData):
///   0x00  PERMIT  — EIP-2612 permit signature included; token pre-approved via
///                    the permit in the same UserOp (gas-efficient on chains
///                    with native USDC/USDT that support permit).
///   0x01  APPROVE — Token must be pre-approved to this paymaster (BSC path,
///                    or any chain where permit is unavailable). Batch the
///                    approve() call in the UserOp's calls array.
///
/// paymasterData encoding:
///   Mode 0x00 (permit):
///     abi.encodePacked(uint8(0), address(token), uint256(permitAmount),
///                      uint256(deadline), uint8(v), bytes32(r), bytes32(s))
///   Mode 0x01 (approve):
///     abi.encodePacked(uint8(1), address(token))
///
/// Price conversion uses two Chainlink feeds per token:
///   - token/USD (e.g. USDC/USD) → 8 decimals
///   - nativeAsset/USD (e.g. BNB/USD, ETH/USD) → 8 decimals
///   tokenPrice = (nativePrice * 10^tokenDecimals) / tokenPrice_USD
///   This gives the cost in token-units per 1 wei of native gas.
///
/// Treasury: markup surplus accumulates in the contract and is withdrawable
///           to a separate treasury address. Unused gas is always refunded
///           to the solver (handled by OZ's PaymasterERC20._postOp).
contract SimplexPaymaster is PaymasterERC20, Ownable {
    using SafeERC20 for IERC20;
    using ERC4337Utils for PackedUserOperation;

    // ── Structs ──────────────────────────────────────────────────────

    struct TokenConfig {
        AggregatorV3Interface tokenOracle;   // token/USD feed
        uint8 tokenOracleDecimals;           // cached decimals() of tokenOracle
        uint8 tokenDecimals;                 // decimals() of the ERC-20
        bool active;                         // kill-switch per token
    }

    // ── Constants ────────────────────────────────────────────────────

    /// @dev 10% markup in basis points. Owner-configurable.
    uint256 public constant MAX_MARKUP_BPS = 5_000; // 50% hard cap

    /// @dev Maximum oracle staleness. Chainlink heartbeats vary per chain:
    ///      BSC stablecoins ~27s, Base/Arbitrum USDC ~24h for stablecoins.
    ///      Default 86400 (24 hours). Owner-configurable to match chain-specific heartbeats.
    uint256 public maxOracleAge = 86400;

    // ── State ────────────────────────────────────────────────────────

    /// @notice Native asset / USD oracle (BNB/USD on BSC, ETH/USD on Ethereum, etc.)
    AggregatorV3Interface public immutable nativeOracle;
    uint8 public immutable nativeOracleDecimals;

    /// @notice Markup in basis points (100 = 1%). Applied on top of oracle price.
    uint256 public markupBps;

    /// @notice Treasury address — receives accumulated markup surplus only.
    ///         Unused gas refunds are returned to the solver by OZ's base
    ///         PaymasterERC20._postOp automatically.
    address public treasury;

    /// @notice Per-token configuration. token address → config.
    mapping(address => TokenConfig) public tokenConfigs;

    /// @notice Set of registered token addresses (for enumeration).
    address[] public registeredTokens;

    // ── Events ───────────────────────────────────────────────────────

    event TokenRegistered(address indexed token, address indexed oracle);
    event TokenDeactivated(address indexed token);
    event MarkupUpdated(uint256 oldBps, uint256 newBps);
    event TreasuryUpdated(address oldTreasury, address newTreasury);
    event PermitExecuted(address indexed token, address indexed owner, uint256 amount);

    // ── Errors ───────────────────────────────────────────────────────

    error TokenNotRegistered(address token);
    error TokenNotActive(address token);
    error StaleOraclePrice(address oracle, uint256 updatedAt);
    error InvalidOraclePrice(address oracle, int256 price);
    error InvalidMarkup(uint256 bps);
    error InvalidMode(uint8 mode);
    error PermitFailed(address token);
    error ZeroAddress();

    // ── Constructor ──────────────────────────────────────────────────

    /// @param _nativeOracle  Chainlink native/USD feed (e.g. BNB/USD on BSC)
    /// @param _markupBps     Initial markup in basis points (e.g. 1000 = 10%)
    /// @param _treasury      Address that receives markup surplus
    /// @param _owner         Owner for admin functions
    constructor(
        AggregatorV3Interface _nativeOracle,
        uint256 _markupBps,
        address _treasury,
        address _owner
    ) Ownable(_owner) {
        if (address(_nativeOracle) == address(0)) revert ZeroAddress();
        if (_treasury == address(0)) revert ZeroAddress();
        if (_markupBps > MAX_MARKUP_BPS) revert InvalidMarkup(_markupBps);

        nativeOracle = _nativeOracle;
        nativeOracleDecimals = _nativeOracle.decimals();
        markupBps = _markupBps;
        treasury = _treasury;
    }

    // ── Admin: token management ──────────────────────────────────────

    /// @notice Register or update a supported ERC-20 token.
    /// @param token      The ERC-20 token address
    /// @param oracle     Chainlink token/USD price feed
    function registerToken(address token, AggregatorV3Interface oracle) external onlyOwner {
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

    /// @notice Deactivate a token (stops new UserOps from using it).
    function deactivateToken(address token) external onlyOwner {
        tokenConfigs[token].active = false;
        emit TokenDeactivated(token);
    }

    // ── Admin: parameters ────────────────────────────────────────────

    function setMarkup(uint256 _markupBps) external onlyOwner {
        if (_markupBps > MAX_MARKUP_BPS) revert InvalidMarkup(_markupBps);
        uint256 old = markupBps;
        markupBps = _markupBps;
        emit MarkupUpdated(old, _markupBps);
    }

    function setTreasury(address _treasury) external onlyOwner {
        if (_treasury == address(0)) revert ZeroAddress();
        address old = treasury;
        treasury = _treasury;
        emit TreasuryUpdated(old, _treasury);
    }

    function setMaxOracleAge(uint256 _maxOracleAge) external onlyOwner {
        maxOracleAge = _maxOracleAge;
    }

    // ── Admin: withdrawals ───────────────────────────────────────────

    /// @notice Withdraw accumulated ERC-20 tokens to the treasury.
    function withdrawTokenToTreasury(IERC20 token, uint256 amount) external onlyOwner {
        token.safeTransfer(treasury, amount);
    }

    /// @notice Withdraw native gas token from EntryPoint deposit.
    function withdrawEntryPointDeposit(uint256 amount) external onlyOwner {
        withdraw(payable(treasury), amount);
    }

    // ── Core: OZ PaymasterERC20 hook ─────────────────────────────────

    /// @dev Called by PaymasterERC20._validatePaymasterUserOp.
    ///      Returns the token to charge and its price relative to native gas.
    ///
    ///      tokenPrice semantics (from OZ docs):
    ///        tokenPrice = cost in token-wei per 1 wei of native gas cost.
    ///        Internally OZ computes: erc20Cost = (gasCost * tokenPrice) / TOKEN_PRICE_DENOMINATOR
    ///        where TOKEN_PRICE_DENOMINATOR = 1e18.
    ///
    ///      So tokenPrice = (nativeUsdPrice * 1e18 * 10^tokenDecimals)
    ///                      / (tokenUsdPrice * 10^18)  [native is 18 decimals]
    ///      Simplified:     = (nativeUsdPrice * 10^tokenDecimals) / tokenUsdPrice
    ///      With markup:    *= (10000 + markupBps) / 10000
    function _fetchDetails(
        PackedUserOperation calldata userOp,
        bytes32 /* userOpHash */
    )
        internal
        view
        override
        returns (uint256 validationData, IERC20 token, uint256 tokenPrice)
    {
        // Decode mode + token from paymasterData
        bytes calldata data = userOp.paymasterData();
        uint8 mode = uint8(data[0]);
        address tokenAddr = address(bytes20(data[1:21]));

        // Validate token
        TokenConfig memory cfg = tokenConfigs[tokenAddr];
        if (address(cfg.tokenOracle) == address(0)) revert TokenNotRegistered(tokenAddr);
        if (!cfg.active) revert TokenNotActive(tokenAddr);

        // If mode == 0x00 (permit), execute permit before validation
        // NOTE: permit is executed in _fetchDetails which is called during
        // validatePaymasterUserOp. The permit sets allowance so that
        // the subsequent transferFrom in prefund() succeeds.
        if (mode == 0x00) {
            // This is a view function override — we cannot execute permit here.
            // The permit must be handled in a separate hook. See _permitIfNeeded below.
            // For _fetchDetails, we just return the pricing.
        } else if (mode != 0x01) {
            revert InvalidMode(mode);
        }

        // Fetch oracle prices
        uint256 nativeUsd = _getOraclePrice(nativeOracle, nativeOracleDecimals);
        uint256 tokenUsd = _getOraclePrice(cfg.tokenOracle, cfg.tokenOracleDecimals);

        // tokenPrice = (nativeUsd * 10^tokenDecimals * (10000 + markupBps)) / (tokenUsd * 10000)
        // This gives token-units per 1 wei of native gas, scaled by 1e18 (TOKEN_PRICE_DENOMINATOR).
        //
        // OZ PaymasterERC20._erc20Cost does:
        //   erc20Cost = (cost * feePerGas * tokenPrice) / TOKEN_PRICE_DENOMINATOR
        // where cost is in gas units and feePerGas is in wei. So cost*feePerGas = wei cost.
        // We need tokenPrice such that: tokenAmount = (weiCost * tokenPrice) / 1e18
        //
        // tokenAmount should be in token base units (e.g. 6-decimal USDC).
        // If BNB = $600, USDC = $1, and we spend 0.001 BNB ($0.60):
        //   tokenAmount = 0.60 USDC = 600000 (6 decimals)
        //   weiCost = 1e15 (0.001 BNB)
        //   tokenPrice = tokenAmount * 1e18 / weiCost = 600000 * 1e18 / 1e15 = 6e8 * 1e18 / 1e15
        //   Let's verify: nativeUsd=600e8, tokenUsd=1e8, tokenDecimals=6
        //   tokenPrice = (600e8 * 1e6 * 1e18) / (1e8 * 1e18) = 600 * 1e6 = 6e8 ✓
        //   (then with markup applied on top)

        tokenPrice = (nativeUsd * (10 ** cfg.tokenDecimals) * (10_000 + markupBps)) / (tokenUsd * 10_000);

        token = IERC20(tokenAddr);
        validationData = 0; // no time-range restriction
    }

    // ── Permit execution ─────────────────────────────────────────────

    /// @dev Override _validatePaymasterUserOp to execute permit before prefund.
    ///      OZ's PaymasterERC20._validatePaymasterUserOp calls _fetchDetails
    ///      then prefund(). We intercept to run the permit between the two.
    function _validatePaymasterUserOp(
        PackedUserOperation calldata userOp,
        bytes32 userOpHash,
        uint256 maxCost
    ) internal override returns (bytes memory context, uint256 validationData) {
        // Execute permit if mode == 0x00
        bytes calldata data = userOp.paymasterData();
        if (data.length > 0 && uint8(data[0]) == 0x00) {
            _executePermit(userOp);
        }

        // Delegate to OZ base implementation (calls _fetchDetails → prefund)
        return super._validatePaymasterUserOp(userOp, userOpHash, maxCost);
    }

    /// @dev Parse and execute EIP-2612 permit from paymasterData.
    ///      Layout: mode(1) + token(20) + permitAmount(32) + deadline(32) + v(1) + r(32) + s(32)
    ///      Total:  150 bytes
    function _executePermit(PackedUserOperation calldata userOp) internal {
        bytes calldata data = userOp.paymasterData();
        // Minimum length: 1 + 20 + 32 + 32 + 1 + 32 + 32 = 150
        require(data.length >= 150, "SimplexPaymaster: permit data too short");

        address tokenAddr = address(bytes20(data[1:21]));
        uint256 permitAmount = uint256(bytes32(data[21:53]));
        uint256 deadline = uint256(bytes32(data[53:85]));
        uint8 v = uint8(data[85]);
        bytes32 r = bytes32(data[86:118]);
        bytes32 s = bytes32(data[118:150]);

        address owner = userOp.sender; // the smart account

        // Execute permit — sets allowance from owner to this paymaster
        try IERC20Permit(tokenAddr).permit(owner, address(this), permitAmount, deadline, v, r, s) {
            emit PermitExecuted(tokenAddr, owner, permitAmount);
        } catch {
            revert PermitFailed(tokenAddr);
        }
    }

    // ── Oracle helpers ───────────────────────────────────────────────

    /// @dev Fetch price from a Chainlink feed, normalized to 8 decimals.
    ///      Reverts on stale or non-positive prices.
    function _getOraclePrice(
        AggregatorV3Interface oracle,
        uint8 oracleDecimals
    ) internal view returns (uint256) {
        (, int256 answer,, uint256 updatedAt,) = oracle.latestRoundData();

        if (answer <= 0) revert InvalidOraclePrice(address(oracle), answer);
        if (block.timestamp - updatedAt > maxOracleAge) {
            revert StaleOraclePrice(address(oracle), updatedAt);
        }

        // Normalize to 8 decimals for consistent math
        if (oracleDecimals < 8) {
            return uint256(answer) * (10 ** (8 - oracleDecimals));
        } else if (oracleDecimals > 8) {
            return uint256(answer) / (10 ** (oracleDecimals - 8));
        }
        return uint256(answer);
    }

    // ── View helpers ─────────────────────────────────────────────────

    /// @notice Get the current token price (for gas estimation offchain).
    function getTokenPrice(address token) external view returns (uint256) {
        TokenConfig memory cfg = tokenConfigs[token];
        if (address(cfg.tokenOracle) == address(0)) revert TokenNotRegistered(token);

        uint256 nativeUsd = _getOraclePrice(nativeOracle, nativeOracleDecimals);
        uint256 tokenUsd = _getOraclePrice(cfg.tokenOracle, cfg.tokenOracleDecimals);

        return (nativeUsd * (10 ** cfg.tokenDecimals) * (10_000 + markupBps)) / (tokenUsd * 10_000);
    }

    /// @notice Estimate token cost for a given gas amount and fee.
    function estimateTokenCost(
        address token,
        uint256 gasAmount,
        uint256 maxFeePerGas
    ) external view returns (uint256) {
        uint256 price = this.getTokenPrice(token);
        uint256 weiCost = gasAmount * maxFeePerGas;
        // Mirror OZ's _erc20Cost: (cost * tokenPrice) / 1e18
        // But our tokenPrice already accounts for decimals, so:
        return (weiCost * price) / 1e18;
    }

    /// @notice List all registered tokens.
    function getRegisteredTokens() external view returns (address[] memory) {
        return registeredTokens;
    }

    // ── PaymasterCore: authorize withdrawal ─────────────────────────

    /// @dev Only the owner can withdraw from the EntryPoint deposit.
    function _authorizeWithdraw() internal view override {
        _checkOwner();
    }
}
