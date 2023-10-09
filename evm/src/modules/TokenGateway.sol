// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;
//
//// import "../IISMPRouter.sol";
//import "../interfaces/IISMPModule.sol";
//import "openzeppelin/utils/introspection/IERC165.sol";
//import "multichain-token/interfaces/IERC6160Ext20.sol";
//import {IERC_ACL_CORE} from "multichain-token/interfaces/IERCAclCore.sol";
//import "../interfaces/IIsmp.sol";
//import "../interfaces/IIsmpHost.sol";
//
////
//// Add supports interface for custom bride token
////
//error LengthMismatch();
//error ZeroAddress();
//error TokenNotMultiChainNative();
//error BurnerRoleMissing();
//error MinterRoleMissing();
//error AuthFailed();
//error NotDispatcher();
//
//contract TokenGateway is IIsmpModule {
//    address admin;
//    address host;
//    bytes4 constant IERC6160Ext20ID = 0xbbb8b47e;
//
//    bytes32 constant MINTER_ROLE = keccak256("MINTER ROLE");
//    bytes32 constant BURNER_ROLE = keccak256("BURNER ROLE");
//
//    mapping(uint256 => address) public chains;
//    mapping(uint256 => address) public tokenIds;
//
//    // auth modifier
//    modifier auth() {
//        if (msg.sender != admin) {
//            revert AuthFailed();
//        }
//        _;
//    }
//
//    // restricts call to `dispatcher`
//    modifier onlyDispatcher() {
//        if (msg.sender != host) {
//            revert NotDispatcher();
//        }
//        _;
//    }
//
//    constructor(
//        address _host,
//        uint256[] memory _SMids,
//        address[] memory _SMaddresses,
//        uint256[] memory _Tids,
//        address[] memory _Taddresses
//    ) {
//        admin = msg.sender;
//        host = _host;
//        setStateMachineIds(_SMids, _SMaddresses);
//        setTokenIds(_Tids, _Taddresses);
//    }
//
//    // sets the addresses for a given StateMachineId
//    function setStateMachineIds(uint256[] memory _ids, address[] memory _addresses) public auth {
//        if (_ids.length != _addresses.length) revert LengthMismatch();
//        for (uint256 i = 0; i < _ids.length;) {
//            address _address = _addresses[i];
//            if (_address == address(0)) continue;
//            chains[_ids[i]] = _addresses[i];
//            unchecked {
//                ++i;
//            }
//        }
//    }
//
//    // sets the Id for a bridge compatible token
//    function setTokenIds(uint256[] memory _tokenIds, address[] memory _addresses) public auth {
//        if (_tokenIds.length != _addresses.length) revert LengthMismatch();
//        for (uint256 i = 0; i < _tokenIds.length;) {
//            address _tokenAddress = _addresses[i];
//            if (_tokenAddress == address(0)) revert ZeroAddress();
//            if (!IERC_ACL_CORE(_tokenAddress).hasRole(BURNER_ROLE, address(this))) revert BurnerRoleMissing();
//            if (!IERC_ACL_CORE(_tokenAddress).hasRole(MINTER_ROLE, address(this))) revert MinterRoleMissing();
//            if (!IERC165(_tokenAddress).supportsInterface(IERC6160Ext20ID)) revert TokenNotMultiChainNative();
//            tokenIds[_tokenIds[i]] = _tokenAddress;
//            unchecked {
//                ++i;
//            }
//        }
//    }
//
//    // The Gateway contract has to have the roles `MINTER` and `BURNER`.
//    function send(
//        bytes memory stateMachine,
//        uint256 tokenId,
//        uint256 amount,
//        address to,
//        bytes memory module,
//        uint64 timestamp,
//        uint64 gasLimit
//    ) public {
//        // USDC -> HyperUSDC(ERC6160)
//        address tokenAddress = tokenIds[tokenId];
//        // check permision at set token.
//        IERC6160Ext20(tokenAddress).burn(msg.sender, amount, "");
//        bytes memory data = abi.encodePacked(to, amount, tokenId);
//        bytes memory source = IIsmpHost(host).host();
//        DispatchPost memory postRequest = DispatchPost({
//            destChain: stateMachine,
//            from: source,
//            to: module,
//            body: data,
//            timeoutTimestamp: timestamp,
//            gaslimit: gasLimit
//        });
//        IIsmp(host).dispatch(postRequest);
//    }
//
//    function onAccept(PostRequest memory request) public onlyDispatcher {
//        (address to, uint256 amount, uint256 tokenId) = _decodePackedData(request.body);
//        address tokenAddress = tokenIds[tokenId];
//
//        IERC6160Ext20(tokenAddress).mint(to, amount, "");
//    }
//
//    function onPostResponse(PostResponse memory response) public view onlyDispatcher {
//        revert("Token gateway doesn't emit responses");
//    }
//
//    function onPostTimeout(PostRequest memory request) public onlyDispatcher {
//        (address to, uint256 amount, uint256 tokenId) = _decodePackedData(request.body);
//        address tokenAddress = tokenIds[tokenId];
//
//        if (tokenAddress == address(0)) revert ZeroAddress();
//
//        IERC6160Ext20(tokenAddress).mint(to, amount, "");
//    }
//
//    function onGetResponse(GetResponse memory response) public view onlyDispatcher {
//        revert("Not implemented");
//    }
//
//    function onGetTimeout(GetRequest memory request) public view onlyDispatcher {
//        revert("Not implemented");
//    }
//
//    function _decodePackedData(bytes memory data)
//        internal
//        pure
//        returns (address to_, uint256 amount_, uint256 tokenId_)
//    {
//        assembly {
//            to_ := div(mload(add(data, 32)), 0x1000000000000000000000000) // hex slicing to get first 20-bytes.
//            amount_ := mload(add(data, 52))
//            tokenId_ := mload(add(data, 84))
//        }
//    }
//}
