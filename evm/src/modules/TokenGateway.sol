// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "ismp/IIsmpModule.sol";
import "ismp/IIsmp.sol";
import "multi-chain-tokens/interfaces/IERC6160Ext20.sol";
import {IERC20} from "openzeppelin/token/ERC20/IERC20.sol";

struct SendParams {
    // amount to be sent
    uint256 amount;
    // Relayer fee
    uint256 fee;
    // Gas limit for the request
    uint256 gaslimit;
    // recipient address
    address to;
    // recipient state machine
    bytes dest;
    // The token identifier
    bytes32 tokenId;
    // timeout in seconds
    uint64 timeout;
}

struct Body {
    // amount to be sent
    uint256 amount;
    // The token identifier
    bytes32 tokenId;
    // sender address
    address from;
    // recipient address
    address to;
}

contract TokenGateway is IIsmpModule {
    address private host;
    address private admin;

    // mapping of token identifier to erc6160 contracts
    mapping(bytes32 => address) private _erc6160s;
    // mapping of token identifier to erc20 contracts
    mapping(bytes32 => address) private _erc20s;
    // foreign to local asset identifier mapping
    mapping(bytes32 => bytes32) private _assets;

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

        address erc20 = _erc20s[params.tokenId];
        address erc6160 = _erc6160s[params.tokenId];
        require(params.to != address(0), "Burn your funds some other way");

        if (erc20 != address(0)) {
            // custody the user's funds
            require(IERC20(erc20).transferFrom(from, address(this), params.amount), "Gateway: Insufficient Balance");
        } else if (erc6160 != address(0)) {
            require(IERC6160Ext20(erc6160).burn(from, params.amount, ""), "Gateway: Insufficient Balance");
        } else {
            revert("Gateway: Unknown Token Identifier");
        }

        bytes memory data =
            abi.encode(Body({from: from, to: params.to, amount: params.amount, tokenId: params.tokenId}));
        DispatchPost memory request = DispatchPost({
            dest: params.dest,
            to: abi.encodePacked(address(this)), // should be the same address across evm hosts
            body: data,
            timeout: params.timeout,
            gaslimit: params.gaslimit,
            fee: params.fee
        });
        IIsmp(host).dispatch(request);
    }

    function onAccept(PostRequest memory request) public onlyIsmpHost {
        Body memory body = abi.decode(request.body, (Body));

        bytes32 localAsset = _assets[body.tokenId];
        address erc20 = _erc20s[localAsset];
        address erc6160 = _erc6160s[localAsset];

        // prefer to give the user erc20
        if (erc20 != address(0)) {
            // relayers double as liquidity providers, todo: protocol fees
            require(IERC20(erc20).transferFrom(tx.origin, body.to, body.amount), "Gateway: Insufficient Balance");
            // hand the relayer the erc6160, so they can redeem on the source chain
            IERC6160Ext20(erc6160).mint(tx.origin, body.amount, "");
        } else if (erc6160 != address(0)) {
            IERC6160Ext20(erc6160).mint(body.to, body.amount, "");
        } else {
            revert("Gateway: Unknown Token Identifier");
        }

        emit AssetReceived(request.source, request.nonce);
    }

    function onPostRequestTimeout(PostRequest memory request) public onlyIsmpHost {
        Body memory body = abi.decode(request.body, (Body));

        address erc20 = _erc20s[body.tokenId];
        address erc6160 = _erc6160s[body.tokenId];

        if (erc20 != address(0)) {
            require(IERC20(erc20).transfer(body.from, body.amount), "Gateway: Insufficient Balance, Undefined State");
        } else if (erc6160 != address(0)) {
            IERC6160Ext20(tokenContract).mint(from, amount, "");
        } else {
            revert("Gateway: Inconsisten State");
        }
    }

    function onPostResponse(PostResponse memory) external view onlyIsmpHost {
        revert("Token gateway doesn't emit Post responses");
    }

    function onPostResponseTimeout(PostResponse memory request) external view onlyIsmpHost {
        revert("Token gateway doesn't emit Post responses");
    }

    function onGetResponse(GetResponse memory) external view onlyIsmpHost {
        revert("Token gateway doesn't emit Get requests");
    }

    function onGetTimeout(GetRequest memory) external view onlyIsmpHost {
        revert("Token gateway doesn't emit Get Requests");
    }
}
