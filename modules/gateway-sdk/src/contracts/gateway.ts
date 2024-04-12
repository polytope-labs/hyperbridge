export default  [
    {
      "inputs": [
        {
          "internalType": "address",
          "name": "admin",
          "type": "address"
        }
      ],
      "stateMutability": "nonpayable",
      "type": "constructor"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": false,
          "internalType": "bytes",
          "name": "source",
          "type": "bytes"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "nonce",
          "type": "uint256"
        }
      ],
      "name": "AssetReceived",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": false,
          "internalType": "address",
          "name": "relayer",
          "type": "address"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "amount",
          "type": "uint256"
        }
      ],
      "name": "LiquidityProvided",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": false,
          "internalType": "bytes32",
          "name": "from",
          "type": "bytes32"
        },
        {
          "indexed": false,
          "internalType": "bytes32",
          "name": "to",
          "type": "bytes32"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "amount",
          "type": "uint256"
        },
        {
          "indexed": false,
          "internalType": "bool",
          "name": "redeem",
          "type": "bool"
        },
        {
          "indexed": false,
          "internalType": "bytes32",
          "name": "requestCommitment",
          "type": "bytes32"
        }
      ],
      "name": "Teleport",
      "type": "event"
    },
    {
      "inputs": [
        {
          "components": [
            {
              "internalType": "address",
              "name": "host",
              "type": "address"
            },
            {
              "internalType": "address",
              "name": "uniswapV2Router",
              "type": "address"
            },
            {
              "internalType": "bytes",
              "name": "hyperbridge",
              "type": "bytes"
            },
            {
              "internalType": "uint256",
              "name": "relayerFeePercentage",
              "type": "uint256"
            },
            {
              "internalType": "uint256",
              "name": "protocolFeePercentage",
              "type": "uint256"
            },
            {
              "components": [
                {
                  "internalType": "address",
                  "name": "erc20",
                  "type": "address"
                },
                {
                  "internalType": "address",
                  "name": "erc6160",
                  "type": "address"
                },
                {
                  "internalType": "bytes32",
                  "name": "identifier",
                  "type": "bytes32"
                }
              ],
              "internalType": "struct Asset[]",
              "name": "assets",
              "type": "tuple[]"
            },
            {
              "internalType": "address",
              "name": "callDispatcher",
              "type": "address"
            }
          ],
          "internalType": "struct InitParams",
          "name": "initialParams",
          "type": "tuple"
        }
      ],
      "name": "init",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "components": [
            {
              "internalType": "bytes",
              "name": "source",
              "type": "bytes"
            },
            {
              "internalType": "bytes",
              "name": "dest",
              "type": "bytes"
            },
            {
              "internalType": "uint64",
              "name": "nonce",
              "type": "uint64"
            },
            {
              "internalType": "bytes",
              "name": "from",
              "type": "bytes"
            },
            {
              "internalType": "bytes",
              "name": "to",
              "type": "bytes"
            },
            {
              "internalType": "uint64",
              "name": "timeoutTimestamp",
              "type": "uint64"
            },
            {
              "internalType": "bytes",
              "name": "body",
              "type": "bytes"
            }
          ],
          "internalType": "struct PostRequest",
          "name": "request",
          "type": "tuple"
        }
      ],
      "name": "onAccept",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "components": [
            {
              "components": [
                {
                  "internalType": "bytes",
                  "name": "source",
                  "type": "bytes"
                },
                {
                  "internalType": "bytes",
                  "name": "dest",
                  "type": "bytes"
                },
                {
                  "internalType": "uint64",
                  "name": "nonce",
                  "type": "uint64"
                },
                {
                  "internalType": "bytes",
                  "name": "from",
                  "type": "bytes"
                },
                {
                  "internalType": "uint64",
                  "name": "timeoutTimestamp",
                  "type": "uint64"
                },
                {
                  "internalType": "bytes[]",
                  "name": "keys",
                  "type": "bytes[]"
                },
                {
                  "internalType": "uint64",
                  "name": "height",
                  "type": "uint64"
                }
              ],
              "internalType": "struct GetRequest",
              "name": "request",
              "type": "tuple"
            },
            {
              "components": [
                {
                  "internalType": "bytes",
                  "name": "key",
                  "type": "bytes"
                },
                {
                  "internalType": "bytes",
                  "name": "value",
                  "type": "bytes"
                }
              ],
              "internalType": "struct StorageValue[]",
              "name": "values",
              "type": "tuple[]"
            }
          ],
          "internalType": "struct GetResponse",
          "name": "",
          "type": "tuple"
        }
      ],
      "name": "onGetResponse",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "components": [
            {
              "internalType": "bytes",
              "name": "source",
              "type": "bytes"
            },
            {
              "internalType": "bytes",
              "name": "dest",
              "type": "bytes"
            },
            {
              "internalType": "uint64",
              "name": "nonce",
              "type": "uint64"
            },
            {
              "internalType": "bytes",
              "name": "from",
              "type": "bytes"
            },
            {
              "internalType": "uint64",
              "name": "timeoutTimestamp",
              "type": "uint64"
            },
            {
              "internalType": "bytes[]",
              "name": "keys",
              "type": "bytes[]"
            },
            {
              "internalType": "uint64",
              "name": "height",
              "type": "uint64"
            }
          ],
          "internalType": "struct GetRequest",
          "name": "",
          "type": "tuple"
        }
      ],
      "name": "onGetTimeout",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "components": [
            {
              "internalType": "bytes",
              "name": "source",
              "type": "bytes"
            },
            {
              "internalType": "bytes",
              "name": "dest",
              "type": "bytes"
            },
            {
              "internalType": "uint64",
              "name": "nonce",
              "type": "uint64"
            },
            {
              "internalType": "bytes",
              "name": "from",
              "type": "bytes"
            },
            {
              "internalType": "bytes",
              "name": "to",
              "type": "bytes"
            },
            {
              "internalType": "uint64",
              "name": "timeoutTimestamp",
              "type": "uint64"
            },
            {
              "internalType": "bytes",
              "name": "body",
              "type": "bytes"
            }
          ],
          "internalType": "struct PostRequest",
          "name": "request",
          "type": "tuple"
        }
      ],
      "name": "onPostRequestTimeout",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "components": [
            {
              "components": [
                {
                  "internalType": "bytes",
                  "name": "source",
                  "type": "bytes"
                },
                {
                  "internalType": "bytes",
                  "name": "dest",
                  "type": "bytes"
                },
                {
                  "internalType": "uint64",
                  "name": "nonce",
                  "type": "uint64"
                },
                {
                  "internalType": "bytes",
                  "name": "from",
                  "type": "bytes"
                },
                {
                  "internalType": "bytes",
                  "name": "to",
                  "type": "bytes"
                },
                {
                  "internalType": "uint64",
                  "name": "timeoutTimestamp",
                  "type": "uint64"
                },
                {
                  "internalType": "bytes",
                  "name": "body",
                  "type": "bytes"
                }
              ],
              "internalType": "struct PostRequest",
              "name": "request",
              "type": "tuple"
            },
            {
              "internalType": "bytes",
              "name": "response",
              "type": "bytes"
            },
            {
              "internalType": "uint64",
              "name": "timeoutTimestamp",
              "type": "uint64"
            }
          ],
          "internalType": "struct PostResponse",
          "name": "",
          "type": "tuple"
        }
      ],
      "name": "onPostResponse",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "components": [
            {
              "components": [
                {
                  "internalType": "bytes",
                  "name": "source",
                  "type": "bytes"
                },
                {
                  "internalType": "bytes",
                  "name": "dest",
                  "type": "bytes"
                },
                {
                  "internalType": "uint64",
                  "name": "nonce",
                  "type": "uint64"
                },
                {
                  "internalType": "bytes",
                  "name": "from",
                  "type": "bytes"
                },
                {
                  "internalType": "bytes",
                  "name": "to",
                  "type": "bytes"
                },
                {
                  "internalType": "uint64",
                  "name": "timeoutTimestamp",
                  "type": "uint64"
                },
                {
                  "internalType": "bytes",
                  "name": "body",
                  "type": "bytes"
                }
              ],
              "internalType": "struct PostRequest",
              "name": "request",
              "type": "tuple"
            },
            {
              "internalType": "bytes",
              "name": "response",
              "type": "bytes"
            },
            {
              "internalType": "uint64",
              "name": "timeoutTimestamp",
              "type": "uint64"
            }
          ],
          "internalType": "struct PostResponse",
          "name": "",
          "type": "tuple"
        }
      ],
      "name": "onPostResponseTimeout",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "components": [
            {
              "internalType": "uint256",
              "name": "amount",
              "type": "uint256"
            },
            {
              "internalType": "uint256",
              "name": "fee",
              "type": "uint256"
            },
            {
              "internalType": "bytes32",
              "name": "assetId",
              "type": "bytes32"
            },
            {
              "internalType": "bool",
              "name": "redeem",
              "type": "bool"
            },
            {
              "internalType": "bytes32",
              "name": "to",
              "type": "bytes32"
            },
            {
              "internalType": "address",
              "name": "feeToken",
              "type": "address"
            },
            {
              "internalType": "bytes",
              "name": "dest",
              "type": "bytes"
            },
            {
              "internalType": "uint64",
              "name": "timeout",
              "type": "uint64"
            },
            {
              "internalType": "bytes",
              "name": "data",
              "type": "bytes"
            },
            {
              "internalType": "uint256",
              "name": "amountInMax",
              "type": "uint256"
            }
          ],
          "internalType": "struct TeleportParams",
          "name": "params",
          "type": "tuple"
        }
      ],
      "name": "teleport",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    }
  ]