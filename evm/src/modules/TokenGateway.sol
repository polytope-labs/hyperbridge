// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "ismp/IIsmpModule.sol";
import "ismp/IIsmp.sol";
import "multi-chain-tokens/interfaces/IERC6160Ext20.sol";

struct SendParams {
    // amount to be sent
    uint256 amount;
    // recipient address
    address to;
    // recipient state machine
    bytes dest;
    // IERC6160Ext20 token contract, should be the same on both chains
    address tokenContract;
    // timeout in seconds
    uint64 timeout;
}

contract TokenGateway is IIsmpModule {
    address private host;
    address private admin;

    // User has received some assets, source chain & nonce
    event AssetReceived(bytes source, uint256 nonce);

    // restricts call to `IIsmpHost`
    modifier onlyIsmpHost() {
        if (msg.sender != host) {
            revert("Unauthorized call");
        }
        _;
    }

    // restricts call to `admin`
    modifier onlyAdmin() {
        if (msg.sender != admin) {
            revert("Unauthorized call");
        }
        _;
    }

    constructor(address _admin) {
        admin = _admin;
    }

    // set the ismp host address
    function setIsmpHost(address _host) public onlyAdmin {
        host = _host;
        admin = address(0);
    }

    // The Gateway contract has to have the roles `MINTER` and `BURNER`.
    function send(SendParams memory params) public {
        address from = msg.sender;
        IERC6160Ext20(params.tokenContract).burn(from, params.amount, "");
        bytes memory data = abi.encode(from, params.to, params.amount, params.tokenContract);
        DispatchPost memory request = DispatchPost({
            dest: params.dest,
            to: abi.encodePacked(address(this)), // should the same address across evm hosts
            body: data,
            timeout: params.timeout, // seconds
            gaslimit: 0 // unused
        });
        IIsmp(host).dispatch(request);
    }

    function onAccept(PostRequest memory request) public onlyIsmpHost {
        (address _from, address to, uint256 amount, address tokenContract) =
            abi.decode(request.body, (address, address, uint256, address));

        IERC6160Ext20(tokenContract).mint(to, amount, "");

        emit AssetReceived(request.source, request.nonce);
    }

    function onPostTimeout(PostRequest memory request) public onlyIsmpHost {
        (address from, address _to, uint256 amount, address tokenContract) =
            abi.decode(request.body, (address, address, uint256, address));

        IERC6160Ext20(tokenContract).mint(from, amount, "");
    }

    function onPostResponse(PostResponse memory response) public view onlyIsmpHost {
        revert("Token gateway doesn't emit responses");
    }

    function onGetResponse(GetResponse memory response) public view onlyIsmpHost {
        revert("Token gateway doesn't emit Get Requests");
    }

    function onGetTimeout(GetRequest memory request) public view onlyIsmpHost {
        revert("Token gateway doesn't emit Get Requests");
    }
}
