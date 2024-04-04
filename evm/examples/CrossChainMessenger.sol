// SPDX-License-Identifier: UNLICENSED
// A Sample ISMP solidity contract for unit tests

pragma solidity 0.8.17;

import "ismp/IIsmpModule.sol";
import "ismp/IIsmpHost.sol";
import "ismp/Message.sol";
import "ismp/IDispatcher.sol";

struct CrossChainMessage {
    bytes dest;
    bytes message;
    uint64 timeout;
}

contract CrossChainMessenger is IIsmpModule {
    event PostReceived(uint256 nonce, bytes source, string message);

    error NotAuthorized();

    // restricts call to `IIsmpHost`
    modifier onlyIsmpHost() {
        if (msg.sender != host) {
            revert NotAuthorized();
        }
        _;
    }

    // restricts call to `admin`
    modifier onlyAdmin() {
        if (msg.sender != admin) {
            revert NotAuthorized();
        }
        _;
    }

    address private host;
    address private admin;

    constructor(address _admin) {
        admin = _admin;
    }

    // set the ismp host address
    function setIsmpHost(address _host) public {
        host = _host;
        admin = address(0);
    }

    function teleport(CrossChainMessage memory params) public {
        DispatchPost memory post = DispatchPost({
            body: params.message,
            dest: params.dest,
            timeout: params.timeout,
            // instance of this pallet on another chain.
            to: abi.encodePacked(address(this)),
            // unused for now
            fee: 0,
            payer: address(this)
        });
        IDispatcher(host).dispatch(post);
    }

    function onAccept(PostRequest memory request) external onlyIsmpHost {
        emit PostReceived(request.nonce, request.source, string(request.body));
    }

    function onPostRequestTimeout(PostRequest memory) external view onlyIsmpHost {
        revert("No timeouts for now");
    }

    function onPostResponse(PostResponse memory) external view onlyIsmpHost {
        revert("CrossChainMessenger doesn't emit responses");
    }

    function onPostResponseTimeout(PostResponse memory) external view onlyIsmpHost {
        revert("Token gateway doesn't emit Get Requests");
    }

    function onGetResponse(GetResponse memory) external view onlyIsmpHost {
        revert("CrossChainMessenger doesn't emit Get Requests");
    }

    function onGetTimeout(GetRequest memory) external view onlyIsmpHost {
        revert("CrossChainMessenger doesn't emit Get Requests");
    }
}
