// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import {PostRequest, GetRequest, GetResponse, Message} from "@hyperbridge/core/libraries/Message.sol";
import {StorageValue} from "@polytope-labs/solidity-merkle-trees/src/trie/Node.sol";
import {Test} from "forge-std/Test.sol";

/// @dev Exposes encode/decode helpers for cross-language testing
contract AbiCodec {
    using Message for PostRequest;
    using Message for GetRequest;
    using Message for GetResponse;

    function encodePostRequest(PostRequest memory req) external pure returns (bytes memory) {
        return Message.encode(req);
    }

    function decodePostRequest(bytes memory data) external pure returns (PostRequest memory) {
        return abi.decode(data, (PostRequest));
    }

    function encodeGetRequest(GetRequest memory req) external pure returns (bytes memory) {
        return Message.encode(req);
    }

    function decodeGetRequest(bytes memory data) external pure returns (GetRequest memory) {
        return abi.decode(data, (GetRequest));
    }

    function encodeGetResponse(GetResponse memory res) external pure returns (bytes memory) {
        return Message.encode(res);
    }

    function decodeGetResponse(bytes memory data) external pure returns (GetResponse memory) {
        return abi.decode(data, (GetResponse));
    }

    function hashPostRequest(PostRequest memory req) external pure returns (bytes32) {
        return req.hash();
    }

    function hashGetRequest(GetRequest memory req) external pure returns (bytes32) {
        return req.hash();
    }

    function hashGetResponse(GetResponse memory res) external pure returns (bytes32) {
        return res.hash();
    }
}

contract AbiEncodeTest is Test {
    AbiCodec codec;

    function setUp() public {
        codec = new AbiCodec();
    }

    function testPostRequestRoundTrip() public view {
        PostRequest memory req = PostRequest({
            source: bytes("POLKADOT-2000"),
            dest: bytes("EVM-1"),
            nonce: 42,
            from: hex"deadbeef",
            to: hex"cafebabe",
            timeoutTimestamp: 1000,
            body: hex"1234"
        });

        bytes memory encoded = codec.encodePostRequest(req);
        PostRequest memory decoded = codec.decodePostRequest(encoded);

        assertEq(decoded.source, req.source);
        assertEq(decoded.dest, req.dest);
        assertEq(decoded.nonce, req.nonce);
        assertEq(decoded.from, req.from);
        assertEq(decoded.to, req.to);
        assertEq(decoded.timeoutTimestamp, req.timeoutTimestamp);
        assertEq(decoded.body, req.body);
        assertEq(codec.hashPostRequest(req), codec.hashPostRequest(decoded));
    }

    function testGetRequestRoundTrip() public view {
        bytes[] memory keys = new bytes[](2);
        keys[0] = hex"aabb";
        keys[1] = hex"ccdd";

        GetRequest memory req = GetRequest({
            source: bytes("POLKADOT-2000"),
            dest: bytes("EVM-1"),
            nonce: 7,
            from: hex"deadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
            timeoutTimestamp: 500,
            keys: keys,
            height: 100,
            context: hex"ff"
        });

        bytes memory encoded = codec.encodeGetRequest(req);
        GetRequest memory decoded = codec.decodeGetRequest(encoded);

        assertEq(decoded.source, req.source);
        assertEq(decoded.dest, req.dest);
        assertEq(decoded.nonce, req.nonce);
        assertEq(decoded.from, req.from);
        assertEq(decoded.timeoutTimestamp, req.timeoutTimestamp);
        assertEq(decoded.height, req.height);
        assertEq(decoded.context, req.context);
        assertEq(decoded.keys.length, req.keys.length);
        assertEq(codec.hashGetRequest(req), codec.hashGetRequest(decoded));
    }

    function testGetResponseRoundTrip() public view {
        bytes[] memory keys = new bytes[](1);
        keys[0] = hex"aabb";

        GetRequest memory req = GetRequest({
            source: bytes("POLKADOT-2000"),
            dest: bytes("EVM-1"),
            nonce: 1,
            from: hex"deadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
            timeoutTimestamp: 500,
            keys: keys,
            height: 50,
            context: hex""
        });

        StorageValue[] memory values = new StorageValue[](1);
        values[0] = StorageValue({key: hex"aabb", value: hex"1122"});

        GetResponse memory res = GetResponse({request: req, values: values});

        bytes memory encoded = codec.encodeGetResponse(res);
        GetResponse memory decoded = codec.decodeGetResponse(encoded);

        assertEq(decoded.request.nonce, req.nonce);
        assertEq(decoded.values.length, 1);
        assertEq(decoded.values[0].key, hex"aabb");
        assertEq(decoded.values[0].value, hex"1122");
        assertEq(codec.hashGetResponse(res), codec.hashGetResponse(decoded));
    }
}
