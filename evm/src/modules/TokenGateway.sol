// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
pragma solidity ^0.8.17;

import {DispatchPost} from "@polytope-labs/ismp-solidity/IDispatcher.sol";
import {IIsmpHost} from "@polytope-labs/ismp-solidity/IIsmpHost.sol";
import {Message} from "@polytope-labs/ismp-solidity/Message.sol";
import {StateMachine} from "@polytope-labs/ismp-solidity/StateMachine.sol";
import {BaseIsmpModule, PostRequest, IncomingPostRequest} from "@polytope-labs/ismp-solidity/IIsmpModule.sol";
import {IERC6160Ext20} from "@polytope-labs/erc6160/interfaces/IERC6160Ext20.sol";
import {ERC6160Ext20} from "@polytope-labs/erc6160/tokens/ERC6160Ext20.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {Bytes} from "@polytope-labs/solidity-merkle-trees/src/trie/Bytes.sol";
import {IUniswapV2Router02} from "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";
import {ICallDispatcher} from "../interfaces/ICallDispatcher.sol";

struct TeleportParams {
    // amount to be sent
    uint256 amount;
    // Relayer fee
    uint256 relayerFee;
    // The token identifier to send
    bytes32 assetId;
    // Redeem ERC20 on the destination?
    bool redeem;
    // recipient address
    bytes32 to;
    // recipient state machine
    bytes dest;
    // request timeout in seconds
    uint64 timeout;
    // Amount of native token to pay for dispatching the request
    // if 0 will use the `IIsmpHost.feeToken`
    uint256 nativeCost;
    // destination contract call data
    bytes data;
}

struct Body {
    // Amount of the asset to be sent
    uint256 amount;
    // The asset identifier
    bytes32 assetId;
    // Flag to redeem the erc20 asset on the destination
    bool redeem;
    // Sender address
    bytes32 from;
    // Recipient address
    bytes32 to;
}

struct BodyWithCall {
    // Amount of the asset to be sent
    uint256 amount;
    // The asset identifier
    bytes32 assetId;
    // Flag to redeem the erc20 asset on the destination
    bool redeem;
    // Sender address
    bytes32 from;
    // Recipient address
    bytes32 to;
    // Calldata to be passed to the asset destination
    bytes data;
}

struct TokenGatewayParamsExt {
    // Initial params for TokenGateway
    TokenGatewayParams params;
    // List of initial assets
    AssetMetadata[] assets;
}

struct Asset {
    // ERC20 token contract address for the asset
    address erc20;
    // ERC6160 token contract address for the asset
    address erc6160;
    // Asset's identifier
    bytes32 identifier;
}

struct ContractInstance {
    // The state machine identifier for this chain
    bytes chain;
    // The token gateway contract address on this chain
    address moduleId;
}

enum OnAcceptActions {
    // Incoming asset from a chain
    IncomingAsset,
    // Governance action to update protocol parameters
    GovernanceAction,
    // Request from hyperbridge to create a new asset
    CreateAsset,
    // Remove an asset from the registry
    DeregisterAsset,
    // Change the admin of an asset
    ChangeAssetAdmin,
    // Add a new pre-approved address
    NewContractInstance
}

struct AssetMetadata {
    // ERC20 token contract address for the asset
    address erc20;
    // ERC6160 token contract address for the asset
    address erc6160;
    // Asset's name
    string name;
    // Asset's symbol
    string symbol;
    // The initial supply of asset
    uint256 initialSupply;
    // Initial beneficiary of the total supply
    address beneficiary;
}

struct ChangeAssetAdmin {
    // Address of the asset
    bytes32 assetId;
    // The address of the new admin
    address newAdmin;
}

struct DeregsiterAsset {
    // List of assets to deregister
    bytes32[] assetIds;
}

// Abi-encoded size of Body struct
uint256 constant BODY_BYTES_SIZE = 161;

// Params for the TokenGateway contract
struct TokenGatewayParams {
    // address of the IsmpHost contract on this chain
    address host;
    // dispatcher for delegating external calls
    address dispatcher;
}

/**
 * @title The TokenGateway.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice Allows users send either ERC20 or ERC6160 tokens using Hyperbridge as a message-passing layer.
 *
 * @dev ERC20 tokens are custodied in exchange for ERC6160 tokens to be minted on the destination chain,
 * Otherwise if ERC6160 tokens are sent, then it simply performs a burn-and-mint.
 */
