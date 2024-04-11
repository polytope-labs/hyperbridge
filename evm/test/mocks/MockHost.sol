// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.13;

import {TokenGateway, IDispatcher} from "../../src/modules/TokenGateway.sol";
import {HostParams} from "../../src/hosts/EvmHost.sol";
import {ERC20Token} from "../mocks/ERC20Token.sol";
import "openzeppelin/utils/math/Math.sol";
import "ismp/Message.sol";
import "ismp/IDispatcher.sol";
import {MockAutoRelayer} from "./MockAutoRelayer.sol";

contract MockHost {
    uint256 private _nonce;
    HostParams private _hostParams;
    bytes _host;
    address relayer;

    constructor(bytes memory host_, address _feeToken, address _relayer, uint256 _perByteFee) {
        _host = host_;
        _hostParams.feeToken = _feeToken;
        relayer = _relayer;
        _hostParams.perByteFee = _perByteFee;
    }

    function host() public view returns (bytes memory) {
        return _host;
    }

    function feeToken() public view returns (address) {
        return _hostParams.feeToken;
    }

    function hostParams() public view returns (HostParams memory) {
        return _hostParams;
    }

    /**
     * @dev Dispatch a POST request to the hyperbridge
     * @param post - post request
     */
    function dispatch(DispatchPost memory post) external {
        uint256 fee = (_hostParams.perByteFee * post.body.length) + post.fee;
        require(ERC20Token(_hostParams.feeToken).transferFrom(tx.origin, address(this), fee), "Insufficient funds");

        // adjust the timeout
        uint64 timeout =
            post.timeout == 0 ? 0 : uint64(block.timestamp) + uint64(Math.max(_hostParams.defaultTimeout, post.timeout));
        PostRequest memory request = PostRequest({
            source: host(),
            dest: post.dest,
            nonce: uint64(_nextNonce()),
            from: abi.encodePacked(tx.origin),
            to: post.to,
            timeoutTimestamp: timeout,
            body: post.body
        });
        MockAutoRelayer(relayer).autoRelay(request);
    }

    /**
     * @dev Dispatch an incoming post request to destination module
     * @param request - post request
     */
    function dispatchIncoming(PostRequest memory request) external {
        address destination = _bytesToAddress(request.to);
        require(destination.code.length > 0, "no code");

        (bool success, bytes memory d) =
            address(destination).call(abi.encodeWithSelector(TokenGateway.onAccept.selector, request));
        // require(success);
        assembly {
            if iszero(success) {
                let free := mload(0x40)
                mstore(free, 0x20)
                pop(staticcall(gas(), 4, d, mload(d), add(free, 0x20), mload(d)))
                revert(free, add(mload(d), 0x40))
            }
        }
    }

    function _nextNonce() private returns (uint256) {
        uint256 _nonce_copy = _nonce;

        unchecked {
            ++_nonce;
        }

        return _nonce_copy;
    }
}

// global function
function _bytesToAddress(bytes memory _bytes) pure returns (address addr) {
    require(_bytes.length >= 20, "Invalid address length");
    assembly {
        addr := mload(add(_bytes, 20))
    }
}
