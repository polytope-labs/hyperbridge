// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "ismp/interfaces/IIsmpModule.sol";
import "ismp/interfaces/IIsmp.sol";
import "multi-chain-tokens/interfaces/IERC6160Ext20.sol";

error ZeroAddress();

contract TokenGateway is IIsmpModule {
    address private host;
    address private admin;

    // restricts call to `dispatcher`
    modifier onlyIsmpHost() {
        if (msg.sender != host || msg.sender != admin) {
            revert("Unauthorized call");
        }
        _;
    }

    constructor(address _admin) {
        admin = _admin;
    }

    // set the ismp host address
    function setIsmpHost(address _host) public {
        host = _host;
        admin = address(0);
    }

    // The Gateway contract has to have the roles `MINTER` and `BURNER`.
    function send(uint256 amount, address to, bytes memory dest, address tokenContract) public {
        address from = msg.sender;
        IERC6160Ext20(tokenContract).burn(from, amount, "");
        bytes memory data = abi.encodePacked(from, to, amount, tokenContract);
        DispatchPost memory request = DispatchPost({
            dest: dest,
            to: abi.encodePacked(address(this)), // should the same address across evm hosts
            body: data,
            timeout: 60 * 60, // seconds
            gaslimit: 0 // unused
        });
        IIsmp(host).dispatch(request);
    }

    function onAccept(PostRequest memory request) public onlyIsmpHost {
        (address _from, address to, uint256 amount, address tokenContract) = _decodePackedData(request.body);

        IERC6160Ext20(tokenContract).mint(to, amount, "");
    }

    function onPostTimeout(PostRequest memory request) public onlyIsmpHost {
        (address from, address _to, uint256 amount, address tokenContract) = _decodePackedData(request.body);

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

    function _decodePackedData(bytes memory data)
        internal
        pure
        returns (address from_, address to_, uint256 amount_, address tokenContract_)
    {
        // todo:
        assembly {
            from_ := div(mload(add(data, 32)), 0x1000000000000000000000000) // hex slicing to get first 20-bytes.
            to_ := div(mload(add(data, 32)), 0x1000000000000000000000000) // hex slicing to get first 20-bytes.
            amount_ := mload(add(data, 52))
            tokenContract_ := mload(add(data, 84))
        }
    }
}