contract TokenGateway is BaseIsmpModule {
    using Bytes for bytes;
    using Message for PostRequest;

    // TokenGateway protocol parameters
    TokenGatewayParams private _params;

    // admin account
    address private _admin;

    // mapping of token identifier to erc6160 contracts
    mapping(bytes32 => address) private _erc6160s;

    // mapping of token identifier to erc20 contracts
    mapping(bytes32 => address) private _erc20s;

    // mapping of keccak256(source chain) to the token gateway contract address
    mapping(bytes32 => address) private _instances;

    // User has received some assets
    event AssetReceived(
        // The amount that was provided to the user
        uint256 amount,
        // The associated request commitment
        bytes32 commitment,
        // The source of the funds
        bytes32 indexed from,
        // The beneficiary of the funds
        address indexed beneficiary,
        // The provided assetId
        bytes32 indexed assetId
    );

    // User has sent some assets
    event AssetTeleported(
        // The beneficiary of the funds
        bytes32 to,
        // The destination chain
        string dest,
        // The amount that was requested to be sent
        uint256 amount,
        // The associated request commitment
        bytes32 commitment,
        // The source of the funds
        address indexed from,
        // The provided assetId
        bytes32 indexed assetId,
        // Flag to redeem funds from the TokenGateway
        bool redeem
    );

    // User assets could not be delivered and have been refunded.
    event AssetRefunded(
        // The amount that was requested to be sent
        uint256 amount,
        // The associated request commitment
        bytes32 commitment,
        // The beneficiary of the funds
        address indexed beneficiary,
        // The provided assetId
        bytes32 indexed assetId
    );

    // A new asset has been registered
    event AssetRegistered(
        // ERC20 token contract address for the asset
        address erc20,
        // ERC6160 token contract address for the asset
        address erc6160,
        // Asset's name
        string name,
        // Asset's symbol
        string symbol,
        // Registered asset identifier
        bytes32 assetId,
        // The initial supply of asset
        uint256 initialSupply,
        // Initial beneficiary of the total supply
        address beneficiary
    );

    // A new contract instance has been registered
    event NewContractInstance(
        // The chain for this new contract instance
        string chain,
        // The address for token gateway on this chain
        address moduleId
    );

    // Contract parameters have been updated by Hyperbridge governance
    event ParamsUpdated(
        // The old token gateway params
        TokenGatewayParams oldParams,
        // The new token gateway params
        TokenGatewayParams newParams
    );

    // An asset has been deregistered
    event AssetRemoved(bytes32 assetId);

    // An asset owner has requested to change the admin of their asset
    event AssetAdminChanged(
        // The ERC6160 token contract address
        address asset,
        // The new admin
        address newAdmin
    );

    // @dev Action is unauthorized
    error UnauthorizedAction();

    // @dev Unexpected zero address
    error ZeroAddress();

    // @dev Provided amount was invalid
    error InvalidAmount();

    // @dev Provided token was unknown
    error UnknownAsset();

    // @dev Protocol invariant violated
    error InconsistentState();

    // @dev Provided address didn't fit address type size
    error InvalidAddressLength();

    // @dev restricts call to the provided `caller`
    modifier restrict(address caller) {
        if (msg.sender != caller) revert UnauthorizedAction();
        _;
    }

    /**
     * @dev Checks that the request originates from a known instance of the TokenGateway.
     */
    modifier authenticate(PostRequest calldata request) {
        // TokenGateway only accepts incoming assets from itself
        bool unknown = !request.from.equals(abi.encodePacked(address(this))) &&
            _instances[keccak256(request.source)] != bytesToAddress(request.from);
        if (unknown) revert UnauthorizedAction();
        _;
    }

    constructor(address admin) {
        _admin = admin;
    }

    /**
     * @dev initialize required parameters
     */
    function init(TokenGatewayParamsExt calldata teleportParams) public restrict(_admin) {
        _params = teleportParams.params;
        createAssets(teleportParams.assets);

        // infinite approval to save on gas
        IERC20(IIsmpHost(_params.host).feeToken()).approve(
            teleportParams.params.host,
            type(uint256).max
        );

        // admin can only call this once
        _admin = address(0);
    }

    /**
     * @dev Read the protocol parameters
     */
    function params() external view returns (TokenGatewayParams memory) {
        return _params;
    }

    /**
     * @dev Fetch the address for an ERC20 asset
     */
    function erc20(bytes32 assetId) public view returns (address) {
        return _erc20s[assetId];
    }

    /**
     * @dev Fetch the address for an ERC6160 asset
     */
    function erc6160(bytes32 assetId) public view returns (address) {
        return _erc6160s[assetId];
    }

    /**
     * @dev Fetch the TokenGateway instance for a destination.
     */
    function instance(bytes calldata destination) public view returns (address) {
        address gateway = _instances[keccak256(destination)];
        return gateway == address(0) ? address(this) : gateway;
    }

    /**
     * @dev Teleports a local ERC20/ERC6160 asset to the destination chain. Allows users to pay
     * the Hyperbridge fees in the native token or `IIsmpHost.feeToken`
     *
     * @notice If a request times out, users can request a refund permissionlessly through
     * `HandlerV1.handlePostRequestTimeouts`.
     */
    function teleport(TeleportParams calldata teleportParams) public payable {
        if (teleportParams.to == bytes32(0)) revert ZeroAddress();
        if (teleportParams.amount == 0) revert InvalidAmount();

        uint256 msgValue = msg.value;
        address _erc20 = _erc20s[teleportParams.assetId];
        address _erc6160 = _erc6160s[teleportParams.assetId];
        address feeToken = IIsmpHost(_params.host).feeToken();

        // custody or burn funds to be bridged
        if (_erc20 != address(0) && !teleportParams.redeem) {
            address uniswapV2 = IIsmpHost(_params.host).uniswapV2Router();
            address WETH = IUniswapV2Router02(uniswapV2).WETH();
            if (msgValue >= teleportParams.amount && _erc20 == WETH) {
                // wrap native token
                (bool sent, ) = WETH.call{value: teleportParams.amount}("");
                if (!sent) revert InconsistentState();
                msgValue -= teleportParams.amount;
            } else {
                SafeERC20.safeTransferFrom(IERC20(_erc20), msg.sender, address(this), teleportParams.amount);
            }
        } else if (_erc6160 != address(0)) {
            IERC6160Ext20(_erc6160).burn(msg.sender, teleportParams.amount);
        } else {
            revert UnknownAsset();
        }

        // dispatch request
        bytes memory data = teleportParams.data.length > 0
            ? abi.encode(
                BodyWithCall({
                    from: addressToBytes32(msg.sender),
                    to: teleportParams.to,
                    amount: teleportParams.amount,
                    assetId: teleportParams.assetId,
                    redeem: teleportParams.redeem,
                    data: teleportParams.data
                })
            )
            : abi.encode(
                Body({
                    from: addressToBytes32(msg.sender),
                    to: teleportParams.to,
                    amount: teleportParams.amount,
                    assetId: teleportParams.assetId,
                    redeem: teleportParams.redeem
                })
            );
        data = bytes.concat(hex"00", data); // add enum variant for body
        DispatchPost memory request = DispatchPost({
            dest: teleportParams.dest,
            to: abi.encodePacked(instance(teleportParams.dest)),
            body: data,
            timeout: teleportParams.timeout,
            fee: teleportParams.relayerFee,
            payer: msg.sender
        });
        bytes32 commitment = bytes32(0);
        if (msgValue >= teleportParams.nativeCost && teleportParams.nativeCost > 0) {
            // there's some native tokens left to pay for request dispatch
            commitment = IIsmpHost(_params.host).dispatch{value: teleportParams.nativeCost}(request);
        } else {
            // try to pay for dispatch with fee token
            uint256 fee = (IIsmpHost(_params.host).perByteFee(teleportParams.dest) * data.length) + teleportParams.relayerFee;
            SafeERC20.safeTransferFrom(IERC20(feeToken), msg.sender, address(this), fee);
            commitment = IIsmpHost(_params.host).dispatch(request);
        }

        emit AssetTeleported({
            from: msg.sender,
            to: teleportParams.to,
            dest: string(teleportParams.dest),
            assetId: teleportParams.assetId,
            amount: teleportParams.amount,
            redeem: teleportParams.redeem,
            commitment: commitment
        });
    }

    /**
     * @dev Entry point for all cross-chain messages.
     */
    function onAccept(IncomingPostRequest calldata incoming) external override restrict(_params.host) {
        OnAcceptActions action = OnAcceptActions(uint8(incoming.request.body[0]));

        if (action == OnAcceptActions.IncomingAsset) {
            if (incoming.request.body.length > BODY_BYTES_SIZE) {
                handleIncomingAssetWithCall(incoming);
            } else {
                handleIncomingAssetWithoutCall(incoming);
            }
        } else if (action == OnAcceptActions.GovernanceAction) {
            handleGovernance(incoming.request);
        } else if (action == OnAcceptActions.CreateAsset) {
            handleCreateAsset(incoming.request);
        } else if (action == OnAcceptActions.DeregisterAsset) {
            handleDeregisterAssets(incoming.request);
        } else if (action == OnAcceptActions.ChangeAssetAdmin) {
            handleChangeAssetAdmin(incoming.request);
        } else if (action == OnAcceptActions.NewContractInstance) {
            handleNewContractInstance(incoming.request);
        }
    }

    /**
     * @dev Triggered when a previously sent out request is confirmed to be timed-out by the IsmpHost.
     * @notice This means the funds could not be sent, we simply refund the user's assets here.
     */
    function onPostRequestTimeout(PostRequest calldata request) external override restrict(_params.host) {
        Body memory body;
        if (request.body.length > BODY_BYTES_SIZE) {
            BodyWithCall memory bodyWithCall = abi.decode(request.body[1:], (BodyWithCall));
            body = Body({
                amount: bodyWithCall.amount,
                assetId: bodyWithCall.assetId,
                redeem: bodyWithCall.redeem,
                from: bodyWithCall.from,
                to: bodyWithCall.to
            });
        } else {
            body = abi.decode(request.body[1:], (Body));
        }

        address _erc20 = _erc20s[body.assetId];
        address _erc6160 = _erc6160s[body.assetId];
        address from = bytes32ToAddress(body.from);

        if (_erc20 != address(0) && !body.redeem) {
            SafeERC20.safeTransfer(IERC20(_erc20), from, body.amount);
        } else if (_erc6160 != address(0)) {
            IERC6160Ext20(_erc6160).mint(from, body.amount);
        } else {
            revert InconsistentState();
        }

        emit AssetRefunded({commitment: request.hash(), beneficiary: from, amount: body.amount, assetId: body.assetId});
    }

    /**
     * @dev Execute an incoming request with no calldata
     */
    function handleIncomingAssetWithoutCall(
        IncomingPostRequest calldata incoming
    ) internal authenticate(incoming.request) {
        Body memory body = abi.decode(incoming.request.body[1:], (Body));
        bytes32 commitment = incoming.request.hash();
        handleIncomingAsset(body);

        emit AssetReceived({
            commitment: commitment,
            beneficiary: bytes32ToAddress(body.to),
            from: body.from,
            amount: body.amount,
            assetId: body.assetId
        });
    }

    /**
     * @dev Execute an incoming request with calldata, delegates calls to 3rd party contracts to
     * the `_params.dispatcher` for safety reasons.
     */
    function handleIncomingAssetWithCall(
        IncomingPostRequest calldata incoming
    ) internal authenticate(incoming.request) {
        BodyWithCall memory body = abi.decode(incoming.request.body[1:], (BodyWithCall));
        bytes32 commitment = incoming.request.hash();
        handleIncomingAsset(
            Body({amount: body.amount, assetId: body.assetId, redeem: body.redeem, from: body.from, to: body.to})
        );

        ICallDispatcher(_params.dispatcher).dispatch(body.data);

        emit AssetReceived({
            commitment: commitment,
            beneficiary: bytes32ToAddress(body.to),
            from: body.from,
            amount: body.amount,
            assetId: body.assetId
        });
    }

    /**
     * @dev Executes the asset disbursement for the provided request
     */
    function handleIncomingAsset(Body memory body) internal {
        address _erc20 = _erc20s[body.assetId];
        address _erc6160 = _erc6160s[body.assetId];

        if (_erc20 != address(0) && body.redeem) {
            // a relayer/user is redeeming the native asset
            SafeERC20.safeTransfer(IERC20(_erc20), bytes32ToAddress(body.to), body.amount);
        } else if (_erc6160 != address(0)) {
            IERC6160Ext20(_erc6160).mint(bytes32ToAddress(body.to), body.amount);
        } else {
            revert UnknownAsset();
        }
    }

    /**
     * @dev Handles requests from cross-chain governance
     */
    function handleGovernance(PostRequest calldata request) internal {
        if (!request.source.equals(IIsmpHost(_params.host).hyperbridge())) revert UnauthorizedAction();

        TokenGatewayParams memory newParams = abi.decode(request.body[1:], (TokenGatewayParams));

        emit ParamsUpdated({oldParams: _params, newParams: newParams});

        _params = newParams;
    }

    /**
     * @dev registers a new asset as requested by cross-chain governance
     */
    function handleCreateAsset(PostRequest calldata request) internal {
        if (!request.source.equals(IIsmpHost(_params.host).hyperbridge())) revert UnauthorizedAction();

        AssetMetadata[] memory assets = new AssetMetadata[](1);
        assets[0] = abi.decode(request.body[1:], (AssetMetadata));
        createAssets(assets);
    }

    /**
     * @dev Deregisters the asset from TokenGateway. Users will be unable to bridge the asset
     * through TokenGateway once they are deregistered
     */
    function handleDeregisterAssets(PostRequest calldata request) internal {
        if (!request.source.equals(IIsmpHost(_params.host).hyperbridge())) revert UnauthorizedAction();

        DeregsiterAsset memory deregister = abi.decode(request.body[1:], (DeregsiterAsset));
        uint256 length = deregister.assetIds.length;
        for (uint256 i = 0; i < length; ++i) {
            delete _erc20s[deregister.assetIds[i]];
            delete _erc6160s[deregister.assetIds[i]];

            emit AssetRemoved({assetId: deregister.assetIds[i]});
        }
    }

    /**
     * @dev Changes the asset admin from this contract to some other address. Changing the admin to a
     * zero address is disallowed for safety reasons
     */
    function handleChangeAssetAdmin(PostRequest calldata request) internal {
        if (!request.source.equals(IIsmpHost(_params.host).hyperbridge())) revert UnauthorizedAction();

        ChangeAssetAdmin memory asset = abi.decode(request.body[1:], (ChangeAssetAdmin));
        address erc6160Address = _erc6160s[asset.assetId];

        if (asset.newAdmin == address(0)) revert ZeroAddress();
        if (erc6160Address == address(0)) revert UnknownAsset();

        IERC6160Ext20(erc6160Address).changeAdmin(asset.newAdmin);

        emit AssetAdminChanged({asset: erc6160Address, newAdmin: asset.newAdmin});
    }

    /**
     * @dev registers a new instance of `TokenGateway` to permit receiving assets
     */
    function handleNewContractInstance(PostRequest calldata request) internal {
        if (!request.source.equals(IIsmpHost(_params.host).hyperbridge())) revert UnauthorizedAction();

        ContractInstance memory newInstance = abi.decode(request.body[1:], (ContractInstance));

        _instances[keccak256(newInstance.chain)] = newInstance.moduleId;

        emit NewContractInstance({chain: string(newInstance.chain), moduleId: newInstance.moduleId});
    }

    /**
     * @dev Creates a new entry for the provided asset in the mappings. If there's no existing
     * ERC6160 address provided, then a contract for the asset is created.
     */
    function createAssets(AssetMetadata[] memory assets) internal {
        uint256 length = assets.length;
        for (uint256 i = 0; i < length; ++i) {
            AssetMetadata memory asset = assets[i];
            bytes32 identifier = keccak256(bytes(asset.symbol));
            string memory symbol = asset.symbol;
            if (asset.erc20 != address(0)) {
                symbol = string.concat(symbol, ".h");
            }
            if (asset.erc6160 == address(0)) {
                ERC6160Ext20 erc6160Asset = new ERC6160Ext20{salt: identifier}(address(this), asset.name, symbol);
                asset.erc6160 = address(erc6160Asset);
                if (asset.beneficiary != address(0) && asset.initialSupply != 0) {
                    erc6160Asset.mint(asset.beneficiary, asset.initialSupply);
                }
            }
            _erc20s[identifier] = asset.erc20;
            _erc6160s[identifier] = asset.erc6160;

            emit AssetRegistered({
                erc20: asset.erc20,
                erc6160: asset.erc6160,
                name: asset.name,
                symbol: asset.symbol,
                assetId: identifier,
                beneficiary: asset.beneficiary,
                initialSupply: asset.initialSupply
            });
        }
    }

    /**
     * @dev Converts bytes to address.
     * @param _bytes bytes value to be converted
     * @return addr returns the address
     */
    function bytesToAddress(bytes memory _bytes) internal pure returns (address addr) {
        if (_bytes.length != 20) {
            revert InvalidAddressLength();
        }
        assembly {
            addr := mload(add(_bytes, 20))
        }
    }

    function addressToBytes32(address _address) internal pure returns (bytes32) {
        return bytes32(uint256(uint160(_address)));
    }

    function bytes32ToAddress(bytes32 _bytes) internal pure returns (address) {
        return address(uint160(uint256(_bytes)));
    }
}
