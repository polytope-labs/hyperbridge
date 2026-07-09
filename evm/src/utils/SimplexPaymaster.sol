// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ERC4337Utils, PackedUserOperation} from "@openzeppelin/contracts/account/utils/draft-ERC4337Utils.sol";
import {PaymasterERC20} from "@openzeppelin/community-contracts/contracts/account/paymaster/PaymasterERC20.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {IERC20Metadata} from "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";
import {IERC20Permit} from "@openzeppelin/contracts/token/ERC20/extensions/IERC20Permit.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {Ownable2Step} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {Initializable} from "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import {UUPSUpgradeable} from "@openzeppelin/contracts/proxy/utils/UUPSUpgradeable.sol";

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
///         for gas payment. Deployed behind an ERC1967Proxy (UUPS).
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
///      accounts. Three independent layers bound the damage:
///      1. Clients keep allowances and permit amounts small (a few dollars),
///         so the contract can never pull more than the residual allowance.
///      2. Upgrades are timelocked: `scheduleUpgrade` announces the new
///         implementation onchain and `upgradeToAndCall` only succeeds after
///         `UPGRADE_DELAY`, giving solvers time to revoke allowances if a
///         malicious upgrade is scheduled. The owner is expected to be a
///         multisig.
contract SimplexPaymaster is Initializable, UUPSUpgradeable, PaymasterERC20, Ownable2Step {
    using SafeERC20 for IERC20;
    using ERC4337Utils for PackedUserOperation;

    struct TokenConfig {
        AggregatorV3Interface tokenOracle; // token/USD feed
        uint8 tokenOracleDecimals; // cached decimals() of tokenOracle
        uint8 tokenDecimals; // decimals() of the ERC-20
        bool active; // kill-switch per token
    }

    /// @dev Hard cap on the owner-configurable markup (50%).
    uint256 public constant MAX_MARKUP_BPS = 5_000;

    /// @dev Delay between scheduling an upgrade and executing it.
    uint256 public constant UPGRADE_DELAY = 48 hours;

    /// @notice Native asset / USD oracle (BNB/USD on BSC, ETH/USD on Ethereum, etc.)
    AggregatorV3Interface public nativeOracle;
    uint8 public nativeOracleDecimals;

    /// @notice Maximum oracle staleness. Chainlink heartbeats vary per chain
    ///         (BSC stablecoins ~27s, Base/Arbitrum stablecoins up to 24h), so this
    ///         defaults to 24 hours and is owner-configurable per deployment.
    uint256 public maxOracleAge;

    /// @notice Markup in basis points (100 = 1%). Applied on top of the oracle price.
    uint256 public markupBps;

    /// @notice Receives accumulated markup surplus and EntryPoint deposit withdrawals.
    address public treasury;

    mapping(address => TokenConfig) public tokenConfigs;

    /// @notice Set of registered token addresses (for enumeration).
    address[] public registeredTokens;

    /// @notice Implementation scheduled for upgrade, executable after {pendingUpgradeAfter}.
    address public pendingImplementation;
    uint256 public pendingUpgradeAfter;

    uint256[50] private __gap;

    event TokenRegistered(address indexed token, address indexed oracle);
    event TokenDeactivated(address indexed token);
    event MarkupUpdated(uint256 oldBps, uint256 newBps);
    event TreasuryUpdated(address oldTreasury, address newTreasury);
    event PermitExecuted(address indexed token, address indexed owner, uint256 amount);
    event UpgradeScheduled(address indexed implementation, uint256 executableAfter);
    event UpgradeCancelled(address indexed implementation);

    error TokenNotRegistered(address token);
    error TokenNotActive(address token);
    error StaleOraclePrice(address oracle, uint256 updatedAt);
    error InvalidOraclePrice(address oracle, int256 price);
    error InvalidMarkup(uint256 bps);
    error InvalidMode(uint8 mode);
    error InvalidPaymasterData(uint256 length);
    error PermitFailed(address token);
    error ZeroAddress();
    error UpgradeNotScheduled(address implementation);
    error UpgradeDelayNotElapsed(uint256 executableAfter);

    constructor() Ownable(msg.sender) {
        _disableInitializers();
    }

    /// @param nativeOracle_  Chainlink native/USD feed (e.g. BNB/USD on BSC)
    /// @param markupBps_     Initial markup in basis points (e.g. 200 = 2%)
    /// @param treasury_      Address that receives markup surplus
    /// @param owner_         Owner for admin functions (expected to be a multisig)
    function initialize(
        AggregatorV3Interface nativeOracle_,
        uint256 markupBps_,
        address treasury_,
        address owner_
    ) external initializer {
        if (address(nativeOracle_) == address(0)) revert ZeroAddress();
        if (treasury_ == address(0)) revert ZeroAddress();
        if (owner_ == address(0)) revert ZeroAddress();
        if (markupBps_ > MAX_MARKUP_BPS) revert InvalidMarkup(markupBps_);

        nativeOracle = nativeOracle_;
        nativeOracleDecimals = nativeOracle_.decimals();
        markupBps = markupBps_;
        treasury = treasury_;
        maxOracleAge = 86400;
        _transferOwnership(owner_);
    }

    // ── Upgrades ─────────────────────────────────────────────────────

    /// @notice Announce an upgrade. Executable via upgradeToAndCall after UPGRADE_DELAY.
    function scheduleUpgrade(address newImplementation) external onlyOwner {
        if (newImplementation == address(0)) revert ZeroAddress();
        pendingImplementation = newImplementation;
        pendingUpgradeAfter = block.timestamp + UPGRADE_DELAY;
        emit UpgradeScheduled(newImplementation, pendingUpgradeAfter);
    }

    /// @notice Cancel a scheduled upgrade.
    function cancelUpgrade() external onlyOwner {
        address implementation = pendingImplementation;
        if (implementation == address(0)) revert UpgradeNotScheduled(address(0));
        delete pendingImplementation;
        delete pendingUpgradeAfter;
        emit UpgradeCancelled(implementation);
    }

    /// @dev Only the scheduled implementation, only after the delay has elapsed.
    function _authorizeUpgrade(address newImplementation) internal override onlyOwner {
        if (newImplementation == address(0) || newImplementation != pendingImplementation) {
            revert UpgradeNotScheduled(newImplementation);
        }
        if (block.timestamp < pendingUpgradeAfter) revert UpgradeDelayNotElapsed(pendingUpgradeAfter);
        delete pendingImplementation;
        delete pendingUpgradeAfter;
    }

    // ── Admin ────────────────────────────────────────────────────────

    /// @notice Register or update a supported ERC-20 token with its token/USD feed.
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

    /// @notice Withdraw accumulated ERC-20 tokens (markup surplus) to the treasury.
    function withdrawTokenToTreasury(IERC20 token, uint256 amount) external onlyOwner {
        token.safeTransfer(treasury, amount);
    }

    /// @notice Withdraw native gas token from the EntryPoint deposit to the treasury.
    function withdrawEntryPointDeposit(uint256 amount) external onlyOwner {
        withdraw(payable(treasury), amount);
    }

    // ── PaymasterERC20 hooks ─────────────────────────────────────────

    /// @dev Executes the EIP-2612 permit (mode 0x00) before the base
    ///      implementation runs _fetchDetails and prefunds via transferFrom.
    function _validatePaymasterUserOp(
        PackedUserOperation calldata userOp,
        bytes32 userOpHash,
        uint256 maxCost
    ) internal override returns (bytes memory context, uint256 validationData) {
        bytes calldata data = userOp.paymasterData();
        if (data.length == 0) revert InvalidPaymasterData(0);
        if (uint8(data[0]) == 0x00) {
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

    /// @dev Only the owner can withdraw the EntryPoint deposit or stake.
    function _authorizeWithdraw() internal view override {
        _checkOwner();
    }
}
