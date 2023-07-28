// SPDX-License-Identifier: UNLICENSED
// A Sample ISMP solidity contract for unit tests

pragma solidity ^0.8.2;

import "ismp-solidity/SubstrateHost.sol";
import "ismp-solidity/interfaces/IIsmpDispatcher.sol";
import "solidity-merkle-trees/MerklePatricia.sol";

address constant HOST = 0x843b131BD76419934dae248F6e5a195c0A3C324D;

error NotIsmpHost();
error ExecutionFailed();

struct Payload {
    address to;
    address from;
    uint64 amount;
}

contract IsmpDemo is IIsmpModule {
    using SubstrateHost for *;
    uint64 totalSupply;

    // Mapping of user address to balance
    mapping(address => uint64) public balances;
    event ResponseReceived();
    event TimeoutReceived();
    event BalanceMinted();
    event BalanceBurnt();
    event GetDispatched();

    // restricts call to `IsmpHost`
    modifier onlyIsmpHost() {
        if (msg.sender != HOST) {
            revert NotIsmpHost();
        }
        _;
    }

    constructor() {
        totalSupply = 1000000000;
    }

    function onAccept(PostRequest memory request) public onlyIsmpHost {
        Payload memory payload = decodePayload(request.body);
        PostResponse memory response = PostResponse({
            request: request,
            response: abi.encodePacked(msg.sender)
        });
        _mint(payload.to, payload.amount);
        SubstrateHost.dispatch(response);
        emit BalanceMinted();
      
    }

    function onPostResponse(PostResponse memory response) public onlyIsmpHost {
        // In this callback just try to decode the payload of the corresponding request
        Payload memory payload = decodePayload(response.request.body);
        emit ResponseReceived();
    }

    function onGetResponse(GetResponse memory response) public onlyIsmpHost {
        // For the purpose of this test
        // we just validate the responses in this callback
        for (uint256 index = 0; index < response.values.length; index++) {
            StorageValue memory storageValue = response.values[index];
            if (storageValue.value.length == 0) {
                revert ExecutionFailed();
            }
        }
        emit ResponseReceived();
    }

    function onGetTimeout(GetRequest memory request) public onlyIsmpHost {
        // We validate the keys in this callback
        for (uint256 index = 0; index < request.keys.length; index++) {
            bytes memory key = request.keys[index];
            // No keys should be empty
            if (key.length == 0) {
                revert ExecutionFailed();
            }
        }
        emit TimeoutReceived();
    }

    function onPostTimeout(PostRequest memory request) public onlyIsmpHost {
        Payload memory payload = decodePayload(request.body);
        _mint(payload.from, payload.amount);
        emit BalanceMinted();
    }

    function decodePayload(
        bytes memory data
    ) internal pure returns (Payload memory payload) {
        (payload) = abi.decode(data, (Payload));
        return payload;
    }

    function transfer(
        address to,
        bytes memory dest,
        uint64 amount,
        uint64 timeout,
        uint64 gasLimit
    ) public {
        _burn(msg.sender, amount);
        Payload memory payload = Payload({
            from: msg.sender,
            to: to,
            amount: amount
        });
        DispatchPost memory dispatchPost = DispatchPost({
            body: abi.encode(payload.from, payload.to, payload.amount),
            dest: dest,
            timeoutTimestamp: timeout,
            to: abi.encodePacked(address(12)),
            gaslimit: gasLimit
        });
        SubstrateHost.dispatch(dispatchPost);
        emit BalanceBurnt();
    }

    function dispatchGet(
        bytes memory dest,
        bytes[] memory keys,
        uint64 height,
        uint64 timeout,
        uint64 gasLimit
    ) public {
        DispatchGet memory get = DispatchGet({
            keys: keys,
            dest: dest,
            height: height,
            timeoutTimestamp: timeout,
            gaslimit: gasLimit
        });
        SubstrateHost.dispatch(get);
        emit GetDispatched();
    
    }

    function mintTo(address who, uint64 amount) public onlyIsmpHost {
        _mint(who, amount);
    }

    function _mint(address who, uint64 amount) internal {
        totalSupply = totalSupply + amount;
        balances[who] = balances[who] + amount;
    }

    function _burn(address who, uint64 amount) internal {
        totalSupply = totalSupply - amount;
        balances[who] = balances[who] - amount;
    }
}
