// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {ISignatureTransfer} from "../../src/utils/SimplexPaymaster.sol";

/// @dev Permit2 AllowanceTransfer surface — the signature-less pull path.
interface IAllowanceTransfer {
    function allowance(address owner, address token, address spender)
        external
        view
        returns (uint160 amount, uint48 expiration, uint48 nonce);

    function transferFrom(address from, address to, uint160 amount, address token) external;

    function DOMAIN_SEPARATOR() external view returns (bytes32);

    function nonceBitmap(address owner, uint256 word) external view returns (uint256);
}

/// @notice Answers: with the filler holding a uint256-max token approval to Permit2, can an
///         attacker who fully controls the paymaster drain the filler's balance? Runs against
///         the real Permit2 and USDT on an Ethereum-mainnet fork.
contract Permit2CompromiseForkTest is Test {
    bytes32 constant TOKEN_PERMISSIONS_TYPEHASH = keccak256("TokenPermissions(address token,uint256 amount)");
    bytes32 constant PERMIT_TRANSFER_FROM_TYPEHASH = keccak256(
        "PermitTransferFrom(TokenPermissions permitted,address spender,uint256 nonce,uint256 deadline)TokenPermissions(address token,uint256 amount)"
    );

    ISignatureTransfer constant PERMIT2 = ISignatureTransfer(0x000000000022D473030F116dDEE9F6B43aC78BA3);
    address constant USDT = 0xdAC17F958D2ee523a2206206994597C13D831ec7;

    address filler;
    uint256 fillerKey;
    address attacker = makeAddr("attacker");
    address paymaster = makeAddr("compromisedPaymaster");

    function setUp() public {
        string memory url = vm.envOr("MAINNET_FORK_URL", string(""));
        if (bytes(url).length == 0) return;
        vm.selectFork(vm.createFork(url));

        (filler, fillerKey) = makeAddrAndKey("compromise-filler");
        vm.etch(filler, "");
        deal(USDT, filler, 1_000_000e6); // 1,000,000 USDT in the filler's account

        // The one lifetime approval the PERMIT2 mode bootstrap performs.
        vm.prank(filler);
        (bool ok,) = USDT.call(abi.encodeWithSelector(IERC20.approve.selector, address(PERMIT2), type(uint256).max));
        require(ok, "approve failed");
    }

    modifier onFork() {
        vm.skip(filler == address(0) || filler.code.length != 0);
        _;
    }

    /// The max token approval targets Permit2, and Permit2 never grants an AllowanceTransfer
    /// allowance without the owner's own action. A signature-less pull drains nothing.
    function testCompromisedPaymasterCannotUseAllowanceTransfer() public onFork {
        (uint160 amount,,) = IAllowanceTransfer(address(PERMIT2)).allowance(filler, USDT, paymaster);
        assertEq(amount, 0, "paymaster has no AllowanceTransfer allowance");

        // The attacker/paymaster tries the signature-less transferFrom for the whole balance.
        vm.prank(paymaster);
        vm.expectRevert();
        IAllowanceTransfer(address(PERMIT2)).transferFrom(filler, attacker, 1_000_000e6, USDT);

        vm.prank(attacker);
        vm.expectRevert();
        IAllowanceTransfer(address(PERMIT2)).transferFrom(filler, attacker, 1_000_000e6, USDT);

        assertEq(IERC20(USDT).balanceOf(filler), 1_000_000e6, "nothing moved");
    }

    /// SignatureTransfer needs a fresh owner signature the attacker cannot forge.
    function testCompromisedPaymasterCannotForgeSignatureTransfer() public onFork {
        // Attacker fabricates a permit for the whole balance, signed by its own key.
        (, uint256 attackerKey) = makeAddrAndKey("attacker-key");
        uint256 deadline = block.timestamp + 1 hours;
        bytes memory forged = _sign(attackerKey, paymaster, USDT, 1_000_000e6, 1, deadline);

        vm.prank(paymaster);
        vm.expectRevert(); // InvalidSigner: recovered signer is not `filler`
        PERMIT2.permitTransferFrom(
            ISignatureTransfer.PermitTransferFrom({
                permitted: ISignatureTransfer.TokenPermissions({token: USDT, amount: 1_000_000e6}),
                nonce: 1,
                deadline: deadline
            }),
            ISignatureTransfer.SignatureTransferDetails({to: attacker, requestedAmount: 1_000_000e6}),
            filler,
            forged
        );

        assertEq(IERC20(USDT).balanceOf(filler), 1_000_000e6, "nothing moved");
    }

    /// The exposure is exactly what the filler actually signed: an outstanding permit can be
    /// redirected to the attacker, but only up to its signed amount, and only once.
    function testCompromisedPaymasterCanOnlyRedirectAnOutstandingPermitUpToItsAmount() public onFork {
        // A real, outstanding $5-equivalent permit the filler signed for the paymaster (a losing bid).
        uint256 signed = 5e6;
        uint256 deadline = block.timestamp + 1 hours;
        bytes memory sig = _sign(fillerKey, paymaster, USDT, signed, 7, deadline);

        // The compromised paymaster redirects it to the attacker and tries to inflate the amount.
        vm.prank(paymaster);
        vm.expectRevert(); // InvalidAmount: requestedAmount > permitted.amount
        PERMIT2.permitTransferFrom(
            ISignatureTransfer.PermitTransferFrom({
                permitted: ISignatureTransfer.TokenPermissions({token: USDT, amount: signed}),
                nonce: 7,
                deadline: deadline
            }),
            ISignatureTransfer.SignatureTransferDetails({to: attacker, requestedAmount: 1_000_000e6}),
            filler,
            sig
        );

        // It can still take the signed amount — this is the whole exposure.
        vm.prank(paymaster);
        PERMIT2.permitTransferFrom(
            ISignatureTransfer.PermitTransferFrom({
                permitted: ISignatureTransfer.TokenPermissions({token: USDT, amount: signed}),
                nonce: 7,
                deadline: deadline
            }),
            ISignatureTransfer.SignatureTransferDetails({to: attacker, requestedAmount: signed}),
            filler,
            sig
        );
        assertEq(IERC20(USDT).balanceOf(attacker), signed, "took the signed amount");

        // The nonce is now spent: the same signature cannot be replayed.
        vm.prank(paymaster);
        vm.expectRevert();
        PERMIT2.permitTransferFrom(
            ISignatureTransfer.PermitTransferFrom({
                permitted: ISignatureTransfer.TokenPermissions({token: USDT, amount: signed}),
                nonce: 7,
                deadline: deadline
            }),
            ISignatureTransfer.SignatureTransferDetails({to: attacker, requestedAmount: signed}),
            filler,
            sig
        );
        assertEq(IERC20(USDT).balanceOf(filler), 1_000_000e6 - signed, "only the signed amount ever left");
    }

    function _sign(uint256 key, address spender, address token, uint256 amount, uint256 nonce, uint256 deadline)
        internal
        view
        returns (bytes memory)
    {
        bytes32 structHash = keccak256(
            abi.encode(
                PERMIT_TRANSFER_FROM_TYPEHASH,
                keccak256(abi.encode(TOKEN_PERMISSIONS_TYPEHASH, token, amount)),
                spender,
                nonce,
                deadline
            )
        );
        bytes32 digest = keccak256(
            abi.encodePacked("\x19\x01", IAllowanceTransfer(address(PERMIT2)).DOMAIN_SEPARATOR(), structHash)
        );
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(key, digest);
        return abi.encodePacked(r, s, v);
    }
}
