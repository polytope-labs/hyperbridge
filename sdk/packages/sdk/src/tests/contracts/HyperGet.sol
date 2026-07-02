// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

// --- ISMP types (mirror @hyperbridge/core: libraries/Message.sol, interfaces/{IDispatcher,IApp}.sol) ---
struct DispatchGet {
    bytes dest;
    uint64 height;
    bytes[] keys;
    uint64 timeout;
    uint256 fee;
    bytes context;
    address payer;
}

struct GetRequest {
    bytes source;
    bytes dest;
    uint64 nonce;
    bytes from;
    uint64 timeoutTimestamp;
    bytes[] keys;
    uint64 height;
    bytes context;
}

struct PostRequest {
    bytes source;
    bytes dest;
    uint64 nonce;
    bytes from;
    bytes to;
    uint64 timeoutTimestamp;
    bytes body;
}

struct StorageValue {
    bytes key;
    bytes value;
}

struct GetResponse {
    GetRequest request;
    StorageValue[] values;
}

struct IncomingGetResponse {
    GetResponse response;
    address relayer;
}

struct IncomingPostRequest {
    PostRequest request;
    address relayer;
}

struct PostRequestTimeout {
    PostRequest request;
    address relayer;
}

struct GetRequestTimeout {
    GetRequest request;
    address relayer;
}

interface IDispatcher {
    function dispatch(DispatchGet memory request) external payable returns (bytes32 commitment);
}

/**
 * @title HyperGet
 * @notice Minimal Hyperbridge test app for cross-chain GET requests.
 * @dev Reads an ERC20 `balanceOf(account)` on a destination chain and, when the response returns,
 * RLP-decodes the storage value to a uint256 and emits the account's balance. The queried account
 * is carried in the request `context` (echoed back in the response) rather than stored on-chain.
 */
contract HyperGet {
    address public immutable host;

    event BalanceRequested(bytes32 indexed commitment, bytes dest, address token, address account, uint64 height);
    event BalanceReceived(bytes32 indexed commitment, address indexed account, uint256 balance);

    error UnauthorizedCall();

    modifier onlyHost() {
        if (msg.sender != host) revert UnauthorizedCall();
        _;
    }

    constructor(address _host) {
        host = _host;
    }

    /**
     * @notice Dispatch a GET reading `token.balanceOf(account)` at `height` on `dest`.
     * @param dest destination state machine id as bytes (e.g. "EVM-80002")
     * @param token ERC20 contract address on the destination chain
     * @param account account whose balance to read
     * @param balanceSlot the ERC20 balance mapping's storage slot index
     * @param height destination block height to read at (must be finalized on Hyperbridge)
     */
    function readBalance(
        bytes calldata dest,
        address token,
        address account,
        uint256 balanceSlot,
        uint64 height
    ) external returns (bytes32 commitment) {
        bytes32 slot = keccak256(abi.encode(account, balanceSlot));
        bytes[] memory keys = new bytes[](1);
        keys[0] = abi.encodePacked(token, slot); // address(20) || slot(32)

        commitment = IDispatcher(host).dispatch(
            DispatchGet({
                dest: dest,
                height: height,
                keys: keys,
                timeout: 0,
                fee: 0,
                context: abi.encode(account), // carry the queried account; echoed back in the response
                payer: msg.sender
            })
        );
        emit BalanceRequested(commitment, dest, token, account, height);
    }

    /**
     * @notice Host callback delivering the GET response. Recovers the queried account from the
     * request `context`, RLP-decodes each returned storage value to a uint256 balance, and emits it.
     */
    function onGetResponse(IncomingGetResponse memory incoming) external onlyHost {
        GetRequest memory request = incoming.response.request;
        bytes32 commitment = keccak256(abi.encode(request));
        address account = abi.decode(request.context, (address));
        StorageValue[] memory values = incoming.response.values;
        for (uint256 i = 0; i < values.length; i++) {
            emit BalanceReceived(commitment, account, _rlpToUint(values[i].value));
        }
    }

    /**
     * @dev Minimal RLP decode of a single byte-string item to a uint256. EVM storage values are
     * returned RLP-encoded (leading zeros stripped); handles single-byte and short-string items.
     */
    function _rlpToUint(bytes memory item) internal pure returns (uint256 value) {
        if (item.length == 0) return 0;
        uint8 prefix = uint8(item[0]);
        if (prefix < 0x80) return prefix; // a single byte < 0x80 encodes itself
        require(prefix <= 0xb7, "HyperGet: unsupported RLP");
        uint256 len = prefix - 0x80;
        require(len <= 32 && item.length >= 1 + len, "HyperGet: bad RLP length");
        for (uint256 i = 0; i < len; i++) {
            value = (value << 8) | uint8(item[1 + i]);
        }
    }
}
