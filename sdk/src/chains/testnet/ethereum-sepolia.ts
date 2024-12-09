import { EthereumDatasourceKind, EthereumHandlerKind, EthereumProject } from '@subql/types-ethereum';

const project: EthereumProject = {
  specVersion: '1.0.0',
  version: '0.0.1',
  name: 'ethereum-sepolia',
  description: 'Ethereum Sepolia Chain Indexer',
  runner: {
    node: {
      name: '@subql/node-ethereum',
      version: '>=3.0.0',
    },
    query: {
      name: '@subql/query',
      version: '*',
    },
  },
  schema: {
    file: './schema.graphql',
  },
  network: {
    chainId: '11155111',
    endpoint: [
      'https://ethereum-sepolia-rpc.publicnode.com',
      'https://sepolia.drpc.org',
      'https://rpc2.sepolia.org',
    ],
  },
  dataSources: [
    {
      kind: EthereumDatasourceKind.Runtime,
      startBlock: 6429539,
      options: {
        abi: 'ethereumHost',
        address: '0xfCA0c05bEb9564AC154f55173881B4DD221A18A8',
      },
      assets: new Map([
        ['ethereumHost', { file: './abis/EthereumHost.abi.json' }],
        ['chainLinkAggregatorV3', { file: './abis/ChainLinkAggregatorV3.abi.json' }],
      ]),
      mapping: {
        file: './dist/index.js',
        handlers: [
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handlePostRequestEvent',
            filter: {
              topics: [
                'PostRequestEvent(string,string,address,bytes,uint256,uint256,bytes,uint256)',
              ],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handlePostResponseEvent',
            filter: {
              topics: [
                'PostResponseEvent(string,string,address,bytes,uint256,uint256,bytes,bytes,uint256,uint256)',
              ],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handlePostRequestHandledEvent',
            filter: {
              topics: ['PostRequestHandled(bytes32,address)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handlePostResponseHandledEvent',
            filter: {
              topics: ['PostResponseHandled(bytes32,address)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handlePostRequestTimeoutHandledEvent',
            filter: {
              topics: ['PostRequestTimeoutHandled(bytes32,string)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handlePostResponseTimeoutHandledEvent',
            filter: {
              topics: ['PostResponseTimeoutHandled(bytes32,string)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handleGetRequestHandledEvent',
            filter: {
              topics: ['GetRequestHandled(bytes32,address)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handleGetRequestTimeoutHandledEvent',
            filter: {
              topics: ['GetRequestTimeoutHandled(bytes32,string)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handleStateMachineUpdatedEvent',
            filter: {
              topics: ['StateMachineUpdated(string,uint256)'],
            },
          },
        ],
      },
    },
    {
      kind: EthereumDatasourceKind.Runtime,
      startBlock: 6429539,
      options: {
        abi: 'erc6160ext20',
        address: '0x83aF3a8a53bf0E379c17A8611AD697401A5970fD',
      },
      assets: new Map([
        ['erc6160ext20', { file: './abis/ERC6160Ext20.abi.json' }],
      ]),
      mapping: {
        file: './dist/index.js',
        handlers: [
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handleTransferEvent',
            filter: {
              topics: ['Transfer(address indexed from, address indexed to, uint256 amount)'],
            },
          },
        ],
      },
    },
    {
      kind: EthereumDatasourceKind.Runtime,
      startBlock: 6429539,
      options: {
        abi: 'handlerV1',
        address: '0x3a3A8BF5454a5ad98a3ca6b095cD6929aD441D7f',
      },
      assets: new Map([
        ['handlerV1', { file: './abis/HandlerV1.abi.json' }],
      ]),
      mapping: {
        file: './dist/index.js',
        handlers: [
          {
            handler: 'handlePostRequestTransactionHandler',
            kind: EthereumHandlerKind.Call,
            filter: {
              function: 'handlePostRequests(address,(((uint256,uint256),bytes32[],uint256),((bytes,bytes,uint64,bytes,bytes,uint64,bytes),uint256,uint256)[]))',
            },
          },
          {
            handler: 'handlePostResponseTransactionHandler',
            kind: EthereumHandlerKind.Call,
            filter: {
              function: 'handlePostResponses(address,(((uint256,uint256),bytes32[],uint256),(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64),uint256,uint256)[]))',
            },
          },
        ],
      },
    },
    {
      kind: EthereumDatasourceKind.Runtime,
      startBlock: 6429539,
      options: {
        abi: 'tokenGateway',
        address: '0x41867Dd678E3055649c04Fa10180ba90229cAd9F',
      },
      assets: new Map([
        ['tokenGateway', { file: './abis/TokenGateway.abi.json' }],
      ]),
      mapping: {
        file: './dist/index.js',
        handlers: [
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handleBidPlacedEvent',
            filter: {
              topics: ['BidPlaced(bytes32,uint256,bytes32,address)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handleBidRefundedEvent',
            filter: {
              topics: ['BidRefunded(bytes32,bytes32,address)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handleRequestFulfilledEvent',
            filter: {
              topics: ['RequestFulfilled(uint256,address,bytes32)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handleAssetReceivedEvent',
            filter: {
              topics: ['AssetReceived(uint256,bytes32,bytes32,address,bytes32)'],
            },
          },
          {
            kind: EthereumHandlerKind.Event,
            handler: 'handleAssetTeleportedEvent',
            filter: {
              topics: ['AssetTeleported(bytes32,string,uint256,bytes32,address,bytes32,bool)'],
            },
          },
        ],
      },
    },
  ],
  repository: 'https://github.com/polytope-labs/hyperbridge',
};

export default project;